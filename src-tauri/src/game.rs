use crate::{
    model::{Game, Prerequisite},
    safety::check_tree,
};
use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn inspect(path: &Path, launcher: &str) -> Result<Game> {
    check_tree(path)?;
    let root = fs::canonicalize(path)?;
    for marker in ["GTA5.exe", "GTA5_Enhanced.exe", "update/update.rpf", "x64"] {
        check_tree(&root.join(marker))?;
    }
    let enhanced = root.join("GTA5_Enhanced.exe").is_file();
    let legacy = root.join("GTA5.exe").is_file();
    if !legacy && !enhanced {
        bail!(
            "Select the game folder containing GTA5.exe (Legacy) or GTA5_Enhanced.exe (Enhanced)."
        );
    }
    if !root.join("update/update.rpf").is_file() || !root.join("x64").is_dir() {
        bail!("This is not a complete GTA V folder: update/update.rpf and x64 are required.");
    }
    let edition = if enhanced && legacy {
        "Ambiguous"
    } else if enhanced {
        "Enhanced"
    } else {
        "Legacy"
    };
    let specs = [
        (
            "shv",
            "Script Hook V",
            vec!["ScriptHookV.dll"],
            "https://www.dev-c.com/gtav/scripthookv/",
        ),
        (
            "asi",
            "ASI Loader",
            vec!["dinput8.dll"],
            "https://www.dev-c.com/gtav/scripthookv/",
        ),
        (
            "shvdn",
            "ScriptHookVDotNet",
            vec![
                "ScriptHookVDotNet.asi",
                "ScriptHookVDotNet2.dll",
                "ScriptHookVDotNet3.dll",
            ],
            "https://github.com/scripthookvdotnet/scripthookvdotnet/releases",
        ),
        (
            "openiv",
            "OpenIV ASI",
            vec!["OpenIV.asi"],
            "https://openiv.com/",
        ),
        (
            "rph",
            "RAGE Plugin Hook",
            vec!["RAGEPluginHook.exe"],
            "https://www.lcpdfr.com/lspdfr/legacy/install/",
        ),
        (
            "lspdfr",
            "LSPDFR",
            vec!["plugins/LSPD First Response.dll"],
            "https://www.lcpdfr.com/lspdfr/legacy/install/",
        ),
    ];
    let prerequisites = specs
        .into_iter()
        .map(|(id, name, markers, url)| {
            let missing: Vec<_> = markers
                .iter()
                .filter(|f| check_tree(&root.join(f)).is_err() || !root.join(f).is_file())
                .copied()
                .collect();
            Prerequisite {
                id: id.into(),
                name: name.into(),
                present: missing.is_empty(),
                detail: if missing.is_empty() {
                    "Files detected; version compatibility is unverified.".into()
                } else {
                    format!("Not detected: {}", missing.join(", "))
                },
                url: url.into(),
            }
        })
        .collect();
    let mut warnings = vec!["Prerequisite file presence does not establish build compatibility. Check each publisher's supported game builds before installing.".into()];
    if edition != "Legacy" {
        warnings.push(format!(
            "{edition} edition is not supported for automated installation in Phase 1."
        ));
    }
    let version = file_version(&root.join(if enhanced {
        "GTA5_Enhanced.exe"
    } else {
        "GTA5.exe"
    }));
    if version.is_none() {
        warnings
            .push("Game build could not be read reliably. Verify compatibility manually.".into());
    }
    Ok(Game {
        path: root.to_string_lossy().into(),
        edition: edition.into(),
        launcher: launcher.into(),
        version,
        prerequisites,
        warnings,
    })
}

fn file_version(path: &Path) -> Option<String> {
    if fs::metadata(path).ok()?.len() < 1024 {
        return None;
    }
    #[cfg(windows)]
    {
        use std::{os::windows::process::CommandExt, process::Command};
        let out = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-Item -LiteralPath $env:ZOXYRN_VERSION_TARGET).VersionInfo.FileVersion",
            ])
            .env("ZOXYRN_VERSION_TARGET", path)
            .creation_flags(0x08000000)
            .output()
            .ok()?;
        if out.status.success() {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if regex::Regex::new(r"^\d+\.\d+\.\d+\.\d+$")
                .ok()?
                .is_match(&v)
            {
                return Some(v);
            }
        }
    }
    let _ = path;
    None
}

pub fn game_running() -> bool {
    #[cfg(all(windows, not(test)))]
    {
        use std::{os::windows::process::CommandExt, process::Command};
        if let Ok(out) = Command::new("tasklist.exe")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(0x08000000)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return text.lines().any(|line| {
                [
                    "\"gta5.exe\"",
                    "\"gta5_enhanced.exe\"",
                    "\"playgtav.exe\"",
                    "\"ragepluginhook.exe\"",
                ]
                .iter()
                .any(|name| line.starts_with(name))
            });
        }
    }
    false
}

pub fn steam_candidates(root: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![root.to_path_buf()];
    if let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
        let re = regex::Regex::new(r#""path"\s*"([^"]+)""#).unwrap();
        for cap in re.captures_iter(&text) {
            libraries.push(PathBuf::from(cap[1].replace("\\\\", "\\")));
        }
    }
    let mut paths = Vec::new();
    let re = regex::Regex::new(r#""installdir"\s*"([^"]+)""#).unwrap();
    for library in libraries {
        for app in ["271590", "3240220"] {
            if let Ok(text) =
                fs::read_to_string(library.join(format!("steamapps/appmanifest_{app}.acf")))
            {
                if let Some(cap) = re.captures(&text) {
                    if !cap[1].contains(['/', '\\', ':']) && cap[1] != *".." {
                        paths.push(library.join("steamapps/common").join(&cap[1]));
                    }
                }
            }
        }
    }
    paths
}

pub fn detect() -> Vec<Game> {
    let mut candidates: Vec<(PathBuf, String)> = Vec::new();
    #[cfg(windows)]
    {
        use winreg::{enums::*, RegKey};
        let user = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = user.open_subkey("Software\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("SteamPath") {
                for p in steam_candidates(Path::new(&path)) {
                    candidates.push((p, "Steam".into()));
                }
            }
        }
        for base in ["C:/Program Files (x86)/Steam", "C:/Program Files/Steam"] {
            for p in steam_candidates(Path::new(base)) {
                candidates.push((p, "Steam".into()));
            }
        }
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            for sub in [
                "SOFTWARE\\Rockstar Games\\Grand Theft Auto V",
                "SOFTWARE\\WOW6432Node\\Rockstar Games\\Grand Theft Auto V",
            ] {
                if let Ok(key) = RegKey::predef(hive).open_subkey(sub) {
                    for val in ["InstallFolder", "InstallLocation"] {
                        if let Ok(path) = key.get_value::<String, _>(val) {
                            candidates.push((path.into(), "Rockstar Games Launcher".into()));
                        }
                    }
                }
            }
        }
    }
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        if let Ok(items) =
            fs::read_dir(PathBuf::from(program_data).join("Epic/EpicGamesLauncher/Data/Manifests"))
        {
            for item in items.flatten() {
                if item.path().extension().is_some_and(|e| e == "item") {
                    if let Ok(bytes) = fs::read(item.path()) {
                        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            let name = value["DisplayName"].as_str().unwrap_or("").to_lowercase();
                            if name.contains("grand theft auto v") {
                                if let Some(p) = value["InstallLocation"].as_str() {
                                    candidates.push((p.into(), "Epic Games".into()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut result = Vec::new();
    for (path, launcher) in candidates {
        if let Ok(game) = inspect(&path, &launcher) {
            if !result
                .iter()
                .any(|g: &Game| g.path.eq_ignore_ascii_case(&game.path))
            {
                result.push(game);
            }
        }
    }
    result
}
