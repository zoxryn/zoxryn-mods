use crate::{
    model::{Manifest, ModFile, ModRecord},
    safety::*,
};
use anyhow::{bail, Context, Result};
use std::{collections::HashSet, fs::File, io::Read, path::Path};
use zip::ZipArchive;

const MANUAL: &str = "This layout requires manual installation. Read the author's instructions. For OIV or DLC/RPF changes, use OpenIV's mods-folder workflow; never edit the original RPF archives. Zoxryn has not installed any files. Manual changes are not adopted automatically. Use Installed Mods to inventory external files and review safely supported adoption.";

pub fn inspect(path: &Path) -> Result<ModRecord> {
    check_tree(path)?;
    if std::fs::metadata(path)?.len() > MAX_ARCHIVE {
        bail!("Archive exceeds the 1 GiB import limit");
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["zip", "7z", "rar", "oiv"].contains(&ext.as_str()) {
        bail!("Unsupported archive format. Import ZIP, or keep 7z/RAR/OIV for manual guidance. No installer or script will be executed.");
    }
    let mut record = ModRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        author: "Unknown".into(),
        version: "Unknown".into(),
        category: "Gameplay".into(),
        source: "Local archive".into(),
        imported_at: now(),
        installed_at: None,
        status: "imported".into(),
        archive_path: "".into(),
        archive_hash: hash_file(path)?,
        kind: "Manual".into(),
        edition: "Unspecified".into(),
        requirements: vec![],
        warnings: vec![],
        instructions: "".into(),
        files: vec![],
        supported_builds: vec![],
        game_path: None,
        adopted_from: None,
        origin: "Zoxryn".into(),
    };
    if ext == "rar" || ext == "7z" {
        record.instructions = format!("{ext} extraction is unavailable in this build. Use a trusted, updated archive tool to inspect it and extract to a separate folder. Review the author's instructions, then create a ZIP preserving the original layout and import that ZIP. Never run bundled installers automatically.");
        record.warnings.push(
            "Contents have not been inspected. Automated installation is unavailable.".into(),
        );
        return Ok(record);
    }
    let mut zip = ZipArchive::new(File::open(path)?)
        .context("Could not read ZIP/OIV; encrypted or damaged archives are not supported")?;
    if zip.len() > 10000 {
        bail!("Archive exceeds 10,000 entries");
    }
    zip.validate_header_consistency()
        .context("ZIP local headers do not match the inspected central directory")?;
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut total = 0u64;
    let mut readme = String::new();
    let mut manifest = None;
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .context("Encrypted or unsupported compression in ZIP")?;
        let name = relative_name(file.name()?.trim_end_matches('/'))?;
        if file.is_symlink()
            || file.unix_mode().is_some_and(|m| {
                (m & 0o170000) != 0 && (m & 0o170000) != 0o100000 && (m & 0o170000) != 0o040000
            })
        {
            bail!("Archive contains a symlink or special file: {name}");
        }
        if !seen.insert(name.to_lowercase()) {
            bail!("Duplicate or case-colliding archive path: {name}");
        }
        if file.is_dir() {
            continue;
        }
        total = total
            .checked_add(file.size())
            .context("Archive size overflow")?;
        if total > MAX_EXPANDED
            || file.size() > MAX_FILE
            || file.size() / file.compressed_size().max(1) > 1000
        {
            bail!("Archive expansion limit exceeded");
        }
        let mut bytes = Vec::new();
        (&mut file).take(MAX_FILE + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE || bytes.len() as u64 != file.size() {
            bail!("Archive entry size mismatch");
        }
        let lower = name.to_lowercase();
        if lower.split('/').next_back() == Some("zoxryn.json") {
            if manifest.is_some() || bytes.len() > 65536 {
                bail!("Multiple or oversized zoxryn.json manifests");
            }
            manifest = Some(
                serde_json::from_slice::<Manifest>(&bytes)
                    .context("Invalid zoxryn.json metadata")?,
            );
        }
        if (lower.contains("readme") || lower.contains("install"))
            && lower.ends_with(".txt")
            && readme.len() < 65536
        {
            readme.push_str(&String::from_utf8_lossy(&bytes[..bytes.len().min(16384)]));
            readme.push('\n');
        }
        entries.push((name, hash_bytes(&bytes), bytes.len() as u64));
    }
    let categories = [
        "Vehicles",
        "Police",
        "Superheroes",
        "Graphics",
        "Scripts",
        "Maps",
        "Characters",
        "Gameplay",
    ];
    if let Some(m) = manifest {
        if !m.name.trim().is_empty() {
            record.name = m.name;
        }
        if !m.author.is_empty() {
            record.author = m.author;
        }
        if !m.version.is_empty() {
            record.version = m.version;
        }
        if categories.contains(&m.category.as_str()) {
            record.category = m.category;
        }
        if !m.source.is_empty() {
            record.source = m.source;
        }
        if !m.edition.is_empty() {
            record.edition = m.edition;
        }
        record.requirements = m.requirements;
        record.supported_builds = m.supported_builds;
    }
    let mut prefix = String::new();
    if let Some((first, _, _)) = entries.first() {
        if let Some((top, _)) = first.split_once('/') {
            if !["scripts", "plugins", "mods", "lspdfr", "update", "x64"]
                .contains(&top.to_lowercase().as_str())
                && entries
                    .iter()
                    .all(|(n, _, _)| n.starts_with(&format!("{top}/")))
            {
                prefix = format!("{top}/");
            }
        }
    }
    let names: Vec<String> = entries
        .iter()
        .map(|(n, _, _)| n.strip_prefix(&prefix).unwrap_or(n).to_string())
        .collect();
    let asi_stems: HashSet<String> = names
        .iter()
        .filter(|n| !n.contains('/') && n.to_lowercase().ends_with(".asi"))
        .map(|n| n[..n.len() - 4].to_lowercase())
        .collect();
    let mut unsupported = Vec::new();
    let mut is_script = false;
    let mut police = false;
    let mut dlc = false;
    for ((original, hash, size), name) in entries.into_iter().zip(names) {
        let lower = name.to_lowercase();
        let ext = lower.rsplit('.').next().unwrap_or("");
        if lower == "zoxryn.json"
            || [
                "txt", "md", "pdf", "jpg", "jpeg", "png", "url", "html", "htm",
            ]
            .contains(&ext)
        {
            continue;
        }
        if lower.starts_with("plugins/")
            || lower.starts_with("lspdfr/")
            || lower.contains("ragepluginhook")
        {
            police = true;
        }
        if lower.ends_with(".rpf")
            || lower.contains("dlcpacks/")
            || lower.ends_with(".oiv")
            || lower == "assembly.xml"
        {
            dlc = true;
        }
        let root_asi = !lower.contains('/')
            && ext == "asi"
            && !["openiv.asi", "scripthookvdotnet.asi"].contains(&lower.as_str());
        let sidecar = !lower.contains('/')
            && ["ini", "json", "xml", "cfg"].contains(&ext)
            && asi_stems.contains(lower.rsplit_once('.').unwrap().0);
        let script = lower.starts_with("scripts/")
            && ["dll", "cs", "vb", "ini", "json", "xml", "cfg", "dat"].contains(&ext);
        if root_asi || sidecar || script {
            if script {
                is_script = true;
            }
            record.files.push(ModFile {
                archive_entry: original,
                target: name,
                hash,
                size,
            });
        } else {
            unsupported.push(name);
        }
    }
    if police {
        record.category = "Police".into();
        record.requirements.extend(["rph".into(), "lspdfr".into()]);
    }
    if dlc {
        record.requirements.push("openiv".into());
    }
    if !asi_stems.is_empty() {
        record.requirements.extend(["shv".into(), "asi".into()]);
    }
    if is_script {
        record
            .requirements
            .extend(["shv".into(), "asi".into(), "shvdn".into()]);
        if record.category == "Gameplay" {
            record.category = "Scripts".into();
        }
    }
    if record.edition == "Unspecified" && readme.to_lowercase().contains("enhanced only") {
        record.edition = "Enhanced".into();
    }
    record.requirements.sort();
    record.requirements.dedup();
    record.instructions = readme;
    if !unsupported.is_empty() || record.files.is_empty() || dlc || police || ext == "oiv" {
        record.kind = if ext == "oiv" || dlc {
            "OpenIV / DLC".into()
        } else if police {
            "RAGE / LSPDFR".into()
        } else {
            "Manual".into()
        };
        record.warnings.push(format!(
            "Unmapped files ({}): {}",
            unsupported.len(),
            unsupported
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
        record.instructions = format!("{MANUAL}\n\n{}", record.instructions);
        record.files.clear();
    } else {
        record.kind = if is_script {
            "Script".into()
        } else {
            "ASI".into()
        };
    }
    if record.edition == "Unspecified" {
        record.warnings.push("Edition support was not declared. Review the author's Legacy compatibility instructions.".into());
    }
    record.warnings.push("Archives are inspected for structure, not malware. Install only files and creators you trust.".into());
    Ok(record)
}

pub fn extract_selected(archive: &Path, file: &ModFile, destination: &Path) -> Result<()> {
    let mut zip = ZipArchive::new(File::open(archive)?)?;
    let mut source = zip.by_name(&file.archive_entry)?;
    let mut bytes = Vec::new();
    (&mut source).take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != file.size || hash_bytes(&bytes) != file.hash {
        bail!("Imported archive changed; import it again");
    }
    std::fs::write(destination, bytes)?;
    File::options().write(true).open(destination)?.sync_all()?;
    Ok(())
}
