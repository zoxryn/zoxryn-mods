//! Explicit fixture authoring utility. Refuses to write to an existing directory.
use anyhow::{bail, Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
use zip::{write::SimpleFileOptions, ZipWriter};
use zoxryn_core::Engine;
fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("Pass a new disposable workspace directory")?,
    );
    if root.exists() {
        bail!("Refusing to modify an existing directory");
    }
    let game = root.join("Mock GTA V Legacy");
    fs::create_dir_all(game.join("update"))?;
    fs::create_dir_all(game.join("x64"))?;
    fs::create_dir_all(root.join("downloads"))?;
    fs::write(
        game.join("GTA5.exe"),
        b"SYNTHETIC TEST FIXTURE - NOT EXECUTABLE",
    )?;
    fs::write(game.join("update/update.rpf"), b"SYNTHETIC ORIGINAL RPF")?;
    for p in [
        "ScriptHookV.dll",
        "dinput8.dll",
        "ScriptHookVDotNet.asi",
        "ScriptHookVDotNet2.dll",
        "ScriptHookVDotNet3.dll",
    ] {
        fs::write(game.join(p), b"SYNTHETIC PREREQUISITE - NOT EXECUTABLE")?;
    }
    let mut engine = Engine::open(&root.join("data"))?;
    engine.set_setting("show_tips", "false")?;
    // Unknown future settings are seeded directly in this new synthetic database
    // to prove the updater preserves them without adding unsupported UI features.
    let fixture_db = rusqlite::Connection::open(root.join("data/zoxryn.sqlite3"))?;
    fixture_db.execute(
        "INSERT INTO settings VALUES (?,?)",
        [
            "collection.Weekend",
            r#"{"label":"Weekend","mods":["Harbor Lights","Weekend Drives"]}"#,
        ],
    )?;
    fixture_db.execute(
        "INSERT INTO settings VALUES (?,?)",
        [
            "custom_categories",
            r#"["Synthetic favorites","Weekend collection"]"#,
        ],
    )?;
    drop(fixture_db);
    engine.select_game(&game, "Synthetic test fixture")?;
    for (i, (name, category, target, contents)) in [
        (
            "Harbor Lights",
            "Graphics",
            "HarborLights.asi",
            "SYNTHETIC ASI - NOT EXECUTABLE",
        ),
        (
            "Los Santos Stories",
            "Scripts",
            "scripts/Stories.cs",
            "// SYNTHETIC SCRIPT - NO CODE",
        ),
        (
            "Weekend Drives",
            "Gameplay",
            "WeekendDrives.asi",
            "SYNTHETIC ASI - NOT EXECUTABLE",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let path = root.join("downloads").join(format!("{name}.zip"));
        let mut zip = ZipWriter::new(File::create(&path)?);
        zip.start_file(target, SimpleFileOptions::default())?;
        zip.write_all(contents.as_bytes())?;
        zip.start_file("zoxryn.json", SimpleFileOptions::default())?;
        zip.write_all(serde_json::json!({"name":format!("{name} · synthetic"),"author":"Zoxryn test fixture","version":"0.0.0-test","category":category,"edition":"Legacy","source":"Disposable synthetic fixture"}).to_string().as_bytes())?;
        zip.start_file("readme.txt", SimpleFileOptions::default())?;
        zip.write_all(b"This is a non-functional synthetic test archive. Never copy it to a real GTA V installation.")?;
        zip.finish()?;
        let m = engine.import(&path)?;
        if i < 2 {
            let p = engine.prepare(&m.id, "install")?;
            engine.apply(&p.token, true, true)?;
        }
        if i == 1 {
            let p = engine.prepare(&m.id, "disable")?;
            engine.apply(&p.token, true, true)?;
        }
    }
    println!(
        "Synthetic workspace: {}\nLaunch: zoxryn-mods.exe --data-dir \"{}\"",
        root.display(),
        root.join("data").display()
    );
    Ok(())
}
