use super::*;
use crate::organization::MemberRef;
use std::collections::BTreeMap;
fn put(f: &Fixture, path: &str, bytes: &[u8]) {
    let p = f.root.join(path);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}
#[test]
fn realistic_loader_siblings_support_libraries_and_police_packages_stay_virtual() {
    let f = Fixture::new();
    f.prereqs();
    let heroes = [
        (
            "TheSpiderman",
            "TheSpiderman script files",
            "Spider-Man Mod",
        ),
        ("Batman", "Batman script files", "Batman Mod"),
        ("Hulk", "HULK v3 Files", "Hulk Mod"),
        ("IronmanV", "IronmanV Files (v3 E.G.)", "Iron Man Mod"),
        ("Superman", "Superman V2 script files", "Superman Mod"),
    ];
    for (loader, folder, _) in heroes {
        put(&f, &format!("scripts/{loader}.dll"), b"synthetic loader");
        put(&f, &format!("scripts/{loader}.ini"), b"synthetic sidecar");
        for suffix in [
            "Sounds/basic melee 7.wav",
            "Suits/Classic.ini",
            "Config/options.xml",
            "Assets/AudioBridge/Support.dll",
            "Assets/AudioBridge/Support.ini",
            "Libraries/JsonHelper.dll",
            "Models/Classic.ydd",
            "Textures/Classic.ytd",
        ] {
            put(
                &f,
                &format!("scripts/{folder}/{suffix}"),
                b"synthetic support",
            );
        }
    }
    for path in [
        "plugins/LSPD First Response.dll",
        "plugins/LSPD First Response.ini",
        "lspdfr/lspdfr.ini",
        "RAGEPluginHook.exe",
        "plugins/LSPDFR/MetroCallouts.dll",
        "plugins/LSPDFR/MetroCallouts.ini",
        "plugins/LSPDFR/MetroCallouts/Audio/dispatch.wav",
        "plugins/LSPDFR/MetroCallouts/Assets/Codec/Support.dll",
        "plugins/LSPDFR/CountyCallouts.dll",
        "plugins/LSPDFR/CountyCallouts/Config/settings.xml",
        "lml/VehiclePack/stream/car.yft",
        "lml/WeaponPack/stream/w_pistol.ytd",
        "scripts/GeneralUtility/Config/settings.xml",
        "scripts/unidentified.ini",
    ] {
        put(&f, path, b"synthetic");
    }
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    for (loader, _, name) in heroes {
        let groups: Vec<_> = s.collections.iter().filter(|g| g.name == name).collect();
        assert_eq!(groups.len(), 1, "{name}");
        assert_eq!(groups[0].unique_files, 10, "{name}");
        assert!(groups[0]
            .members
            .iter()
            .flat_map(|m| &m.files)
            .any(|p| p == &format!("scripts/{loader}.dll")));
    }
    assert!(!s
        .collections
        .iter()
        .any(|g| ["AudioBridge", "Support", "JsonHelper", "Codec"].contains(&g.name.as_str())));
    let core = s
        .collections
        .iter()
        .find(|g| g.name == "LSPDFR Core")
        .unwrap();
    assert_eq!(core.unique_files, 3);
    let metro = s
        .collections
        .iter()
        .find(|g| g.name == "MetroCallouts")
        .unwrap();
    assert_eq!(metro.unique_files, 4);
    assert!(!metro
        .members
        .iter()
        .flat_map(|m| &m.files)
        .any(|p| p.contains("CountyCallouts")));
    assert!(s
        .unidentified
        .iter()
        .any(|m| m.location == "scripts/unidentified.ini"));
    assert!(s.suits.is_empty());
    assert_eq!(e.records().unwrap().len(), 0);
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM owned", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read(
            f.root
                .join("scripts/TheSpiderman script files/Sounds/basic melee 7.wav")
        )
        .unwrap(),
        b"synthetic support"
    );
}
#[test]
fn multiple_hero_versions_do_not_claim_a_shared_loader() {
    let f = Fixture::new();
    for p in [
        "scripts/Hulk.dll",
        "scripts/HULK v3 Files/Config/settings.ini",
        "scripts/HULK v4 Files/Config/settings.ini",
    ] {
        put(&f, p, b"synthetic");
    }
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    let groups: Vec<_> = s
        .collections
        .iter()
        .filter(|g| g.name == "Hulk Mod")
        .collect();
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|g| !g
        .members
        .iter()
        .flat_map(|m| &m.files)
        .any(|p| p == "scripts/Hulk.dll")));
    assert!(s.collections.iter().any(|g| g
        .members
        .iter()
        .flat_map(|m| &m.files)
        .any(|p| p == "scripts/Hulk.dll")));
}
#[test]
fn declared_local_artwork_is_bounded_verified_cached_and_never_grants_management() {
    let f = Fixture::new();
    put(&f, "scripts/Wardrobe/Models/Officer.ydd", b"model");
    put(&f, "scripts/Wardrobe/Models/Officer.ytd", b"texture");
    fs::create_dir_all(f.root.join("scripts/Wardrobe/Artwork")).unwrap();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([120, 40, 190, 255]))
        .save(f.root.join("scripts/Wardrobe/Artwork/cover.png"))
        .unwrap();
    put(&f,"scripts/Wardrobe/zoxryn.json",br#"{"name":"Officer Wardrobe","organization":{"preview":"Artwork/cover.png","suits":[{"key":"officer","name":"Officer Uniform","category":"Police Uniforms","files":["Models/Officer.ydd","Models/Officer.ytd"],"preview":"Artwork/cover.png"}]}}"#);
    let e = f.engine();
    let before = hash_file(&f.root.join("scripts/Wardrobe/Artwork/cover.png")).unwrap();
    let s = e.organization_snapshot().unwrap();
    let cover = s
        .collections
        .iter()
        .find(|g| g.name == "Officer Wardrobe")
        .unwrap()
        .preview
        .as_ref()
        .unwrap();
    assert_eq!(s.suits[0].preview.as_ref(), Some(cover));
    assert!(e.read_preview(cover).is_ok());
    assert!(!s.suits[0].manageable);
    assert_eq!(
        e.organization_snapshot().unwrap().suits[0].preview.as_ref(),
        Some(cover)
    );
    assert_eq!(
        before,
        hash_file(&f.root.join("scripts/Wardrobe/Artwork/cover.png")).unwrap()
    );
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM owned", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn invalid_or_escaping_declared_artwork_is_not_used() {
    let f = Fixture::new();
    put(&f, "scripts/Bad/cover.png", b"not an image");
    put(
        &f,
        "scripts/Bad/zoxryn.json",
        br#"{"name":"Bad Artwork","organization":{"preview":"cover.png"}}"#,
    );
    put(
        &f,
        "scripts/Escape/zoxryn.json",
        br#"{"name":"Escaping Artwork","organization":{"preview":"../../outside.png"}}"#,
    );
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert!(s.collections.iter().all(|c| c.preview.is_none()));
    assert!(e
        .inventory()
        .unwrap()
        .unwrap()
        .items
        .iter()
        .any(|i| i.location == "scripts/Escape/zoxryn.json" && i.package_evidence.is_none()));
}
#[test]
fn descriptive_superhero_folders_group_support_files_without_new_ownership() {
    let f = Fixture::new();
    f.prereqs();
    let names = [
        ("TheSpiderman script files", "Spider-Man Mod"),
        ("HULK v3 Files", "Hulk Mod"),
        ("Batman script files", "Batman Mod"),
        ("IronmanV Files (v3 E.G.)", "Iron Man Mod"),
        ("Superman V2 script files", "Superman Mod"),
    ];
    for (folder, _) in names {
        for suffix in [
            "Sounds/basic melee 7.wav",
            "Sounds/basic melee 8.wav",
            "Suits/0_keepPlayerChar.ini",
            "Suits/0_keepPlayerChar_models_cfg.ini",
            "Images/HUD.png",
        ] {
            put(
                &f,
                &format!("scripts/{folder}/{suffix}"),
                b"synthetic support file",
            );
        }
    }
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    for (folder, name) in names {
        let c = s.collections.iter().find(|c| c.name == name).unwrap();
        assert_eq!(c.category, "Superhero Mods");
        assert_eq!(c.members.len(), 5);
        assert_eq!(c.unique_files, 5);
        assert!(c.relationship.contains("Inferred"));
        assert!(c.members.iter().all(|m| m
            .files
            .iter()
            .all(|p| p.starts_with(&format!("scripts/{folder}/")))));
    }
    assert!(s.suits.is_empty());
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM owned", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read(
            f.root
                .join("scripts/TheSpiderman script files/Sounds/basic melee 7.wav")
        )
        .unwrap(),
        b"synthetic support file"
    );
    assert_eq!(e.inventory().unwrap().unwrap().hashed_bytes, 0);
}
#[test]
fn separate_police_callouts_core_plugins_audio_and_shared_rage_references() {
    let f = Fixture::new();
    f.prereqs();
    for p in [
        "RAGEPluginHook.exe",
        "LSPDFR.asi",
        "plugins/LSPDFR/AlphaCallouts.dll",
        "plugins/LSPDFR/AlphaCallouts.ini",
        "plugins/LSPDFR/AlphaCallouts/Sounds/dispatch.wav",
        "plugins/LSPDFR/BetaCallouts.dll",
        "plugins/LSPDFR/BetaCallouts/Settings/options.xml",
        "plugins/LSPDFR/BodyCam.dll",
        "plugins/LSPDFR/BodyCam.ini",
        "lspdfr/audio/scanner/OfficerVoicePack/dispatch.wav",
    ] {
        put(&f, p, b"synthetic not executable");
    }
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    for name in [
        "AlphaCallouts",
        "BetaCallouts",
        "BodyCam",
        "OfficerVoicePack",
        "LSPDFR Core",
        "RAGE Plugin Hook (shared dependency)",
    ] {
        assert!(s.collections.iter().any(|c| c.name == name), "{name}");
    }
    let alpha = s
        .collections
        .iter()
        .find(|c| c.name == "AlphaCallouts")
        .unwrap();
    assert_eq!(
        alpha
            .members
            .iter()
            .filter(|m| m.role != "Shared dependency reference")
            .count(),
        3
    );
    assert!(alpha
        .members
        .iter()
        .any(|m| m.role == "Shared dependency reference" && m.name.contains("RAGE")));
    assert!(!alpha
        .members
        .iter()
        .flat_map(|m| &m.files)
        .any(|p| p.contains("Beta")));
    assert_eq!(
        s.collections
            .iter()
            .filter(|c| c.name == "AlphaCallouts")
            .count(),
        1
    );
    assert!(s.suits.is_empty());
    assert_eq!(e.records().unwrap().len(), 0);
}
#[test]
fn nested_packages_and_all_categories_keep_unknown_evidence_and_safe_boundaries() {
    let f = Fixture::new();
    f.prereqs();
    for p in [
        "scripts/Bundle/FirstPlugin/Main.cs",
        "scripts/Bundle/SecondPlugin/Main.cs",
        "scripts/UtilityPack/Config/options.ini",
        "lml/WeaponPack/stream/w_pistol.ytd",
        "reshade-shaders/NeonPreset/Shaders/neon.fx",
        "scripts/VehicleTools/Settings/config.xml",
        "mystery.zzz",
        "mods/unknown_resource.ymt",
    ] {
        put(&f, p, b"synthetic");
    }
    put(&f, "scripts/SolarWardrobe/config.ini", b"synthetic");
    put(
        &f,
        "scripts/SolarWardrobe/zoxryn.json",
        br#"{"name":"Solar Wardrobe","category":"Superheroes"}"#,
    );
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    let bundle = s.collections.iter().find(|c| c.name == "Bundle").unwrap();
    assert_eq!(
        bundle
            .members
            .iter()
            .filter(|m| m.reference.kind == "collection")
            .count(),
        2
    );
    for (name, category) in [
        ("UtilityPack", "Scripts & Plugins"),
        ("WeaponPack", "Weapons"),
        ("NeonPreset", "Maps & Graphics"),
        ("VehicleTools", "Vehicles"),
        ("Solar Wardrobe", "Superhero Mods"),
    ] {
        assert!(
            s.collections
                .iter()
                .any(|c| c.name == name && c.category == category),
            "{name}"
        );
    }
    assert!(s.unidentified.iter().any(|m| m.location == "mystery.zzz"));
    assert!(s.suits.is_empty());
    assert!(!s.candidates.is_empty());
    assert!(e
        .inventory()
        .unwrap()
        .unwrap()
        .items
        .iter()
        .any(|i| i.location == "mods/unknown_resource.ymt"));
}
#[test]
fn explicit_external_outfit_metadata_identifies_one_gallery_entry_without_unlocking_controls() {
    let f = Fixture::new();
    f.prereqs();
    put(
        &f,
        "scripts/Wardrobe/Models/Officer.ydd",
        b"synthetic model",
    );
    put(
        &f,
        "scripts/Wardrobe/Models/Officer.ytd",
        b"synthetic texture",
    );
    put(&f,"scripts/Wardrobe/zoxryn.json",br#"{"name":"Officer Wardrobe","category":"Police","requirements":["rph"],"organization":{"suits":[{"key":"officer","name":"Officer Uniform","category":"Police Uniforms","model":"officer_test","files":["Models/Officer.ydd","Models/Officer.ytd"]}]}}"#);
    let mut e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert_eq!(s.suits.len(), 1);
    let suit = &s.suits[0];
    assert_eq!(suit.name, "Officer Uniform");
    assert_eq!(suit.files.len(), 2);
    assert!(!s.candidates.iter().any(|candidate| candidate
        .files
        .iter()
        .any(|file| file.ends_with("Models/Officer.ydd"))));
    assert!(!suit.manageable);
    assert!(e.prepare_suit(&suit.id, "disable").is_err());
    assert!(s.health.issues.iter().any(|i| i.detail.contains("rph")));
    let id = s
        .collections
        .iter()
        .find(|c| c.name == "Officer Wardrobe")
        .unwrap()
        .id
        .clone();
    let refs: Vec<_> = s
        .collections
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .members
        .iter()
        .map(|m| m.reference.clone())
        .collect();
    e.save_virtual_group(&id, "My police wardrobe", "Police & LSPDFR", &refs, true)
        .unwrap();
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    let s = e.organization_snapshot().unwrap();
    let c = s.collections.iter().find(|c| c.id == id).unwrap();
    assert_eq!(c.name, "My police wardrobe");
    assert!(c.relationship.contains("Manually confirmed"));
    assert_eq!(e.records().unwrap().len(), 0);
}
#[test]
fn manual_identity_and_grouping_corrections_are_virtual_and_keep_all_raw_files() {
    let f = Fixture::new();
    put(&f, "scripts/ExamplePack/unknown.ydd", b"synthetic");
    put(&f, "scripts/ExamplePack/config.ini", b"config");
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert!(s.suits.is_empty());
    let candidate = &s.candidates[0];
    e.rename_suit(&candidate.id, "Edited label only", "Custom Peds")
        .unwrap();
    assert!(e.organization_snapshot().unwrap().suits.is_empty());
    e.confirm_skin_identity(
        &candidate.id,
        "Manually identified outfit",
        "Custom Peds",
        true,
    )
    .unwrap();
    let named = e.organization_snapshot().unwrap();
    assert_eq!(named.suits.len(), 1);
    assert!(!named.suits[0].manageable);
    let group = named
        .collections
        .iter()
        .find(|c| c.name == "ExamplePack")
        .unwrap();
    let retained = group
        .members
        .iter()
        .find(|m| m.files.iter().any(|p| p.ends_with("config.ini")))
        .unwrap()
        .reference
        .clone();
    e.save_virtual_group(
        &group.id,
        "Corrected virtual folder",
        "Other Mods",
        &[retained],
        false,
    )
    .unwrap();
    assert_eq!(
        e.inventory()
            .unwrap()
            .unwrap()
            .items
            .iter()
            .filter(|i| i.location.starts_with("scripts/ExamplePack/"))
            .count(),
        2
    );
    assert!(f.root.join("scripts/ExamplePack/unknown.ydd").exists());
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM owned", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn unique_file_counts_include_game_root_without_changing_ownership() {
    let first = Fixture::new();
    first.prereqs();
    let second = Fixture::new();
    second.prereqs();
    let mut e = first.engine();
    let a = e.import(&first.asi("first.zip", b"first package")).unwrap();
    perform(&mut e, &a.id, "install");
    e.select_game(&second.root, "Synthetic second installation")
        .unwrap();
    let b = e
        .import(&first.asi("second.zip", b"second package"))
        .unwrap();
    perform(&mut e, &b.id, "install");
    let owners = (e.ownership(&a.id).unwrap(), e.ownership(&b.id).unwrap());
    let refs = [
        MemberRef {
            kind: "managed".into(),
            id: a.id.clone(),
        },
        MemberRef {
            kind: "managed".into(),
            id: b.id.clone(),
        },
    ];
    let id = e
        .save_collection(None, "Two games", "", "Custom", &refs)
        .unwrap();
    let c = e
        .organization_snapshot()
        .unwrap()
        .collections
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    assert_eq!(c.unique_files, 4);
    assert_eq!(
        (e.ownership(&a.id).unwrap(), e.ownership(&b.id).unwrap()),
        owners
    );
}

#[test]
fn schema_three_reopen_preserves_records_archives_blobs_and_custom_metadata() {
    fn rows(e: &Engine, table: &str) -> Vec<Vec<String>> {
        let mut statement =
            e.db.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
        let count = statement.column_count();
        statement
            .query_map([], |row| {
                Ok((0..count)
                    .map(|i| row.get_ref(i).unwrap().as_str().unwrap().to_string())
                    .collect())
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }
    fn blobs(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                out.insert(
                    entry.file_name().to_string_lossy().to_string(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
        out
    }
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let record = e
        .import(&f.asi("preservation.zip", b"synthetic preservation bytes"))
        .unwrap();
    perform(&mut e, &record.id, "install");
    perform(&mut e, &record.id, "disable");
    e.save_collection(
        None,
        "Preserved collection",
        "Existing schema 3",
        "Custom",
        &[MemberRef {
            kind: "managed".into(),
            id: record.id.clone(),
        }],
    )
    .unwrap();
    e.db.execute(
        "INSERT INTO settings VALUES ('future.setting','retained')",
        [],
    )
    .unwrap();
    let tables = [
        "settings",
        "mods",
        "owned",
        "journals",
        "collections",
        "collection_members",
        "visual_metadata",
    ];
    let before: Vec<_> = tables.iter().map(|table| rows(&e, table)).collect();
    let files = ["archives", "blobs", "journals"].map(|dir| blobs(&f.data.join(dir)));
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    let _ = e.organization_snapshot().unwrap();
    assert_eq!(
        before,
        tables
            .iter()
            .map(|table| rows(&e, table))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        files,
        ["archives", "blobs", "journals"].map(|dir| blobs(&f.data.join(dir)))
    );
    assert_eq!(
        e.db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn manual_assignment_resolves_virtual_parent_and_removed_members_remain_inspectable() {
    let f = Fixture::new();
    put(&f, "orphan.fixture", b"unidentified");
    put(&f, "scripts/ExamplePack/config.ini", b"config");
    let e = f.engine();
    let original = e.organization_snapshot().unwrap();
    let orphan = original
        .unidentified
        .iter()
        .find(|m| m.location == "orphan.fixture")
        .unwrap();
    let group = original
        .collections
        .iter()
        .find(|c| c.name == "ExamplePack")
        .unwrap();
    e.save_virtual_group(
        &group.id,
        "Corrected folder",
        "Other Mods",
        &[orphan.reference.clone()],
        true,
    )
    .unwrap();
    let updated = e.organization_snapshot().unwrap();
    assert!(!updated
        .unidentified
        .iter()
        .any(|m| m.reference == orphan.reference));
    assert!(updated
        .unidentified
        .iter()
        .any(|m| m.location == "scripts/ExamplePack/config.ini"));
    assert!(updated
        .available_members
        .iter()
        .any(|m| m.reference == orphan.reference));
    assert_eq!(
        e.inventory().unwrap().unwrap().items.len(),
        original
            .available_members
            .iter()
            .filter(|m| m.reference.kind == "external")
            .count()
    );
    assert_eq!(
        fs::read(f.root.join("orphan.fixture")).unwrap(),
        b"unidentified"
    );
}

#[test]
fn seven_police_sections_preserve_distinct_packages_and_canonical_shared_tool_files() {
    let f = Fixture::new();
    f.prereqs();
    for path in [
        "RAGEPluginHook.exe",
        "LSPDFR.asi",
        "plugins/LSPDFR/AlphaCallouts.dll",
        "plugins/LSPDFR/BetaCallouts.dll",
        "plugins/LSPDFR/BodyCamera.dll",
        "lml/PoliceVehiclePack/stream/police.ytd",
        "lml/PoliceUniformPack/stream/officer.ydd",
        "lspdfr/audio/scanner/CityVoicePack/dispatch.wav",
    ] {
        put(&f, path, b"synthetic");
    }
    let e = f.engine();
    let snapshot = e.organization_snapshot().unwrap();
    for section in [
        "LSPDFR Core",
        "RAGE Plugin Hook (shared dependency)",
        "Police Callout Packs",
        "Police Plugins",
        "Police Vehicle Packs",
        "Police Uniforms & Peds",
        "Police Audio and Other Add-ons",
    ] {
        assert!(
            snapshot
                .collections
                .iter()
                .any(|c| c.description.starts_with(&format!("{section} · "))),
            "{section}"
        );
    }
    let tool = snapshot
        .collections
        .iter()
        .find(|c| c.name == "RAGE Plugin Hook (shared dependency)")
        .unwrap();
    assert_eq!(tool.unique_files, 1);
    assert!(snapshot.suits.is_empty());
    assert_eq!(e.records().unwrap().len(), 0);
}

#[test]
fn corrected_nested_groups_refresh_parent_counts_names_and_previews_without_file_changes() {
    let f = Fixture::new();
    for path in [
        "scripts/Bundle/FirstPlugin/Main.cs",
        "scripts/Bundle/FirstPlugin/config.ini",
        "scripts/Bundle/SecondPlugin/Main.cs",
    ] {
        put(&f, path, b"synthetic");
    }
    let e = f.engine();
    let snapshot = e.organization_snapshot().unwrap();
    let child = snapshot
        .collections
        .iter()
        .find(|c| c.name == "FirstPlugin")
        .unwrap();
    let reference = child
        .members
        .iter()
        .find(|m| m.location.ends_with("Main.cs"))
        .unwrap()
        .reference
        .clone();
    e.save_virtual_group(
        &child.id,
        "Corrected child",
        "Scripts & Plugins",
        &[reference],
        true,
    )
    .unwrap();
    let preview = f.data.join("synthetic-preview.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([126, 62, 195, 255]))
        .save(&preview)
        .unwrap();
    e.assign_preview("collection", &child.id, Some(&preview))
        .unwrap();
    let updated = e.organization_snapshot().unwrap();
    let child = updated
        .collections
        .iter()
        .find(|c| c.id == child.id)
        .unwrap();
    assert_eq!(child.unique_files, 1);
    assert!(child.preview.is_some());
    let parent = updated
        .collections
        .iter()
        .find(|c| c.name == "Bundle")
        .unwrap();
    assert_eq!(parent.unique_files, 2);
    let link = parent
        .members
        .iter()
        .find(|m| m.reference.id == child.id)
        .unwrap();
    assert_eq!(link.name, "Corrected child");
    assert_eq!(link.preview, child.preview);
    assert_eq!(link.files, vec!["scripts/Bundle/FirstPlugin/Main.cs"]);
    assert!(updated
        .unidentified
        .iter()
        .any(|m| m.location.ends_with("FirstPlugin/config.ini")));
    assert!(f
        .root
        .join("scripts/Bundle/FirstPlugin/config.ini")
        .is_file());
    assert_eq!(e.records().unwrap().len(), 0);
}
