use crate::{model::*, rpf, safety::*};
use anyhow::{bail, Result};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    time::Instant,
};
pub const MAX_FILES: u64 = 50_000;
pub const MAX_ITEMS: usize = 20_000;
pub const MAX_READ: u64 = 32 * 1024 * 1024;
const DIRS: &[&str] = &[
    "scripts",
    "plugins",
    "mods",
    "lspdfr",
    "ragepluginhook",
    "menyoostuff",
    "maps",
    "mapeditor",
    "addonpeds",
    "lml",
    "reshade-shaders",
    "enbseries",
];

pub fn stable_id(game: &str, location: &str) -> String {
    hash_bytes(format!("{}\n{}", game.to_lowercase(), location.to_lowercase()).as_bytes())
}
pub fn scan(game: &Game, owners: &HashMap<String, String>) -> Result<ScanInventory> {
    if game.edition != "Legacy" {
        bail!("Installed-mod scanning is supported for GTA V Legacy only");
    }
    let root = Path::new(&game.path);
    check_tree(root)?;
    let started = Instant::now();
    let mut result = ScanInventory {
        game_path: game.path.clone(),
        scanned_at: now(),
        elapsed_ms: 0,
        visited_files: 0,
        visited_entries: 0,
        bytes_read: 0,
        hashed_bytes: 0,
        complete: true,
        stale: false,
        skipped: 0,
        warnings: vec![],
        items: vec![],
        managed: vec![],
    };
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    walk(root, "", 0, &mut files, &mut dirs, &mut result)?;
    // Prefix lookup makes bounded package metadata inspection O(log n + children),
    // rather than walking the entire inventory for every manifest.
    let mut package_paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    package_paths.sort_unstable();
    let all: HashSet<_> = files.iter().map(|f| f.path.to_lowercase()).collect();
    if all.len() != files.len() {
        result.complete = false;
        warn(
            &mut result,
            "Case-colliding filesystem paths were found; ownership cannot be established.",
        );
    }
    let directory_names: HashSet<_> = dirs.iter().map(|d| d.to_lowercase()).collect();
    let mut groups: HashMap<String, Vec<ScannedFile>> = HashMap::new();
    for f in &files {
        let active = entrypoint(&f.path)
            .map(|v| v.0)
            .unwrap_or_else(|| f.path.clone());
        let parent = active.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        groups
            .entry(format!("{parent}/{}", stem(&active)).to_lowercase())
            .or_default()
            .push(f.clone());
    }
    let mut stems: HashMap<String, Vec<String>> = HashMap::new();
    for f in &files {
        if let Some((active, _, ext)) = entrypoint(&f.path) {
            if ["asi", "dll", "cs", "vb", "lua"].contains(&ext.as_str()) {
                stems.entry(stem(&active)).or_default().push(f.path.clone());
            }
        }
    }
    let mut budget = MAX_READ;
    // Directory records retain package boundaries; they are never treated as one identified mod.
    for directory in dirs {
        if result.items.len() >= MAX_ITEMS {
            result.complete = false;
            warn(
                &mut result,
                "Inventory result limit reached; some containers were not listed.",
            );
            break;
        }
        let lower = directory.to_lowercase();
        if lower == "mods"
            || lower.starts_with("mods/update/x64/dlcpacks/") && lower.split('/').count() == 5
        {
            let mut item = base(
                game,
                &directory,
                if lower == "mods" {
                    "OpenIV mods directory"
                } else {
                    "DLC package"
                },
                "OpenIV",
                "possible",
            );
            item.name = if lower == "mods" {
                "OpenIV mods folder".into()
            } else {
                format!(
                    "DLC pack: {}",
                    directory.rsplit('/').next().unwrap_or("Unknown")
                )
            };
            item.kind = if lower == "mods" {
                "OpenIV folder"
            } else {
                "DLC package"
            }
            .into();
            item.notes.push("Container only: it may be unused, an unchanged copy, or contain several unrelated modifications. Individual mod ownership is unverified.".into());
            item.dependencies = vec!["asi".into(), "openiv".into()];
            result.items.push(item);
        }
    }
    for file in &files {
        if result.items.len() >= MAX_ITEMS {
            result.complete = false;
            warn(
                &mut result,
                "Inventory result limit reached; some files were not listed.",
            );
            break;
        }
        let path = &file.path;
        let lower = path.to_lowercase();
        if owners.contains_key(&lower) {
            continue;
        }
        let is_root = !lower.contains('/');
        let is_openiv = lower.starts_with("mods/");
        let is_original_rpf = lower.ends_with(".rpf") && !is_openiv;
        let (active, disabled, ext) =
            entrypoint(path).unwrap_or_else(|| (path.clone(), false, String::new()));
        if is_root && vanilla(&lower) {
            continue;
        }
        let prerequisite = prerequisite(&active);
        let scripts = lower.starts_with("scripts/");
        let police = lower.starts_with("plugins/")
            || lower.starts_with("lspdfr/")
            || lower.starts_with("ragepluginhook/")
            || active.eq_ignore_ascii_case("ragepluginhook.exe");
        let recognized = (is_root && ext == "asi")
            || (scripts && ["dll", "cs", "vb", "lua"].contains(&ext.as_str()))
            || (police && ext == "dll")
            || prerequisite.is_some();
        let data_only = !recognized && !is_openiv && !is_original_rpf;
        let confidence = if recognized {
            "identified"
        } else if is_openiv {
            "possible"
        } else {
            "unknown"
        };
        let source = if is_openiv {
            "OpenIV"
        } else if recognized {
            "Manual"
        } else {
            "Unknown"
        };
        let mut item = base(
            game,
            path,
            if recognized {
                "Loadable component"
            } else if is_original_rpf {
                "Original RPF (unverified)"
            } else {
                "Unidentified file"
            },
            source,
            confidence,
        );
        item.name = Path::new(&active)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into();
        item.files.push(file.clone());
        item.category = if police {
            "Police"
        } else if scripts {
            "Scripts"
        } else {
            rpf::category_hint(path).unwrap_or("Unknown")
        }
        .into();
        if recognized {
            // Executable names alone do not create suit identities.
            item.notes.push("Installer provenance is not recorded externally. Manual means an unmanaged component found in a recognized loading location.".into());
            item.status = if disabled { "disabled" } else { "enabled" }.into();
            item.notes.push("Status is inferred from a recognized file location and disabled suffix, not from game execution. Filename is the component name; creator identity and full mod boundaries are unverified.".into());
        }
        if let Some((name, requirements)) = prerequisite {
            item.name = name.into();
            item.kind = "Shared prerequisite".into();
            item.dependencies = requirements.iter().map(|r| r.to_string()).collect();
            item.control_reason =
                "Shared prerequisite: other mods may require it. Adoption and toggles are blocked."
                    .into();
        } else if recognized {
            item.dependencies = if police {
                vec!["rph".into()]
            } else if scripts {
                vec!["asi".into(), "shv".into(), "shvdn".into()]
            } else {
                vec!["asi".into(), "shv".into()]
            };
            if lower.contains("lspdfr") {
                item.dependencies.push("lspdfr".into());
            }
            let parent = active.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
            let active_stem = Path::new(&active)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            let companions: Vec<_> = groups
                .get(&format!("{parent}/{active_stem}").to_lowercase())
                .into_iter()
                .flatten()
                .filter(|f| f.path != *path)
                .cloned()
                .collect();
            for companion in &companions {
                item.files.push(companion.clone());
            }
            let resource_folder = if parent.is_empty() {
                active_stem.clone()
            } else {
                format!("{parent}/{active_stem}")
            };
            let grouped =
                !companions.is_empty() || directory_names.contains(&resource_folder.to_lowercase());
            if grouped {
                item.conflicts.push("Matching companions or resources make complete ownership ambiguous; no file grouping is assumed.".into());
            }
            if stems.get(&stem(&active)).is_some_and(|list| list.len() > 1) {
                item.conflicts.push("Another executable/script has the same stem; it may be a shared component or an active/disabled duplicate.".into());
            }
            let standalone = (is_root && ext == "asi")
                || (parent.eq_ignore_ascii_case("scripts") && ["cs", "vb"].contains(&ext.as_str()));
            let active_exists = disabled && all.contains(&active.to_lowercase());
            if active_exists {
                item.conflicts
                    .push("An active counterpart also exists.".into());
            }
            item.adoptable =
                standalone && !disabled && !grouped && item.conflicts.is_empty() && result.complete;
            item.control_reason=if item.adoptable{"Standalone entrypoint only. Adoption requires an exact backup plan and your confirmation that this single file is independent and not shared."}else if disabled{"External disabled naming schemes are not adopted. Re-enable manually according to its original tool, then rescan."}else{"Only an independent root ASI or a single source script directly in scripts/ can be adopted. Binary script libraries, RAGE packages, resource groups and ambiguous ownership require manual management."}.into();
            if !item.conflicts.is_empty() {
                item.status = "needs_review".into();
            }
        }
        if scripts && ext == "lua" {
            item.confidence = "possible".into();
            item.status = "needs_review".into();
            item.adoptable = false;
            item.dependencies.clear();
            item.notes.push("Lua loading method is unknown; ScriptHookVDotNet is not assumed to load Lua files.".into());
        }
        if is_root
            && ["dxgi.dll", "d3d11.dll", "d3d9.dll", "version.dll"]
                .contains(&active.to_lowercase().as_str())
        {
            item.category = "Graphics".into();
            item.kind = "Possible graphics/loader component".into();
            item.confidence = "possible".into();
            item.status = "needs_review".into();
        }
        if ext == "rpf" {
            item.name = format!(
                "{} · RPF container",
                Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
            item.kind = if is_openiv {
                "OpenIV RPF container"
            } else {
                "Original RPF (unverified)"
            }
            .into();
            item.status = if is_openiv { "needs_review" } else { "unknown" }.into();
            item.notes.push(if is_openiv{"OpenIV copy detected; it may contain multiple mods or unchanged game content. No payload comparison with the original was performed."}else{"Original game archive inventoried by location only. Without a trusted baseline, modification cannot be established."}.into());
            if is_openiv {
                let inspected = (|| -> Result<RpfSummary> {
                    let p = target(root, path)?;
                    let _guard = guard_existing_tree(p.parent().unwrap())?;
                    let mut f = open_read_no_follow(&p)?;
                    Ok(rpf::inspect(&mut f, file.size, &mut budget))
                })();
                match inspected {
                    Ok(summary) => {
                        item.rpf = Some(summary);
                    }
                    Err(e) => {
                        item.notes.push(format!("RPF was not read: {e}"));
                    }
                }
            }
            item.control_reason="A container may contain multiple unrelated mods. Safe individual isolation is unavailable; controls are disabled. Original RPFs are never modified.".into();
            item.adoptable = false;
        } else if data_only {
            item.notes.push("File inventory only; it may be configuration, shared resources, or an ordinary file rather than a mod.".into());
        }
        if lower.ends_with("/peds.meta") && (is_openiv || lower.starts_with("addonpeds/")) {
            let metadata = (|| -> Result<Vec<String>> {
                use std::io::Read;
                if file.size > 256 * 1024 || file.size + 1 > budget {
                    bail!("Ped metadata read budget unavailable");
                }
                let p = target(root, path)?;
                let _guard = guard_existing_tree(p.parent().unwrap())?;
                let f = open_read_no_follow(&p)?;
                let mut bytes = Vec::new();
                f.take(file.size + 1).read_to_end(&mut bytes)?;
                budget = budget.saturating_sub(bytes.len() as u64);
                if bytes.len() as u64 != file.size {
                    bail!("Ped metadata changed while scanning");
                }
                crate::organization::parse_ped_metadata(&bytes)
            })();
            match metadata {
                Ok(models) => {
                    item.character_models = models;
                    item.category = "Characters".into();
                    item.kind = "Shared ped metadata".into();
                    item.status = "needs_review".into();
                    item.notes.push("Loose CPedModelInfo metadata identifies model names; AddonPeds runtime registration and asset ownership are not inferred. Shared registry edits are unsupported.".into());
                }
                Err(e) => item.notes.push(format!("Ped metadata unavailable: {e}")),
            }
        }
        if lower.ends_with("/zoxryn.json") && file.size <= 256 * 1024 && file.size + 1 <= budget {
            let evidence = (|| -> Result<crate::grouping::PackageEvidence> {
                use std::io::Read;
                let path_on_disk = target(root, path)?;
                let _guard = guard_existing_tree(path_on_disk.parent().unwrap())?;
                let mut bytes = Vec::new();
                open_read_no_follow(&path_on_disk)?
                    .take(file.size + 1)
                    .read_to_end(&mut bytes)?;
                budget = budget.saturating_sub(bytes.len() as u64);
                if bytes.len() as u64 != file.size {
                    bail!("Package metadata changed during scan");
                }
                let mut manifest: Manifest = serde_json::from_slice(&bytes)?;
                let package_root = path.rsplit_once('/').unwrap().0;
                let prefix = format!("{package_root}/");
                let start = package_paths.partition_point(|path| *path < prefix.as_str());
                let paths: Vec<_> = package_paths[start..]
                    .iter()
                    .take_while(|path| path.starts_with(&prefix))
                    .filter_map(|path| path.strip_prefix(&prefix).map(str::to_string))
                    .collect();
                manifest.organization.validate(&paths)?;
                let name = crate::organization::clean_name(&manifest.name)?;
                if !manifest.category.is_empty() {
                    crate::organization::clean_name(&manifest.category)?;
                }
                if !manifest.version.is_empty() {
                    crate::organization::clean_name(&manifest.version)?;
                }
                if manifest.requirements.len() > 64
                    || manifest.requirements.iter().any(|id| {
                        id.is_empty()
                            || id.len() > 64
                            || !id
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
                    })
                {
                    bail!("Package dependency metadata exceeds its bounded identifier format");
                }
                for suit in &mut manifest.organization.suits {
                    if let Some(preview) = &mut suit.preview {
                        *preview = format!("{prefix}{preview}");
                    }
                    for file in &mut suit.files {
                        *file = format!("{prefix}{file}");
                    }
                }
                Ok(crate::grouping::PackageEvidence {
                    root: package_root.into(),
                    name,
                    category: manifest.category,
                    version: manifest.version,
                    preview: manifest
                        .organization
                        .preview
                        .map(|p| format!("{prefix}{p}")),
                    suits: manifest.organization.suits,
                    requirements: manifest.requirements,
                })
            })();
            match evidence {
                Ok(package) => {
                    item.dependencies.extend(package.requirements.clone());
                    item.dependencies.sort();
                    item.dependencies.dedup();
                    item.package_evidence = Some(package);
                    item.notes.push("Bounded explicit local package metadata; visual associations only, no ownership or safe toggling inferred.".into());
                }
                Err(error) => item
                    .notes
                    .push(format!("Package metadata not accepted: {error}")),
            }
        }
        for dependency in &item.dependencies {
            if !game
                .prerequisites
                .iter()
                .any(|p| &p.id == dependency && p.present)
            {
                item.conflicts
                    .push(format!("Dependency not detected: {dependency}"));
            }
        }
        if !item.conflicts.is_empty() && item.status == "enabled" {
            item.status = "needs_review".into();
        }
        if !item.conflicts.is_empty() {
            item.adoptable = false;
        }
        result.items.push(item);
    }
    if !result.complete {
        for item in &mut result.items {
            item.adoptable = false;
            item.control_reason =
                "Scan is incomplete; rescan successfully before adopting any file.".into();
        }
    }
    result
        .items
        .sort_by(|a, b| a.location.to_lowercase().cmp(&b.location.to_lowercase()));
    result.items.dedup_by(|a, b| a.id == b.id);
    result.bytes_read = MAX_READ - budget;
    result.elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(result)
}
fn base(game: &Game, path: &str, kind: &str, source: &str, confidence: &str) -> DiscoveredMod {
    DiscoveredMod {
        id: stable_id(&game.path, path),
        game_path: game.path.clone(),
        name: path.into(),
        category: "Unknown".into(),
        status: if confidence == "possible" {
            "needs_review"
        } else {
            "unknown"
        }
        .into(),
        source: source.into(),
        confidence: confidence.into(),
        kind: kind.into(),
        location: path.into(),
        files: vec![],
        dependencies: vec![],
        conflicts: vec![],
        notes: vec![],
        adoptable: false,
        control_reason: "Ownership or installation method is unknown. Manual review is required."
            .into(),
        managed_id: None,
        rpf: None,
        character_models: vec![],
        package_evidence: None,
    }
}
fn warn(result: &mut ScanInventory, message: &str) {
    if result.warnings.len() < 100 {
        result.warnings.push(message.into());
    }
}
fn walk(
    root: &Path,
    relative: &str,
    depth: usize,
    files: &mut Vec<ScannedFile>,
    dirs: &mut Vec<String>,
    out: &mut ScanInventory,
) -> Result<()> {
    if depth > 16 {
        out.complete = false;
        warn(out, &format!("Depth limit: {relative}"));
        return Ok(());
    }
    let folder = root.join(relative);
    let _guard = match guard_existing_tree(&folder) {
        Ok(g) => g,
        Err(e) => {
            out.complete = false;
            out.skipped += 1;
            warn(
                out,
                &format!("Skipped unsafe/inaccessible directory {relative}: {e}"),
            );
            return Ok(());
        }
    };
    let entries = match fs::read_dir(&folder) {
        Ok(e) => e,
        Err(e) => {
            out.complete = false;
            warn(out, &format!("Could not list {relative}: {e}"));
            return Ok(());
        }
    };
    let mut count = 0;
    for entry in entries {
        count += 1;
        if count > MAX_FILES || out.visited_entries >= MAX_FILES {
            out.complete = false;
            warn(out, "File traversal limit reached.");
            break;
        }
        out.visited_entries += 1;
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                out.complete = false;
                warn(out, &format!("Unreadable directory entry: {e}"));
                continue;
            }
        };
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            out.skipped += 1;
            out.complete = false;
            continue;
        };
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        if relative_name(&path).is_err() {
            out.skipped += 1;
            out.complete = false;
            warn(out, &format!("Skipped untrusted filename: {path}"));
            continue;
        }
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => {
                out.skipped += 1;
                out.complete = false;
                continue;
            }
        };
        if is_link(&metadata) {
            out.skipped += 1;
            out.complete = false;
            warn(out, &format!("Skipped symlink/reparse point: {path}"));
            continue;
        }
        if metadata.is_dir() {
            let lower = path.to_lowercase();
            dirs.push(path.clone());
            let recurse = depth > 0 || DIRS.contains(&name.to_lowercase().as_str());
            if recurse {
                walk(root, &path, depth + 1, files, dirs, out)?;
            } else if lower == "update" {
                let _update_guard = match guard_existing_tree(&entry.path()) {
                    Ok(g) => g,
                    Err(_) => {
                        out.complete = false;
                        out.skipped += 1;
                        continue;
                    }
                };
                // Original update.rpf is inventory-only; never read or hash its contents.
                let p = entry.path().join("update.rpf");
                if let Ok(m) = fs::symlink_metadata(&p) {
                    if m.is_file() && !is_link(&m) {
                        files.push(info("update/update.rpf", &m));
                        out.visited_files += 1;
                    }
                }
            }
        } else if metadata.is_file() {
            out.visited_files += 1;
            files.push(info(&path, &metadata));
        }
    }
    Ok(())
}
fn info(path: &str, metadata: &fs::Metadata) -> ScannedFile {
    ScannedFile {
        path: path.into(),
        size: metadata.len(),
        modified_ns: metadata
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_default(),
    }
}
pub fn entrypoint(path: &str) -> Option<(String, bool, String)> {
    let mut active = path.to_string();
    let mut disabled = false;
    for suffix in [".disabled", ".off", ".bak"] {
        if active.to_lowercase().ends_with(suffix) {
            active.truncate(active.len() - suffix.len());
            disabled = true;
            break;
        }
    }
    let ext = active.rsplit_once('.')?.1.to_lowercase();
    Some((active, disabled, ext))
}
fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}
fn vanilla(path: &str) -> bool {
    [
        "gta5.exe",
        "gta5_enhanced.exe",
        "playgtav.exe",
        "gtavlauncher.exe",
        "bink2w64.dll",
        "d3dcompiler_46.dll",
        "d3dcsx_46.dll",
        "gfSDK_shadowlib.win64.dll",
        "gfsdk_txaa.win64.dll",
        "gfsdk_txaa_alpharesolve.win64.dll",
        "version.txt",
        "commandline.txt",
        "installscript.vdf",
    ]
    .iter()
    .any(|n| n.eq_ignore_ascii_case(path))
}
fn prerequisite(path: &str) -> Option<(&'static str, &'static [&'static str])> {
    match path.to_lowercase().as_str() {
        "openiv.asi" => Some(("OpenIV ASI", &["asi"])),
        "dinput8.dll" | "dsound.dll" => Some(("ASI Loader (possible)", &[])),
        "scripthookv.dll" => Some(("Script Hook V", &[])),
        "scripthookvdotnet.asi" | "scripthookvdotnet2.dll" | "scripthookvdotnet3.dll" => {
            Some(("ScriptHookVDotNet component", &["shv", "asi"]))
        }
        "ragepluginhook.exe" => Some(("RAGE Plugin Hook", &[])),
        "plugins/lspd first response.dll" => Some(("LSPDFR", &["rph"])),
        _ => None,
    }
}
