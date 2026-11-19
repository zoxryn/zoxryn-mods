use super::*;
use crate::{
    organization::{aggregate_status, parse_ped_metadata, MemberRef},
    previews,
};
use std::{collections::BTreeMap, io::Cursor};
fn suit_zip(f: &Fixture, name: &str, multiple: bool) -> PathBuf {
    let suits = if multiple {
        serde_json::json!([{"key":"one","name":"Suit One","category":"Superhero Suits","files":["scripts/Solar.cs"],"standalone":true},{"key":"two","name":"Suit Two","category":"Civilian Outfits","files":["scripts/Solar.ini"],"standalone":true}])
    } else {
        serde_json::json!([{"key":"solar","name":"Solar Suit","category":"Superhero Suits","model":"player_zero","files":["scripts/Solar.cs","scripts/Solar.ini"],"standalone":true}])
    };
    let manifest=serde_json::json!({"name":name,"edition":"Legacy","category":"Characters","organization":{"collection_name":format!("{name} collection"),"suits":suits}}).to_string();
    f.zip(
        &format!("{name}.zip"),
        &[
            ("scripts/Solar.cs", b"// SYNTHETIC: no executable code"),
            ("scripts/Solar.ini", b"[synthetic]\nactive=true"),
            ("zoxryn.json", manifest.as_bytes()),
        ],
    )
}
fn members(e: &Engine) -> MemberRef {
    MemberRef {
        kind: "managed".into(),
        id: e.records().unwrap()[0].id.clone(),
    }
}
fn logical_tables(db: &Connection) -> BTreeMap<String, Vec<Vec<rusqlite::types::Value>>> {
    [
        "settings",
        "mods",
        "owned",
        "journals",
        "scan_runs",
        "external_mods",
    ]
    .into_iter()
    .map(|table| {
        let mut s = db
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
            .unwrap();
        let count = s.column_count();
        let rows = s
            .query_map([], |r| {
                Ok((0..count).map(|i| r.get(i).unwrap()).collect::<Vec<_>>())
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        (table.into(), rows)
    })
    .collect()
}
#[test]
fn automatic_grouping_uses_packages_without_merging_similar_names() {
    let f = Fixture::new();
    let mut e = f.engine();
    let one = f.asi("Similar.zip", b"one");
    let two = f.asi("Similar2.zip", b"two");
    let first = e.import(&one).unwrap();
    let second = e.import(&two).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(e.import(&one).unwrap().id, first.id);
    let snapshot = e.organization_snapshot().unwrap();
    assert_eq!(
        snapshot
            .collections
            .iter()
            .filter(|c| c.id.starts_with("package:"))
            .count(),
        2
    );
    assert!(snapshot
        .collections
        .iter()
        .filter(|c| c.id.starts_with("package:"))
        .all(|c| c.members.len() == 1 && c.unique_files == 2));
    assert_eq!(e.records().unwrap().len(), 2);
}
#[test]
fn manual_collection_crud_deduplicates_links_without_copying_ownership() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&suit_zip(&f, "Solar", false)).unwrap();
    perform(&mut e, &m.id, "install");
    let member = members(&e);
    let id = e
        .save_collection(None, "Weekend", "my selection", "Custom", &[member.clone()])
        .unwrap();
    assert!(e
        .save_collection(None, "Weekend", "", "Custom", &[])
        .is_err());
    assert!(e
        .save_collection(
            None,
            "Duplicate links",
            "",
            "Custom",
            &[member.clone(), member.clone()]
        )
        .is_err());
    let suit = e.organization_snapshot().unwrap().suits[0].clone();
    e.save_collection(
        Some(&id),
        "Heroes",
        "renamed",
        "Superheroes",
        &[
            member.clone(),
            MemberRef {
                kind: "suit".into(),
                id: suit.id,
            },
        ],
    )
    .unwrap();
    e.save_collection(None, "Favorites", "", "Custom", &[member])
        .unwrap();
    let snapshot = e.organization_snapshot().unwrap();
    let c = snapshot.collections.iter().find(|c| c.id == id).unwrap();
    assert_eq!(c.unique_files, 2);
    assert_eq!(c.members.len(), 2);
    assert_eq!(c.name, "Heroes");
    assert_eq!(e.records().unwrap().len(), 1);
    assert_eq!(e.ownership(&m.id).unwrap().len(), 2);
    e.remove_collection(&id).unwrap();
    assert!(e
        .organization_snapshot()
        .unwrap()
        .collections
        .iter()
        .all(|c| c.id != id));
    assert!(f.root.join("scripts/Solar.cs").exists());
    assert_eq!(e.snapshot().unwrap().backups.len(), 1);
}
#[test]
fn parent_status_reports_partial_unknown_and_missing_members() {
    assert_eq!(aggregate_status(["enabled", "disabled"]), "partial");
    assert_eq!(aggregate_status(["enabled", "unknown"]), "unknown");
    assert_eq!(
        aggregate_status(["enabled", "needs_review"]),
        "needs_review"
    );
    let f = Fixture::new();
    f.prereqs();
    fs::write(f.root.join("unknown.bin"), b"unidentified").unwrap();
    let mut e = f.engine();
    let m = e.import(&f.asi("package.zip", b"package")).unwrap();
    perform(&mut e, &m.id, "install");
    e.scan_installed().unwrap();
    let external = e
        .organization_snapshot()
        .unwrap()
        .available_members
        .into_iter()
        .find(|c| c.location == "unknown.bin")
        .unwrap();
    let id = e
        .save_collection(
            None,
            "Mixed",
            "",
            "Custom",
            &[members(&e), external.reference],
        )
        .unwrap();
    assert_eq!(
        e.organization_snapshot()
            .unwrap()
            .collections
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .status,
        "unknown"
    );
    fs::remove_file(f.root.join("unknown.bin")).unwrap();
    e.scan_installed().unwrap();
    let s = e.organization_snapshot().unwrap();
    let c = s.collections.iter().find(|c| c.id == id).unwrap();
    assert_eq!(c.missing_members, 1);
    assert_eq!(c.status, "needs_review");
    assert_eq!(c.members.len(), 2);
    let retained: Vec<_> = c
        .members
        .iter()
        .filter(|m| m.status != "missing")
        .map(|m| m.reference.clone())
        .collect();
    e.save_collection(Some(&id), "Mixed", "", "Custom", &retained)
        .unwrap();
    let updated = e
        .organization_snapshot()
        .unwrap()
        .collections
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    assert_eq!(updated.missing_members, 0);
    assert_eq!(e.ownership(&m.id).unwrap().len(), 2);
}
#[test]
fn declared_parent_children_do_not_duplicate_mod_records_or_unlock_shared_suits() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&suit_zip(&f, "Hero pack", true)).unwrap();
    perform(&mut e, &m.id, "install");
    let snapshot = e.organization_snapshot().unwrap();
    assert_eq!(snapshot.suits.len(), 2);
    assert!(snapshot.suits.iter().all(|s| !s.manageable));
    let c = snapshot
        .collections
        .iter()
        .find(|c| c.id == format!("package:{}", m.id))
        .unwrap();
    assert_eq!(c.members.len(), 3);
    assert_eq!(c.unique_files, 2);
    assert_eq!(e.records().unwrap().len(), 1);
    assert_eq!(e.ownership(&m.id).unwrap().len(), 2);
    assert!(e.prepare_suit(&snapshot.suits[0].id, "disable").is_err());
}
#[test]
fn independent_suit_disable_and_reenable_use_approved_verified_backups() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&suit_zip(&f, "Solar", false)).unwrap();
    perform(&mut e, &m.id, "install");
    let suit = e.organization_snapshot().unwrap().suits[0].clone();
    assert!(suit.manageable);
    let before = fs::read(f.root.join("scripts/Solar.cs")).unwrap();
    let p = e.prepare_suit(&suit.id, "disable").unwrap();
    assert!(p.blockers.is_empty(), "{:?}", p.blockers);
    assert_eq!(p.files.len(), 2);
    assert!(e.apply(&p.token, false, true).is_err());
    assert!(e.apply(&p.token, true, true).is_err());
    e.apply_with_ownership(&p.token, true, true, true).unwrap();
    assert!(!f.root.join("scripts/Solar.cs").exists());
    assert_eq!(
        e.organization_snapshot().unwrap().suits[0].status,
        "disabled"
    );
    let p = e.prepare_suit(&suit.id, "enable").unwrap();
    assert!(p.blockers.is_empty());
    e.apply_with_ownership(&p.token, true, true, true).unwrap();
    assert_eq!(fs::read(f.root.join("scripts/Solar.cs")).unwrap(), before);
    assert_eq!(e.snapshot().unwrap().backups.len(), 3);
    assert_eq!(
        fs::read(f.root.join("update/update.rpf")).unwrap(),
        b"original RPF - must never change"
    );
    let backup = e.snapshot().unwrap().backups[0].id.clone();
    let p = e.prepare_restore(&backup).unwrap();
    e.apply_with_ownership(&p.token, true, true, true).unwrap();
    assert_eq!(e.record(&m.id).unwrap().status, "disabled");
}
#[test]
fn new_shared_companion_after_review_blocks_suit_approval() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&suit_zip(&f, "Solar", false)).unwrap();
    perform(&mut e, &m.id, "install");
    let suit = e.organization_snapshot().unwrap().suits[0].id.clone();
    let p = e.prepare_suit(&suit, "disable").unwrap();
    fs::write(f.root.join("scripts/Solar.cfg"), b"unlisted dependency").unwrap();
    assert!(e.apply_with_ownership(&p.token, true, true, true).is_err());
    assert!(f.root.join("scripts/Solar.cs").exists());
    assert_eq!(e.record(&m.id).unwrap().status, "enabled");
    assert_eq!(e.snapshot().unwrap().backups.len(), 1);
}
#[test]
fn shared_hardlinks_original_replacements_and_missing_dependencies_block_suits() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&suit_zip(&f, "Solar", false)).unwrap();
    perform(&mut e, &m.id, "install");
    let suit = e.organization_snapshot().unwrap().suits[0].id.clone();
    fs::hard_link(
        f.root.join("scripts/Solar.cs"),
        f.temp.path().join("alias.cs"),
    )
    .unwrap();
    assert!(!e
        .prepare_suit(&suit, "disable")
        .unwrap()
        .blockers
        .is_empty());
    fs::remove_file(f.temp.path().join("alias.cs")).unwrap();
    let p = e.prepare_suit(&suit, "disable").unwrap();
    e.apply_with_ownership(&p.token, true, true, true).unwrap();
    fs::remove_file(f.root.join("ScriptHookVDotNet.asi")).unwrap();
    assert!(e
        .prepare_suit(&suit, "enable")
        .unwrap()
        .blockers
        .iter()
        .any(|b| b.contains("dependency")));
    let f = Fixture::new();
    f.prereqs();
    fs::create_dir_all(f.root.join("scripts")).unwrap();
    fs::write(f.root.join("scripts/Solar.cs"), b"another original").unwrap();
    let mut e = f.engine();
    let m = e
        .import(&suit_zip(&f, "Original replacement", false))
        .unwrap();
    perform(&mut e, &m.id, "install");
    assert!(!e.organization_snapshot().unwrap().suits[0].manageable);
}
#[test]
fn schema_two_migration_preserves_all_legacy_data_and_creates_verified_checkpoint() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("before.zip", b"existing")).unwrap();
    perform(&mut e, &m.id, "install");
    e.scan_installed().unwrap();
    e.db.execute(
        "INSERT INTO settings VALUES('collections','{\"opaque\":true}')",
        [],
    )
    .unwrap();
    e.db.execute_batch("DROP TABLE collection_members;DROP TABLE collections;DROP TABLE visual_metadata;PRAGMA user_version=2;").unwrap();
    let before = logical_tables(&e.db);
    let game_bytes = fs::read(f.root.join("Example.asi")).unwrap();
    let archive_hash = hash_file(Path::new(&m.archive_path)).unwrap();
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    assert_eq!(logical_tables(&e.db), before);
    assert_eq!(fs::read(f.root.join("Example.asi")).unwrap(), game_bytes);
    assert_eq!(hash_file(Path::new(&m.archive_path)).unwrap(), archive_hash);
    assert_eq!(e.snapshot().unwrap().backups.len(), 1);
    assert_eq!(
        e.db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    let backup = fs::read_dir(f.data.join("schema-migrations"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let copy = Connection::open(backup).unwrap();
    assert_eq!(logical_tables(&copy), before);
    assert_eq!(
        copy.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}
#[test]
fn previews_are_bounded_cached_verified_and_do_not_change_game_files() {
    let f = Fixture::new();
    let e = f.engine();
    let image = image::DynamicImage::new_rgba8(32, 32);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let first = previews::store(&f.data, bytes.get_ref()).unwrap();
    let again = previews::store(&f.data, bytes.get_ref()).unwrap();
    assert_eq!(first, again);
    assert_eq!(fs::read_dir(f.data.join("thumbnails")).unwrap().count(), 1);
    assert!(e.read_preview(&first).unwrap().len() < previews::MAX_THUMBNAIL);
    assert!(previews::normalize(b"<svg onload='execute' />").is_err());
    assert!(e.read_preview("../../secret").is_err());
    assert!(previews::normalize(&vec![0; previews::MAX_IMAGE as usize + 1]).is_err());
    fs::write(
        f.data.join("thumbnails").join(format!("{first}.png")),
        b"tampered",
    )
    .unwrap();
    assert!(e.read_preview(&first).is_err());
    assert_eq!(
        fs::read(f.root.join("update/update.rpf")).unwrap(),
        b"original RPF - must never change"
    );
}
#[test]
fn loose_ped_metadata_is_read_only_bounded_and_never_unlocks_registry_edits() {
    let xml=br#"<CPedModelInfo__InitDataList><InitDatas><Item><Name>synthetic_ped</Name><Pedtype>CIVMALE</Pedtype></Item></InitDatas></CPedModelInfo__InitDataList>"#;
    assert_eq!(parse_ped_metadata(xml).unwrap(), vec!["synthetic_ped"]);
    assert!(parse_ped_metadata(b"<!DOCTYPE x [<!ENTITY a SYSTEM 'file:///never'>]><x/>").is_err());
    assert!(parse_ped_metadata(b"<Unknown><Name>fake</Name></Unknown>").is_err());
    let f = Fixture::new();
    let path = f.root.join("mods/update/x64/dlcpacks/addonpeds/peds.meta");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, xml).unwrap();
    let before = hash_file(&path).unwrap();
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert!(s.suits.is_empty());
    let suit = s
        .candidates
        .iter()
        .find(|s| s.model.as_deref() == Some("synthetic_ped"))
        .unwrap();
    assert!(!suit.manageable);
    assert_eq!(suit.status, "needs_review");
    assert_eq!(hash_file(&path).unwrap(), before);
    assert_eq!(e.inventory().unwrap().unwrap().hashed_bytes, 0);
}
#[test]
fn thumbnail_assignment_and_collection_metadata_survive_reopening() {
    let f = Fixture::new();
    let mut e = f.engine();
    e.import(&f.asi("one.zip", b"one")).unwrap();
    let member = members(&e);
    let id = e
        .save_collection(None, "Saved", "", "Custom", &[member])
        .unwrap();
    let image = f.temp.path().join("art.png");
    image::DynamicImage::new_rgba8(16, 16).save(&image).unwrap();
    e.assign_preview("collection", &id, Some(&image)).unwrap();
    let preview = e
        .organization_snapshot()
        .unwrap()
        .collections
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .preview
        .clone()
        .unwrap();
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    let s = e.organization_snapshot().unwrap();
    assert_eq!(
        s.collections
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .preview
            .as_deref(),
        Some(preview.as_str())
    );
    assert!(!e.read_preview(&preview).unwrap().is_empty());
}

fn rpf_fixture(names: &[String], encryption: u32) -> Vec<u8> {
    let mut strings = vec![0];
    let mut offsets = Vec::new();
    for name in names {
        offsets.push(strings.len() as u32);
        strings.extend_from_slice(name.as_bytes());
        strings.push(0);
    }
    let mut bytes = Vec::new();
    for word in [
        0x52504637u32,
        names.len() as u32 + 1,
        strings.len() as u32,
        encryption,
    ] {
        bytes.extend(word.to_le_bytes());
    }
    for word in [0u32, 0x7fffff00, 1, names.len() as u32] {
        bytes.extend(word.to_le_bytes());
    }
    for offset in offsets {
        bytes.extend((offset as u64 | (0x80000000u64 << 32)).to_le_bytes());
        bytes.extend([0; 8]);
    }
    bytes.extend(strings);
    bytes
}
#[test]
fn character_resources_beyond_the_general_rpf_sample_are_bounded_and_grouped_by_real_package() {
    let f = Fixture::new();
    let names = (0..270)
        .map(|i| format!("unidentified_{i}.ydd"))
        .collect::<Vec<_>>();
    let bytes = rpf_fixture(&names, 0);
    let archive = f.root.join("mods/update/x64/dlcpacks/wardrobe/dlc.rpf");
    fs::create_dir_all(archive.parent().unwrap()).unwrap();
    fs::write(&archive, &bytes).unwrap();
    let hash = hash_file(&archive).unwrap();
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert!(s.suits.is_empty());
    assert_eq!(s.candidates.len(), 256);
    assert!(s
        .candidates
        .iter()
        .all(|s| !s.manageable && s.model.is_none()));
    assert!(s
        .warnings
        .iter()
        .any(|w| w.contains("Character-resource limit")));
    let package = s
        .collections
        .iter()
        .find(|c| c.name == "DLC pack: wardrobe")
        .unwrap();
    assert!(
        package
            .members
            .iter()
            .filter(|m| m.reference.kind == "suit")
            .count()
            == 256
    );
    assert_eq!(package.unique_files, 1);
    assert_eq!(hash_file(&archive).unwrap(), hash);
    let rpf = e
        .inventory()
        .unwrap()
        .unwrap()
        .items
        .into_iter()
        .find_map(|i| i.rpf)
        .unwrap();
    assert_eq!(rpf.samples.len(), 80);
    assert_eq!(rpf.character_resources.len(), 256);
    assert!(rpf.character_resources_truncated);
}
#[test]
fn encrypted_addonpeds_archive_remains_unknown_and_unsafe_controls_stay_blocked() {
    let f = Fixture::new();
    let path = f.root.join("mods/update/x64/dlcpacks/addonpeds/dlc.rpf");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, rpf_fixture(&["unknown.ydd".into()], 0x414553)).unwrap();
    let mut e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert!(s.suits.is_empty());
    assert_eq!(s.candidates.len(), 1);
    assert_eq!(s.candidates[0].model, None);
    assert!(!s.candidates[0].manageable);
    assert!(e.prepare_suit(&s.candidates[0].id, "disable").is_err());
    assert!(s.candidates[0].name.contains("Unknown archive contents"));
}
#[test]
fn declared_artwork_uses_only_its_explicit_archive_reference() {
    let f = Fixture::new();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(24, 24)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let metadata=serde_json::json!({"name":"Declared artwork","edition":"Legacy","organization":{"preview":"art.png","suits":[{"key":"one","name":"Declared suit","category":"Civilian Outfits","files":["scripts/Look.cs"],"standalone":true,"preview":"art.png"}]}}).to_string();
    let zip = f.zip(
        "artwork.zip",
        &[
            ("Wrapper/scripts/Look.cs", b"// synthetic"),
            ("Wrapper/art.png", png.get_ref()),
            ("Wrapper/zoxryn.json", metadata.as_bytes()),
        ],
    );
    let mut e = f.engine();
    let m = e.import(&zip).unwrap();
    let s = e.organization_snapshot().unwrap();
    assert!(s.suits[0].preview.is_some());
    assert!(s
        .collections
        .iter()
        .find(|c| c.id == format!("package:{}", m.id))
        .unwrap()
        .preview
        .is_some());
    assert_eq!(fs::read_dir(f.data.join("thumbnails")).unwrap().count(), 1);
    let metadata = serde_json::json!({"organization":{"preview":"../../secret.png"}}).to_string();
    let bad = f.zip(
        "bad-art.zip",
        &[
            ("Look.asi", b"synthetic"),
            ("zoxryn.json", metadata.as_bytes()),
        ],
    );
    assert!(e.import(&bad).is_err());
}
#[test]
fn metadata_references_are_exact_and_invalid_shared_claims_do_not_make_suits_manageable() {
    let f = Fixture::new();
    let bad=serde_json::json!({"organization":{"suits":[{"key":"one","name":"Misleading","category":"Superhero Suits","files":["scripts/Missing.cs"],"standalone":true}]}}).to_string();
    let path = f.zip(
        "wrong-members.zip",
        &[
            ("scripts/Real.cs", b"// synthetic"),
            ("zoxryn.json", bad.as_bytes()),
        ],
    );
    assert!(crate::archive::inspect(&path).is_err());
    let duplicate=serde_json::json!({"organization":{"suits":[{"key":"one","name":"Duplicate","files":["scripts/Real.cs","scripts/Real.cs"],"standalone":true}]}}).to_string();
    let path = f.zip(
        "duplicate-members.zip",
        &[
            ("scripts/Real.cs", b"// synthetic"),
            ("zoxryn.json", duplicate.as_bytes()),
        ],
    );
    assert!(crate::archive::inspect(&path).is_err());
}
#[test]
fn dimension_bombs_malformed_images_and_linked_preview_paths_are_rejected() {
    let f = Fixture::new();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(4097, 1)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    assert!(previews::normalize(png.get_ref()).is_err());
    assert!(previews::normalize(b"\x89PNG\r\n\x1a\ntruncated").is_err());
    let outside = f.temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    image::DynamicImage::new_rgba8(2, 2)
        .save(outside.join("image.png"))
        .unwrap();
    #[cfg(windows)]
    {
        let junction = f.temp.path().join("linked");
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(status.status.success());
        assert!(previews::import(&f.data, &junction.join("image.png")).is_err());
    }
}
#[test]
fn detection_budget_bounds_large_character_libraries_without_hashing_model_files() {
    let f = Fixture::new();
    let dir = f.root.join("mods/update/x64/dlcpacks/large");
    fs::create_dir_all(&dir).unwrap();
    for i in 0..2100 {
        fs::write(
            dir.join(format!("resource_{i}.ydd")),
            b"nonfunctional synthetic model",
        )
        .unwrap();
    }
    let started = std::time::Instant::now();
    let e = f.engine();
    let s = e.organization_snapshot().unwrap();
    assert!(s.suits.is_empty());
    assert_eq!(s.candidates.len(), 2048);
    assert!(s.warnings.iter().any(|w| w.contains("2,048")));
    assert!(started.elapsed() < std::time::Duration::from_secs(8));
    assert_eq!(e.inventory().unwrap().unwrap().hashed_bytes, 0);
    assert!(s.candidates.iter().all(|s| !s.manageable));
}
