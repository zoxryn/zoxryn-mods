//! Explicit fixture authoring utility. Refuses to write to an existing directory.
use anyhow::{bail, Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
use zip::{write::SimpleFileOptions, ZipWriter};
use zoxryn_core::organization::MemberRef;
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
    if std::env::args().any(|a| a == "--neon") {
        fs::write(root.join("SYNTHETIC-UI-QA"), b"ZOXRYN SYNTHETIC UI QA ONLY")?;
        for (folder, name) in [
            ("TheSpiderman script files", "Spider-Man Mod"),
            ("HULK v3 Files", "Hulk Mod"),
            ("Batman script files", "Batman Mod"),
            ("IronmanV Files (v3 E.G.)", "Iron Man Mod"),
            ("Superman V2 script files", "Superman Mod"),
            ("PoliceWardrobe", "Police Uniform Pack"),
        ] {
            let base = game.join("scripts").join(folder);
            for path in [
                "Main.cs",
                "Sounds/basic melee 7.wav",
                "Sounds/basic melee 8.wav",
                "Suits/Classic.ini",
                "Suits/Classic_models_cfg.ini",
                "Models/Classic.ydd",
                "Textures/Classic.ytd",
            ] {
                let path = base.join(path);
                fs::create_dir_all(path.parent().unwrap())?;
                fs::write(path, b"SYNTHETIC NON-FUNCTIONAL QA ASSET")?;
            }
            let police = folder == "PoliceWardrobe";
            fs::write(
                base.join("zoxryn.json"),
                serde_json::to_vec(
                    &serde_json::json!({"name":name,"category":if police {"Police"} else {"Superhero Mods"},"requirements":["shvdn"],"organization":{"suits":[{"key":"classic","name":format!("{} · Classic",name.trim_end_matches(" Mod")),"category":if police {"Police Uniforms"} else {"Superhero Suits"},"model":"synthetic_outfit","files":["Suits/Classic.ini","Models/Classic.ydd","Textures/Classic.ytd"]}]}}),
                )?,
            )?;
        }
        for path in [
            "RAGEPluginHook.exe",
            "LSPDFR.asi",
            "OpenIV.asi",
            "plugins/LSPDFR/HarborCallouts.dll",
            "plugins/LSPDFR/HarborCallouts.ini",
            "plugins/LSPDFR/HarborCallouts/Audio/dispatch.wav",
            "plugins/LSPDFR/CountyCallouts.dll",
            "plugins/LSPDFR/CountyCallouts/Config/options.xml",
            "plugins/LSPDFR/BodyCamera.dll",
            "lspdfr/audio/scanner/CityVoicePack/dispatch.wav",
            "lml/PoliceVehiclePack/stream/police.ytd",
            "lml/WeaponPack/stream/w_pistol.ytd",
            "reshade-shaders/NeonPreset/Shaders/neon.fx",
            "scripts/VehicleTools/Config/options.ini",
            "scripts/UtilityPack/Main.cs",
            "mods/update/x64/dlcpacks/fixture/dlc.rpf",
            "mods/unknown_resource.ymt",
            "unidentified.fixture",
        ] {
            let path = game.join(path);
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path, b"SYNTHETIC NON-FUNCTIONAL QA ASSET")?;
        }
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
    let mods = engine.snapshot()?.mods;
    let collection = engine.save_collection(
        None,
        "Synthetic weekend collection",
        "Updater preservation fixture",
        "Custom",
        &mods
            .iter()
            .take(2)
            .map(|m| MemberRef {
                kind: "managed".into(),
                id: m.id.clone(),
            })
            .collect::<Vec<_>>(),
    )?;
    let artwork = root.join("downloads/synthetic-preview.png");
    // Plain color fixture artwork; never presented as real character artwork.
    image::RgbaImage::from_pixel(2, 2, image::Rgba([126, 62, 195, 255])).save(&artwork)?;
    engine.assign_preview("collection", &collection, Some(&artwork))?;
    println!(
        "Synthetic workspace: {}\nData (use the isolated UI QA product only): {}",
        root.display(),
        root.join("data").display()
    );
    Ok(())
}
