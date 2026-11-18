use super::*;
use crate::{rpf, scanner};
use std::{collections::BTreeMap, io::Cursor};
#[test]
fn a_new_hardlink_blocks_toggle_of_an_already_adopted_entrypoint() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let mut e = f.engine();
    let id = adopted(&f, &mut e, "Traffic.asi");
    fs::hard_link(
        f.root.join("Traffic.asi"),
        f.temp.path().join("new-external-alias"),
    )
    .unwrap();
    assert!(e
        .prepare(&id, "disable")
        .unwrap()
        .blockers
        .iter()
        .any(|b| b.contains("hard link")));
    assert!(f.root.join("Traffic.asi").exists());
}
#[test]
fn multiply_parented_rpf_directory_graph_is_rejected_without_expansion() {
    let mut bytes = rpf_fixture(&["one", "two", "three"], 0);
    for i in 1..=3 {
        let start = 16 + i * 16;
        bytes[start + 4..start + 8].copy_from_slice(&0x7fffff00u32.to_le_bytes());
        bytes[start + 8..start + 12].copy_from_slice(&1u32.to_le_bytes());
        bytes[start + 12..start + 16].copy_from_slice(&3u32.to_le_bytes());
    }
    let mut budget = 10000;
    let result = rpf::inspect(&mut Cursor::new(&bytes), bytes.len() as u64, &mut budget);
    assert_eq!(result.state, "Unavailable");
    assert!(result.samples.is_empty());
}
#[test]
fn matching_resource_folder_blocks_an_asi_with_ambiguous_ownership() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    put(&f, "Traffic/resources.bin", b"resources");
    let mut e = f.engine();
    let inventory = e.inventory().unwrap().unwrap();
    let candidate = item(&inventory, "Traffic.asi");
    assert!(!candidate.adoptable);
    assert!(!e
        .prepare_adoption(&candidate.id)
        .unwrap()
        .blockers
        .is_empty());
}
#[test]
fn newer_database_schema_is_not_downgraded() {
    let f = Fixture::new();
    let e = f.engine();
    e.db.execute_batch("PRAGMA user_version=99;").unwrap();
    drop(e);
    assert!(Engine::open(&f.data).is_err());
    let db = Connection::open(f.data.join("zoxryn.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        99
    );
}
fn put(f: &Fixture, path: &str, bytes: &[u8]) {
    let target = f.root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, bytes).unwrap();
}
fn item<'a>(inventory: &'a ScanInventory, path: &str) -> &'a DiscoveredMod {
    inventory
        .items
        .iter()
        .find(|i| i.location == path)
        .unwrap_or_else(|| panic!("Missing scan item {path}"))
}
fn rpf_fixture(names: &[&str], encryption: u32) -> Vec<u8> {
    let mut strings = vec![0u8];
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
        let packed = offset as u64 | (0x80000000u64 << 32);
        bytes.extend(packed.to_le_bytes());
        bytes.extend([0u8; 8]);
    }
    bytes.extend(strings);
    bytes
}
fn fingerprints(root: &Path) -> BTreeMap<String, (String, u64, u128)> {
    let mut map = BTreeMap::new();
    let mut queue = vec![root.to_path_buf()];
    while let Some(dir) = queue.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                queue.push(path);
            } else {
                let m = fs::metadata(&path).unwrap();
                map.insert(
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    (
                        hash_file(&path).unwrap(),
                        m.len(),
                        m.modified()
                            .unwrap()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos(),
                    ),
                );
            }
        }
    }
    map
}
fn adopted(f: &Fixture, e: &mut Engine, path: &str) -> String {
    let inventory = e.scan_installed().unwrap();
    let candidate = item(&inventory, path);
    assert!(candidate.adoptable, "{:?}", candidate);
    let p = e.prepare_adoption(&candidate.id).unwrap();
    assert!(p.blockers.is_empty(), "{:?}", p.blockers);
    let id = p.mod_id.clone();
    e.apply_with_ownership(&p.token, true, true, true).unwrap();
    assert!(f.root.join(path).exists());
    id
}
#[test]
fn automatic_selection_scans_existing_asi_scripts_and_disabled_suffixes() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"synthetic");
    put(&f, "scripts/Hello.cs", b"// test");
    put(&f, "scripts/Assembly.dll", b"synthetic");
    put(&f, "Old.asi.disabled", b"synthetic");
    let e = f.engine();
    let scan = e.inventory().unwrap().unwrap();
    assert_eq!(item(&scan, "Traffic.asi").status, "enabled");
    assert!(item(&scan, "Traffic.asi").adoptable);
    assert_eq!(item(&scan, "scripts/Hello.cs").category, "Scripts");
    assert!(item(&scan, "scripts/Hello.cs").adoptable);
    assert!(!item(&scan, "scripts/Assembly.dll").adoptable);
    assert_eq!(item(&scan, "Old.asi.disabled").status, "disabled");
    assert!(!item(&scan, "Old.asi.disabled").adoptable);
}
#[test]
fn detects_rage_lspdfr_openiv_dlc_and_graphics_loaders() {
    let f = Fixture::new();
    f.prereqs();
    for p in [
        "RAGEPluginHook.exe",
        "plugins/LSPD First Response.dll",
        "plugins/LSPDFR/Callout.dll",
        "lspdfr/data/agencies.xml",
        "OpenIV.asi",
        "dxgi.dll",
    ] {
        put(&f, p, b"synthetic");
    }
    put(
        &f,
        "mods/update/x64/dlcpacks/customcar/dlc.rpf",
        &rpf_fixture(&["adder.yft"], 0x4e45504f),
    );
    let e = f.engine();
    let s = e.inventory().unwrap().unwrap();
    assert_eq!(item(&s, "RAGEPluginHook.exe").name, "RAGE Plugin Hook");
    assert_eq!(item(&s, "plugins/LSPD First Response.dll").name, "LSPDFR");
    assert!(!item(&s, "plugins/LSPDFR/Callout.dll").adoptable);
    assert_eq!(item(&s, "mods").kind, "OpenIV folder");
    assert_eq!(
        item(&s, "mods/update/x64/dlcpacks/customcar").kind,
        "DLC package"
    );
    assert_eq!(item(&s, "dxgi.dll").category, "Graphics");
}
#[test]
fn rpf_resources_are_clues_with_multiple_categories_not_individual_mods() {
    let f = Fixture::new();
    put(
        &f,
        "mods/x64e.rpf",
        &rpf_fixture(
            &[
                "adder.yft",
                "w_pistol.ytd",
                "mp_m_test.ydd",
                "downtown.ymap",
            ],
            0,
        ),
    );
    let e = f.engine();
    let s = e.inventory().unwrap().unwrap();
    let container = item(&s, "mods/x64e.rpf");
    let toc = container.rpf.as_ref().unwrap();
    assert_eq!(toc.state, "Readable TOC");
    for c in ["Vehicles", "Weapons", "Characters", "Maps"] {
        assert_eq!(toc.categories[c], 1);
    }
    assert_eq!(toc.samples.len(), 4);
    assert_eq!(container.confidence, "possible");
    assert_eq!(container.status, "needs_review");
    assert!(!container.adoptable);
    assert!(!s.items.iter().any(|i| i.name == "adder"));
}
#[test]
fn unknown_files_original_archives_and_lua_do_not_claim_known_mods() {
    let f = Fixture::new();
    put(&f, "Mystery.bin", b"unknown");
    put(&f, "scripts/unknown.lua", b"--unknown loader");
    let e = f.engine();
    let s = e.inventory().unwrap().unwrap();
    assert_eq!(item(&s, "Mystery.bin").confidence, "unknown");
    let original = item(&s, "update/update.rpf");
    assert!(original.rpf.is_none());
    assert_eq!(original.status, "unknown");
    assert!(!item(&s, "scripts/unknown.lua")
        .dependencies
        .contains(&"shvdn".into()));
}
#[test]
fn scanner_does_not_change_any_game_content_or_modification_times() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"synthetic");
    put(&f, "mods/copy.rpf", &rpf_fixture(&["map.ymap"], 0));
    let mut e = f.engine();
    let before = fingerprints(&f.root);
    let scan = e.scan_installed().unwrap();
    assert!(scan.complete);
    assert_eq!(fingerprints(&f.root), before);
    assert_eq!(scan.hashed_bytes, 0);
}
#[test]
fn rpf_encryption_malformed_bounds_cycles_and_budget_are_safe() {
    let mut encrypted = rpf_fixture(&["a.yft"], 0x0ffffff9);
    let mut budget = 10000;
    let result = rpf::inspect(
        &mut Cursor::new(&encrypted),
        encrypted.len() as u64,
        &mut budget,
    );
    assert_eq!(result.state, "Encrypted");
    assert_eq!(result.bytes_read, 16);
    encrypted[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    let result = rpf::inspect(
        &mut Cursor::new(&encrypted),
        encrypted.len() as u64,
        &mut budget,
    );
    assert_eq!(result.state, "Unavailable");
    let mut cycle = rpf_fixture(&["a.yft"], 0);
    cycle[24..28].copy_from_slice(&0u32.to_le_bytes());
    let result = rpf::inspect(&mut Cursor::new(&cycle), cycle.len() as u64, &mut budget);
    assert_eq!(result.state, "Unavailable");
    assert!(result.samples.is_empty());
    let valid = rpf_fixture(&["a.yft"], 0);
    let mut budget = 16;
    let result = rpf::inspect(&mut Cursor::new(&valid), valid.len() as u64, &mut budget);
    assert_eq!(result.state, "Unavailable");
    assert_eq!(budget, 0);
    for length in 0..valid.len() {
        let mut budget = 10000;
        let _ = rpf::inspect(
            &mut Cursor::new(&valid[..length]),
            length as u64,
            &mut budget,
        );
    }
}
#[test]
fn large_rpf_scan_reads_only_bounded_tables_and_never_hashes_payloads() {
    let f = Fixture::new();
    let bytes = rpf_fixture(&["a.yft"], 0);
    put(&f, "mods/large.rpf", &bytes);
    File::options()
        .write(true)
        .open(f.root.join("mods/large.rpf"))
        .unwrap()
        .set_len(256 * 1024 * 1024)
        .unwrap();
    let e = f.engine();
    let scan = e.inventory().unwrap().unwrap();
    assert!(scan.bytes_read < 1024);
    assert_eq!(scan.hashed_bytes, 0);
    assert_eq!(
        item(&scan, "mods/large.rpf").rpf.as_ref().unwrap().state,
        "Readable TOC"
    );
}
#[test]
fn standalone_adoption_requires_all_confirmations_and_preserves_game_until_disable() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"original external ASI");
    let mut e = f.engine();
    let scan = e.inventory().unwrap().unwrap();
    let p = e.prepare_adoption(&item(&scan, "Traffic.asi").id).unwrap();
    let before = fingerprints(&f.root);
    assert!(e.apply(&p.token, true, true).is_err());
    assert!(e.apply_with_ownership(&p.token, false, true, true).is_err());
    let id = p.mod_id.clone();
    e.apply_with_ownership(&p.token, true, true, true).unwrap();
    assert_eq!(fingerprints(&f.root), before);
    assert!(e.ownership(&id).unwrap()[0].adopted);
    perform(&mut e, &id, "disable");
    assert!(!f.root.join("Traffic.asi").exists());
    perform(&mut e, &id, "enable");
    assert_eq!(
        fs::read(f.root.join("Traffic.asi")).unwrap(),
        b"original external ASI"
    );
}
#[test]
fn source_script_adoption_and_release_restore_the_external_original() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "scripts/Hello.cs", b"// external source");
    let mut e = f.engine();
    let id = adopted(&f, &mut e, "scripts/Hello.cs");
    perform(&mut e, &id, "disable");
    perform(&mut e, &id, "remove");
    assert_eq!(
        fs::read(f.root.join("scripts/Hello.cs")).unwrap(),
        b"// external source"
    );
    assert!(e.ownership(&id).unwrap().is_empty());
    assert!(!e.prepare(&id, "install").unwrap().blockers.is_empty());
}
#[test]
fn shared_prerequisites_companions_and_duplicate_stems_cannot_be_adopted() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    put(&f, "Traffic.ini", b"config");
    put(&f, "Foo.asi", b"mod");
    put(&f, "scripts/Foo.cs", b"source");
    let mut e = f.engine();
    let scan = e.inventory().unwrap().unwrap();
    for path in [
        "Traffic.asi",
        "Foo.asi",
        "scripts/Foo.cs",
        "dinput8.dll",
        "ScriptHookV.dll",
    ] {
        let found = item(&scan, path);
        assert!(!found.adoptable, "{path}");
        let p = e.prepare_adoption(&found.id).unwrap();
        assert!(!p.blockers.is_empty());
        assert!(e.apply_with_ownership(&p.token, true, true, true).is_err());
    }
}
#[test]
fn changed_scope_stale_approval_and_external_content_edits_are_blocked() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let mut e = f.engine();
    let inventory = e.inventory().unwrap().unwrap();
    let id = item(&inventory, "Traffic.asi").id.clone();
    let p = e.prepare_adoption(&id).unwrap();
    put(&f, "Traffic.ini", b"new companion");
    assert!(e.apply_with_ownership(&p.token, true, true, true).is_err());
    fs::remove_file(f.root.join("Traffic.ini")).unwrap();
    let mid = adopted(&f, &mut e, "Traffic.asi");
    put(&f, "Traffic.asi", b"external edit");
    assert!(!e.prepare(&mid, "disable").unwrap().blockers.is_empty());
    assert_eq!(
        fs::read(f.root.join("Traffic.asi")).unwrap(),
        b"external edit"
    );
}
#[test]
fn newly_shared_companions_block_enable_and_disable_after_adoption() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let mut e = f.engine();
    let id = adopted(&f, &mut e, "Traffic.asi");
    put(&f, "Traffic.ini", b"new config");
    assert!(!e.prepare(&id, "disable").unwrap().blockers.is_empty());
    fs::remove_file(f.root.join("Traffic.ini")).unwrap();
    perform(&mut e, &id, "disable");
    put(&f, "plugins/Traffic.dll", b"possible shared library");
    assert!(!e.prepare(&id, "enable").unwrap().blockers.is_empty());
}
#[test]
fn shared_hardlinked_entrypoint_cannot_be_adopted() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"shared data");
    fs::hard_link(
        f.root.join("Traffic.asi"),
        f.temp.path().join("external-link"),
    )
    .unwrap();
    let mut e = f.engine();
    let scan = e.inventory().unwrap().unwrap();
    let p = e.prepare_adoption(&item(&scan, "Traffic.asi").id).unwrap();
    assert!(p.blockers.iter().any(|s| s.contains("hard-linked")));
    assert!(e.apply_with_ownership(&p.token, true, true, true).is_err());
}
#[test]
fn managed_files_are_not_duplicated_as_external_and_source_is_retained() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let mut e = f.engine();
    let id = adopted(&f, &mut e, "Traffic.asi");
    let inventory = e.scan_installed().unwrap();
    assert_eq!(
        item(&inventory, "Traffic.asi").managed_id.as_ref(),
        Some(&id)
    );
    let managed = inventory.managed.iter().find(|m| m.id == id).unwrap();
    assert_eq!(managed.source, "Manual");
    assert!(managed.adopted);
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM external_mods", [], |r| r
            .get::<_, i64>(0))
            .unwrap() as usize,
        inventory.items.len()
    );
}
#[test]
fn adoption_restore_and_interrupted_disable_recovery_are_reversible() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let mut e = f.engine();
    let id = adopted(&f, &mut e, "Traffic.asi");
    let r = perform(&mut e, &id, "disable");
    let json: String =
        e.db.query_row("SELECT json FROM journals WHERE id=?", [&r.id], |r| {
            r.get(0)
        })
        .unwrap();
    let j: Journal = serde_json::from_str(&json).unwrap();
    e.save_record(&j.before_record).unwrap();
    e.db.execute("UPDATE journals SET status='pending' WHERE id=?", [&r.id])
        .unwrap();
    drop(e);
    let mut e = Engine::open(&f.data).unwrap();
    assert!(e.recovery.is_empty());
    assert_eq!(fs::read(f.root.join("Traffic.asi")).unwrap(), b"mod");
    assert_eq!(e.record(&id).unwrap().status, "enabled");
    let r = perform(&mut e, &id, "disable");
    let p = e.prepare_restore(&r.id).unwrap();
    e.apply(&p.token, true, true).unwrap();
    assert_eq!(fs::read(f.root.join("Traffic.asi")).unwrap(), b"mod");
}
#[test]
fn scan_inventory_persists_and_revision_marks_concurrent_results_stale() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let mut e = f.engine();
    let (ctx, owners, revision) = e.scan_context().unwrap();
    let scan = scanner::scan(&ctx, &owners).unwrap();
    e.revision += 1;
    assert!(e.persist_scan(scan, revision).unwrap().stale);
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    assert!(e
        .inventory()
        .unwrap()
        .unwrap()
        .items
        .iter()
        .any(|m| m.location == "Traffic.asi"));
}
#[test]
fn selected_game_change_rejects_an_old_scan_result() {
    let f = Fixture::new();
    let g = Fixture::new();
    let mut e = f.engine();
    let (ctx, owners, revision) = e.scan_context().unwrap();
    let scan = scanner::scan(&ctx, &owners).unwrap();
    e.select_game(&g.root, "Other fixture").unwrap();
    assert!(e.persist_scan(scan, revision).is_err());
}
#[test]
fn scanner_rejects_junction_escape_and_disables_adoption_when_partial() {
    let f = Fixture::new();
    f.prereqs();
    put(&f, "Traffic.asi", b"mod");
    let outside = f.temp.path().join("Outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("DoNotRead.cs"), b"outside").unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(f.root.join("scripts"))
            .arg(&outside)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, f.root.join("scripts")).unwrap();
    let e = f.engine();
    let s = e.inventory().unwrap().unwrap();
    assert!(!s.complete);
    assert_eq!(s.skipped, 1);
    assert!(!s.items.iter().any(|i| i.location.contains("DoNotRead")));
    assert!(!item(&s, "Traffic.asi").adoptable);
}
#[test]
fn scan_time_and_io_remain_bounded_for_a_large_synthetic_library() {
    let f = Fixture::new();
    for n in 0..2500 {
        put(&f, &format!("scripts/data/item{n}.json"), b"{}");
    }
    let e = f.engine();
    let scan = e.inventory().unwrap().unwrap();
    println!(
        "SCANNER BENCHMARK: {} files, {} entries, {} ms, {} content bytes read, {} bytes hashed",
        scan.visited_files,
        scan.visited_entries,
        scan.elapsed_ms,
        scan.bytes_read,
        scan.hashed_bytes
    );
    assert!(scan.complete);
    assert!(scan.visited_files >= 2500);
    assert_eq!(scan.hashed_bytes, 0);
    assert_eq!(scan.bytes_read, 0);
    assert!(scan.elapsed_ms < 10_000, "scan took {} ms", scan.elapsed_ms);
}
#[test]
fn additive_schema_upgrade_preserves_phase1_records_ownership_and_backups() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"phase1")).unwrap();
    perform(&mut e, &m.id, "install");
    let mut record = serde_json::to_value(e.record(&m.id).unwrap()).unwrap();
    for key in ["origin", "adopted_from", "game_path"] {
        record.as_object_mut().unwrap().remove(key);
    }
    e.db.execute(
        "UPDATE mods SET json=? WHERE id=?",
        params![record.to_string(), m.id],
    )
    .unwrap();
    e.db.execute_batch("DROP TABLE scan_runs;DROP TABLE external_mods;PRAGMA user_version=1;")
        .unwrap();
    drop(e);
    let mut e = Engine::open(&f.data).unwrap();
    assert_eq!(e.record(&m.id).unwrap().origin, "Zoxryn");
    assert_eq!(e.snapshot().unwrap().backups.len(), 1);
    perform(&mut e, &m.id, "disable");
    perform(&mut e, &m.id, "enable");
    assert_eq!(fs::read(f.root.join("Example.asi")).unwrap(), b"phase1");
}
