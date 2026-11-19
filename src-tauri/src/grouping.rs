//! Read-only organization evidence. A visual boundary never grants file ownership.
use crate::{model::*, organization::SuitDeclaration};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEvidence {
    pub root: String,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub preview: Option<String>,
    pub suits: Vec<SuitDeclaration>,
    #[serde(default)]
    pub requirements: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct GroupHint {
    pub root: String,
    pub name: String,
    pub category: String,
    pub evidence: String,
}
pub fn role(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "asi" | "dll" | "cs" | "vb" | "lua" => "Script / plugin",
        "ini" | "cfg" | "xml" | "json" | "dat" | "meta" => "Configuration / metadata",
        "wav" | "mp3" | "ogg" | "awc" => "Audio",
        "ycd" | "ifp" => "Animation",
        "ydd" | "yft" | "ydr" => "Model resource",
        "ytd" | "png" | "jpg" | "jpeg" | "webp" => "Texture / artwork",
        "rpf" => "Archive container; may contain unrelated modifications",
        "ymap" | "ytyp" | "ybn" => "Map resource",
        "ymt" => "Unidentified resource metadata",
        _ => "Supporting file; purpose unverified",
    }
}
pub fn display_category(category: &str) -> String {
    match category {
        "Police" | "LSPDFR" => "Police & LSPDFR",
        "Superheroes" => "Superhero Mods",
        "Graphics" | "Maps" => "Maps & Graphics",
        "Scripts" | "Gameplay" => "Scripts & Plugins",
        "Characters" => "Peds & Skins",
        "Unknown" => "Other Mods",
        other => other,
    }
    .into()
}
fn generic(folder: &str) -> bool {
    [
        "scripts",
        "plugins",
        "lspdfr",
        "mods",
        "update",
        "x64",
        "dlcpacks",
        "data",
        "common",
        "sounds",
        "sound",
        "audio",
        "scanner",
        "car_model",
        "assets",
        "images",
        "huds",
        "suits",
        "skins",
        "characters",
        "models",
        "textures",
        "config",
        "configs",
        "configuration",
        "settings",
        "resources",
        "files",
        "animations",
        "support",
        "dependencies",
        "callouts",
        "bin",
        "lib",
        "libs",
        "libraries",
        "stream",
        "effects",
        "shaders",
    ]
    .contains(&folder)
}
fn known(folder: &str) -> Option<&'static str> {
    let f = folder.to_ascii_lowercase();
    if f.starts_with("thespiderman script files")
        || f == "spiderman"
        || f == "spider-man"
        || f.starts_with("spider-man script")
    {
        Some("Spider-Man Mod")
    } else if f
        .strip_prefix("hulk v")
        .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
        || f == "hulk"
    {
        Some("Hulk Mod")
    } else if f.starts_with("batman script files") || f == "batman" {
        Some("Batman Mod")
    } else if f.starts_with("ironmanv files") || f == "ironmanv" {
        Some("Iron Man Mod")
    } else if f.starts_with("superman v") && f.contains("script files") || f == "superman" {
        Some("Superman Mod")
    } else {
        None
    }
}
fn category(root: &str, name: &str, hint: &str) -> String {
    let all = format!("{root}/{name}").to_ascii_lowercase();
    if all.contains("lspdfr")
        || all.contains("police")
        || all.contains("callout")
        || all.split('/').any(|part| part == "eup")
        || ["Police", "LSPDFR", "Police & LSPDFR"].contains(&hint)
    {
        "Police & LSPDFR"
    } else if known(name).is_some()
        || [
            "Spider-Man Mod",
            "Hulk Mod",
            "Batman Mod",
            "Iron Man Mod",
            "Superman Mod",
        ]
        .contains(&name)
        || all.contains("superhero")
    {
        "Superhero Mods"
    } else if all.contains("vehicle") || all.contains("cars") || hint == "Vehicles" {
        "Vehicles"
    } else if all.contains("weapon") || hint == "Weapons" {
        "Weapons"
    } else if all.contains("graphic")
        || all.contains("reshade")
        || all.contains("enbseries")
        || ["Maps", "Graphics"].contains(&hint)
    {
        "Maps & Graphics"
    } else if all.contains("peds") || all.contains("uniform") || hint == "Characters" {
        "Peds & Skins"
    } else if root.starts_with("scripts/") || root.starts_with("plugins/") {
        "Scripts & Plugins"
    } else {
        "Other Mods"
    }
    .into()
}
pub fn police_section(root: &str, name: &str) -> &'static str {
    let text = format!("{root}/{name}").to_ascii_lowercase();
    if text.contains("ragepluginhook") || text.contains("rage plugin hook") {
        "RAGE Plugin Hook (shared dependency)"
    } else if text.contains("callout") {
        "Police Callout Packs"
    } else if text.contains("uniform") || text.contains("peds") || text.contains("eup") {
        "Police Uniforms & Peds"
    } else if text.contains("vehicle") || text.contains("dlcpacks") {
        "Police Vehicle Packs"
    } else if text.contains("audio") || text.contains("sounds") {
        "Police Audio and Other Add-ons"
    } else if name == "LSPDFR Core" {
        "LSPDFR Core"
    } else {
        "Police Plugins"
    }
}
pub fn dependency(path: &str) -> Option<&'static str> {
    match path.to_ascii_lowercase().as_str() {
        "scripthookv.dll" => Some("shv"),
        "dinput8.dll" => Some("asi"),
        "scripthookvdotnet.asi" | "scripthookvdotnet2.dll" | "scripthookvdotnet3.dll" => {
            Some("shvdn")
        }
        "openiv.asi" => Some("openiv"),
        "ragepluginhook.exe" => Some("rph"),
        _ => None,
    }
}
pub fn hints(inventory: &ScanInventory) -> BTreeMap<String, GroupHint> {
    let mut out = BTreeMap::new();
    let mut roots_with_loaders = HashSet::new();
    let paths: HashSet<_> = inventory
        .items
        .iter()
        .map(|i| i.location.to_ascii_lowercase())
        .collect();
    for item in &inventory.items {
        if item.kind == "Loadable component" {
            if let Some((root, _)) = item.location.rsplit_once('/') {
                let lower = item.location.to_ascii_lowercase();
                let (stem, ext) = lower.rsplit_once('.').unwrap_or((&lower, ""));
                // A supporting DLL alone does not establish an independent nested package.
                // Script entrypoints or an exact DLL sidecar are stronger boundaries.
                if ext != "dll"
                    || (stem.rsplit('/').next()
                        == root
                            .rsplit('/')
                            .next()
                            .map(|s| s.to_ascii_lowercase())
                            .as_deref()
                        && ["ini", "xml", "json"]
                            .iter()
                            .any(|ext| paths.contains(&format!("{stem}.{ext}"))))
                {
                    roots_with_loaders.insert(root.to_ascii_lowercase());
                }
            }
        }
        if let Some(package) = &item.package_evidence {
            let version = if package.version.is_empty() {
                String::new()
            } else {
                format!(" Declared package version: {}.", package.version)
            };
            let declared_category = if package.category.is_empty() {
                category(&package.root, &package.name, "")
            } else {
                display_category(&package.category)
            };
            out.insert(package.root.to_ascii_lowercase(),GroupHint{root:package.root.clone(),name:package.name.clone(),category:declared_category,evidence:format!("Explicit local zoxryn.json package declaration; author authenticity and runtime compatibility unverified.{version}")});
        }
    }
    for item in &inventory.items {
        if item.managed_id.is_some() {
            continue;
        }
        let path = item.location.replace('\\', "/");
        let lower = path.to_ascii_lowercase();
        if let Some(tool) = dependency(&path) {
            let name = match tool {
                "shv" => "Script Hook V",
                "asi" => "ASI Loader",
                "shvdn" => "ScriptHookVDotNet",
                "openiv" => "OpenIV.ASI",
                _ => "RAGE Plugin Hook (shared dependency)",
            };
            out.entry(format!("@dependency/{tool}")).or_insert(GroupHint{root:format!("@dependency/{tool}"),name:name.into(),category:"Tools & Dependencies".into(),evidence:"Detected shared prerequisite. Referenced by dependent collections, never exclusively adopted or duplicated".into()});
            continue;
        }
        if [
            "lspdfr.asi",
            "lspdfr/lspdfr.ini",
            "plugins/lspd first response.dll",
            "plugins/lspd first response.ini",
        ]
        .contains(&lower.as_str())
        {
            out.entry("@lspdfr-core".into()).or_insert(GroupHint{root:"@lspdfr-core".into(),name:"LSPDFR Core".into(),category:"Police & LSPDFR".into(),evidence:"Recognized core marker locations; file presence does not verify the framework build".into()});
            continue;
        }
        let segments: Vec<_> = path.split('/').collect();
        if segments.len() < 2 {
            if item.kind == "Loadable component"
                || item.kind == "Possible graphics/loader component"
            {
                out.entry(lower.clone()).or_insert(GroupHint{root:path.clone(),name:item.name.clone(),category:category(&lower,&item.name,&item.category),evidence:"Recognized standalone loading entrypoint; complete package scope remains unverified".into()});
            }
            continue;
        }
        if ![
            "scripts",
            "plugins",
            "lspdfr",
            "ragepluginhook",
            "lml",
            "menyoostuff",
            "maps",
            "mapeditor",
            "addonpeds",
            "reshade-shaders",
            "enbseries",
            "mods",
        ]
        .contains(&segments[0].to_ascii_lowercase().as_str())
        {
            continue;
        }
        if lower.starts_with("mods/") {
            continue;
        } // Existing explicit DLC/RPF boundaries handle OpenIV.
        let base_index = if segments[0].eq_ignore_ascii_case("plugins")
            && segments
                .get(1)
                .is_some_and(|s| s.eq_ignore_ascii_case("lspdfr"))
        {
            2
        } else {
            1
        };
        for i in base_index..segments.len() - 1 {
            let folder = segments[i];
            let root = segments[..=i].join("/");
            let root_lower = root.to_ascii_lowercase();
            if generic(&folder.to_ascii_lowercase()) {
                continue;
            }
            let first_meaningful = segments[base_index..i]
                .iter()
                .all(|s| generic(&s.to_ascii_lowercase()));
            if !first_meaningful
                && known(folder).is_none()
                && !roots_with_loaders.contains(&root_lower)
            {
                continue;
            }
            let name = known(folder).unwrap_or(folder).to_string();
            out.entry(root_lower).or_insert_with(||GroupHint{category:category(&root,&name,&item.category),root,name,evidence:"Inferred from a bounded descriptive parent directory. Virtual grouping only; installation scope/ownership unverified".into()});
        }
        // Separate each root-level police plugin/callout; do not combine the LSPDFR folder.
        if segments.len() == base_index + 1 && item.kind == "Loadable component" {
            out.entry(lower.clone()).or_insert(GroupHint{root:path.clone(),name:item.name.clone(),category:category(&lower,&item.name,&item.category),evidence:"Individual loading entrypoint. Neighboring packages are separate; companion relationships need review".into()});
        }
    }
    // Exact, known loader aliases may reference a single recognized sibling data
    // directory. Multiple versions stay separate and ambiguous; no fuzzy matching.
    let aliases = [
        ("thespiderman", "Spider-Man Mod"),
        ("spiderman", "Spider-Man Mod"),
        ("batman", "Batman Mod"),
        ("hulk", "Hulk Mod"),
        ("ironmanv", "Iron Man Mod"),
        ("superman", "Superman Mod"),
    ];
    let mut linked = Vec::new();
    for (key, hint) in &out {
        let Some((directory, file)) = key.rsplit_once('/') else {
            continue;
        };
        let Some((stem, ext)) = file.rsplit_once('.') else {
            continue;
        };
        if directory != "scripts" || !["dll", "asi", "cs", "vb"].contains(&ext) {
            continue;
        }
        if let Some((_, name)) = aliases.iter().find(|(alias, _)| *alias == stem) {
            let targets: Vec<_> = out
                .iter()
                .filter(|(root, group)| {
                    group.name == *name
                        && root.rsplit_once('/').is_some_and(|(parent, folder)| {
                            parent == "scripts" && known(folder) == Some(*name)
                        })
                        && ![".dll", ".asi", ".cs", ".vb"]
                            .iter()
                            .any(|ext| root.ends_with(ext))
                        && root.starts_with("scripts/")
                })
                .collect();
            if targets.len() == 1 && targets[0].0 != key {
                let mut alias = targets[0].1.clone();
                alias.evidence = format!("{} Exact known loader alias {} references this single sibling data directory; ownership remains unverified.", alias.evidence, hint.root);
                linked.push((key.clone(), alias));
            }
        }
    }
    for (key, hint) in linked {
        out.insert(key, hint);
    }
    let redundant: Vec<_> = out
        .keys()
        .filter(|key| {
            key.rsplit_once('.').is_some_and(|(stem, ext)| {
                ["dll", "asi", "cs", "vb"].contains(&ext) && out.contains_key(stem)
            })
        })
        .cloned()
        .collect();
    for key in redundant {
        out.remove(&key);
    }
    out
}
pub fn group_for<'a>(path: &str, hints: &'a BTreeMap<String, GroupHint>) -> Option<&'a GroupHint> {
    if let Some(id) = dependency(path) {
        return hints.get(&format!("@dependency/{id}"));
    }
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    if [
        "lspdfr.asi",
        "lspdfr/lspdfr.ini",
        "plugins/lspd first response.dll",
        "plugins/lspd first response.ini",
    ]
    .contains(&lower.as_str())
    {
        return hints.get("@lspdfr-core");
    }
    if let Some((stem, extension)) = lower.rsplit_once('.') {
        if ["dll", "asi", "cs", "vb", "ini", "cfg", "xml", "json", "dat"].contains(&extension) {
            if let Some(folder) = hints.get(stem) {
                return Some(folder);
            }
            if ["ini", "cfg", "xml", "json", "dat"].contains(&extension) {
                let matches: Vec<_> = ["dll", "asi", "cs", "vb"]
                    .iter()
                    .filter_map(|ext| hints.get(&format!("{stem}.{ext}")))
                    .collect();
                if matches.len() == 1 {
                    return Some(matches[0]);
                }
            }
        }
    }
    let mut current = lower;
    loop {
        if let Some(hint) = hints.get(&current) {
            return Some(hint);
        }
        if let Some((parent, _)) = current.rsplit_once('/') {
            current = parent.into();
        } else {
            return None;
        }
    }
}
