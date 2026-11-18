use super::*;
use std::io::Write;
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
#[path = "phase15_tests.rs"]
mod phase15;
#[path = "updater_tests.rs"]
mod updater;

struct Fixture {
    temp: TempDir,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("zoxryn-synthetic-")
            .tempdir()
            .unwrap();
        let root = temp.path().join("Mock GTA V Legacy");
        let data = temp.path().join("ManagerData");
        fs::create_dir_all(root.join("update")).unwrap();
        fs::create_dir_all(root.join("x64")).unwrap();
        fs::write(root.join("GTA5.exe"), b"SYNTHETIC - NOT EXECUTABLE").unwrap();
        fs::write(
            root.join("update/update.rpf"),
            b"original RPF - must never change",
        )
        .unwrap();
        Self { temp, root, data }
    }
    fn engine(&self) -> Engine {
        let mut e = Engine::open(&self.data).unwrap();
        e.select_game(&self.root, "Synthetic fixture").unwrap();
        e
    }
    fn prereqs(&self) {
        for name in [
            "ScriptHookV.dll",
            "dinput8.dll",
            "ScriptHookVDotNet.asi",
            "ScriptHookVDotNet2.dll",
            "ScriptHookVDotNet3.dll",
        ] {
            fs::write(self.root.join(name), b"synthetic prerequisite").unwrap();
        }
    }
    fn zip(&self, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = self.temp.path().join(name);
        let mut writer = ZipWriter::new(File::create(&path).unwrap());
        for (entry, bytes) in files {
            writer
                .start_file(
                    *entry,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
        path
    }
    fn asi(&self, name: &str, content: &[u8]) -> PathBuf {
        self.zip(
            name,
            &[
                ("Example.asi", content),
                ("Example.ini", b"[settings]\nenabled=true"),
                ("readme.txt", b"Legacy ASI installation instructions"),
            ],
        )
    }
}
fn perform(e: &mut Engine, id: &str, operation: &str) -> Report {
    let p = e.prepare(id, operation).unwrap();
    assert!(p.blockers.is_empty(), "{:?}", p.blockers);
    e.apply(&p.token, true, true).unwrap()
}

#[test]
fn recognizes_legacy_without_inventing_a_build() {
    let f = Fixture::new();
    let g = game::inspect(&f.root, "Test").unwrap();
    assert_eq!(g.edition, "Legacy");
    assert!(g.version.is_none());
}
#[test]
fn enhanced_and_ambiguous_folders_are_distinct_and_blocked() {
    let f = Fixture::new();
    fs::rename(f.root.join("GTA5.exe"), f.root.join("GTA5_Enhanced.exe")).unwrap();
    assert_eq!(game::inspect(&f.root, "Test").unwrap().edition, "Enhanced");
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"test")).unwrap();
    assert!(e
        .prepare(&m.id, "install")
        .unwrap()
        .blockers
        .iter()
        .any(|s| s.contains("Legacy")));
    fs::write(f.root.join("GTA5.exe"), b"mock").unwrap();
    assert_eq!(game::inspect(&f.root, "Test").unwrap().edition, "Ambiguous");
}
#[test]
fn rejects_incomplete_game_folder() {
    let f = Fixture::new();
    fs::remove_file(f.root.join("update/update.rpf")).unwrap();
    assert!(game::inspect(&f.root, "Test").is_err());
}
#[test]
fn detects_steam_custom_library_manifest() {
    let f = Fixture::new();
    let steam = f.temp.path().join("Steam");
    let library = f.temp.path().join("Library");
    fs::create_dir_all(steam.join("steamapps")).unwrap();
    fs::create_dir_all(library.join("steamapps")).unwrap();
    fs::write(
        steam.join("steamapps/libraryfolders.vdf"),
        format!(
            "\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} }}",
            library.to_string_lossy().replace('\\', "\\\\")
        ),
    )
    .unwrap();
    fs::write(
        library.join("steamapps/appmanifest_271590.acf"),
        "\"AppState\" { \"installdir\" \"Grand Theft Auto V\" }",
    )
    .unwrap();
    assert!(game::steam_candidates(&steam)
        .contains(&library.join("steamapps/common/Grand Theft Auto V")));
}
#[test]
fn zip_import_inspects_without_writing_to_game_and_retains_archive() {
    let f = Fixture::new();
    let mut e = f.engine();
    let p = f.asi("my.zip", b"fake asi");
    let m = e.import(&p).unwrap();
    assert_eq!(m.kind, "ASI");
    assert_eq!(m.files.len(), 2);
    assert!(!f.root.join("Example.asi").exists());
    assert!(Path::new(&m.archive_path).is_file());
    assert_eq!(m.requirements, vec!["asi", "shv"]);
    assert_eq!(e.import(&p).unwrap().id, m.id);
}
#[test]
fn missing_dependencies_block_installation() {
    let f = Fixture::new();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let p = e.prepare(&m.id, "install").unwrap();
    assert!(p.blockers.iter().any(|s| s.contains("Script Hook V")));
    assert!(e.apply(&p.token, true, true).is_err());
    assert!(!f.root.join("Example.asi").exists());
}
#[test]
fn install_disable_enable_uninstall_restores_original_and_keeps_archive() {
    let f = Fixture::new();
    f.prereqs();
    fs::write(f.root.join("Example.ini"), b"original configuration").unwrap();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let report = perform(&mut e, &m.id, "install");
    assert_eq!(report.files.len(), 2);
    assert_eq!(fs::read(f.root.join("Example.asi")).unwrap(), b"mod");
    perform(&mut e, &m.id, "disable");
    assert!(!f.root.join("Example.asi").exists());
    assert_eq!(
        fs::read(f.root.join("Example.ini")).unwrap(),
        b"original configuration"
    );
    perform(&mut e, &m.id, "enable");
    assert_eq!(fs::read(f.root.join("Example.asi")).unwrap(), b"mod");
    perform(&mut e, &m.id, "remove");
    assert!(!f.root.join("Example.asi").exists());
    assert_eq!(
        fs::read(f.root.join("Example.ini")).unwrap(),
        b"original configuration"
    );
    assert!(Path::new(&m.archive_path).exists());
    assert_eq!(e.record(&m.id).unwrap().status, "imported");
    assert!(e.ownership(&m.id).unwrap().is_empty());
    assert_eq!(
        fs::read(f.root.join("update/update.rpf")).unwrap(),
        b"original RPF - must never change"
    );
    assert_eq!(
        fs::read(f.root.join("ScriptHookV.dll")).unwrap(),
        b"synthetic prerequisite"
    );
}
#[test]
fn explicit_scripts_folder_is_supported() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let p = f.zip(
        "script.zip",
        &[
            ("Package/scripts/Drive.cs", b"// synthetic script"),
            ("Package/readme.txt", b"Legacy instructions"),
        ],
    );
    let m = e.import(&p).unwrap();
    assert_eq!(m.kind, "Script");
    assert!(m.requirements.contains(&"shvdn".into()));
    perform(&mut e, &m.id, "install");
    assert!(f.root.join("scripts/Drive.cs").is_file());
    perform(&mut e, &m.id, "remove");
    assert!(!f.root.join("scripts/Drive.cs").exists());
}
#[test]
fn owned_file_conflicts_are_blocked_even_when_disabled() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let a = e.import(&f.asi("a.zip", b"one")).unwrap();
    let b = e.import(&f.asi("b.zip", b"two")).unwrap();
    perform(&mut e, &a.id, "install");
    assert!(e
        .prepare(&b.id, "install")
        .unwrap()
        .blockers
        .iter()
        .any(|s| s.contains("owned")));
    perform(&mut e, &a.id, "disable");
    assert!(e
        .prepare(&b.id, "install")
        .unwrap()
        .blockers
        .iter()
        .any(|s| s.contains("owned")));
}
#[test]
fn external_modifications_block_disable_uninstall_and_restore() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let r = perform(&mut e, &m.id, "install");
    fs::write(f.root.join("Example.asi"), b"external edits").unwrap();
    for op in ["disable", "remove"] {
        assert!(e
            .prepare(&m.id, op)
            .unwrap()
            .blockers
            .iter()
            .any(|s| s.contains("External modification")));
    }
    assert!(e
        .prepare_restore(&r.id)
        .unwrap()
        .blockers
        .iter()
        .any(|s| s.contains("External modification")));
    assert_eq!(
        fs::read(f.root.join("Example.asi")).unwrap(),
        b"external edits"
    );
}
#[test]
fn approval_is_required_and_stale_plans_cannot_write() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let p = e.prepare(&m.id, "install").unwrap();
    assert!(e.apply(&p.token, false, true).is_err());
    assert!(e.apply(&p.token, true, false).is_err());
    fs::write(f.root.join("Example.asi"), b"external file").unwrap();
    assert!(e.apply(&p.token, true, true).is_err());
    assert_eq!(
        fs::read(f.root.join("Example.asi")).unwrap(),
        b"external file"
    );
}
#[test]
fn backup_restore_reverses_install_and_can_itself_be_reversed() {
    let f = Fixture::new();
    f.prereqs();
    fs::write(f.root.join("Example.ini"), b"original").unwrap();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let r = perform(&mut e, &m.id, "install");
    let p = e.prepare_restore(&r.id).unwrap();
    assert!(p.blockers.is_empty(), "{:?}", p.blockers);
    let undo = e.apply(&p.token, true, true).unwrap();
    assert!(!f.root.join("Example.asi").exists());
    assert_eq!(fs::read(f.root.join("Example.ini")).unwrap(), b"original");
    assert_eq!(e.record(&m.id).unwrap().status, "imported");
    let p = e.prepare_restore(&undo.id).unwrap();
    e.apply(&p.token, true, true).unwrap();
    assert_eq!(fs::read(f.root.join("Example.asi")).unwrap(), b"mod");
    assert_eq!(e.record(&m.id).unwrap().status, "enabled");
}
#[test]
fn older_backup_cannot_overwrite_newer_mod_state() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let r = perform(&mut e, &m.id, "install");
    perform(&mut e, &m.id, "disable");
    assert!(!e.prepare_restore(&r.id).unwrap().blockers.is_empty());
}
#[test]
fn recovery_rolls_back_interrupted_multi_file_operation() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let r = perform(&mut e, &m.id, "install");
    // Turn a completed fixture journal into the exact state of a crash after filesystem writes but before DB commit.
    let json: String =
        e.db.query_row("SELECT json FROM journals WHERE id=?", [&r.id], |r| {
            r.get(0)
        })
        .unwrap();
    let j: Journal = serde_json::from_str(&json).unwrap();
    e.save_record(&j.before_record).unwrap();
    e.db.execute("DELETE FROM owned WHERE mod_id=?", [&m.id])
        .unwrap();
    e.db.execute("UPDATE journals SET status='pending' WHERE id=?", [&r.id])
        .unwrap();
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    assert!(e.recovery.is_empty());
    assert!(!f.root.join("Example.asi").exists());
    assert!(!f.root.join("Example.ini").exists());
    assert_eq!(e.record(&m.id).unwrap().status, "imported");
    assert_eq!(e.snapshot().unwrap().backups[0].status, "recovered");
}
#[test]
fn recovery_preserves_external_edits_and_blocks_further_writes() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let r = perform(&mut e, &m.id, "install");
    e.db.execute("UPDATE journals SET status='pending' WHERE id=?", [&r.id])
        .unwrap();
    fs::write(f.root.join("Example.asi"), b"external after crash").unwrap();
    drop(e);
    let mut e = Engine::open(&f.data).unwrap();
    assert!(!e.recovery.is_empty());
    assert_eq!(
        fs::read(f.root.join("Example.asi")).unwrap(),
        b"external after crash"
    );
    assert!(e.select_game(&f.root, "Test").is_err());
}
#[test]
fn malicious_archive_paths_are_rejected_before_import_copy() {
    let f = Fixture::new();
    let mut e = f.engine();
    for (i, path) in [
        "../outside.asi",
        "scripts/../../evil.dll",
        "C:/evil.asi",
        "/evil.asi",
        "scripts\\..\\escape.asi",
        "scripts/file.dll:stream",
        "scripts/CON.dll",
        "scripts/trailing./evil.dll",
    ]
    .iter()
    .enumerate()
    {
        let archive = f.zip(&format!("evil{i}.zip"), &[(path, b"bad")]);
        assert!(e.import(&archive).is_err(), "{path}");
    }
    assert!(e.records().unwrap().is_empty());
    assert_eq!(fs::read_dir(e.data.join("archives")).unwrap().count(), 0);
}
#[test]
fn case_colliding_entries_are_rejected() {
    let f = Fixture::new();
    let mut e = f.engine();
    assert!(e
        .import(&f.zip(
            "evil.zip",
            &[("Example.asi", b"one"), ("EXAMPLE.ASI", b"two")]
        ))
        .is_err());
}
#[test]
fn archive_symlinks_are_rejected() {
    let f = Fixture::new();
    let p = f.temp.path().join("link.zip");
    let mut w = ZipWriter::new(File::create(&p).unwrap());
    w.add_symlink("link.asi", "../outside", SimpleFileOptions::default())
        .unwrap();
    w.finish().unwrap();
    let mut e = f.engine();
    assert!(e.import(&p).is_err());
}
#[test]
fn junction_escape_is_rejected() {
    let f = Fixture::new();
    let outside = f.temp.path().join("Outside");
    fs::create_dir_all(&outside).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(f.root.join("scripts"))
            .arg(&outside)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(result.status.success());
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, f.root.join("scripts")).unwrap();
    assert!(target(&f.root, "scripts/evil.dll").is_err());
    assert!(!outside.join("evil.dll").exists());
}
#[test]
fn oiv_dlc_and_ambiguous_formats_are_manual_only() {
    let f = Fixture::new();
    let mut e = f.engine();
    for (name, entry) in [
        ("package.oiv", "assembly.xml"),
        ("vehicle.zip", "mods/update/x64/dlcpacks/car/dlc.rpf"),
        ("unknown.zip", "Vehicle.yft"),
        ("installer.zip", "setup.exe"),
        ("police.zip", "plugins/LSPDFR/Callout.dll"),
        ("loose.zip", "Unmapped.dll"),
    ] {
        let m = e.import(&f.zip(name, &[(entry, b"synthetic")])).unwrap();
        assert!(m.files.is_empty(), "{name}");
        assert!(m.instructions.contains("manual") || m.instructions.contains("Manual"));
    }
    for ext in ["rar", "7z"] {
        let p = f.temp.path().join(format!("manual.{ext}"));
        fs::write(&p, b"unsupported format synthetic").unwrap();
        let m = e.import(&p).unwrap();
        assert!(m.files.is_empty());
        assert!(m.warnings.iter().any(|s| s.contains("not been inspected")));
    }
    let p = f.temp.path().join("setup.exe");
    fs::write(&p, b"do not execute").unwrap();
    assert!(e.import(&p).is_err());
}
#[test]
fn original_rpf_and_executable_paths_are_never_mapped() {
    let f = Fixture::new();
    let mut e = f.engine();
    let m = e
        .import(&f.zip(
            "danger.zip",
            &[
                ("update/update.rpf", b"bad"),
                ("GTA5.exe", b"bad"),
                ("Example.asi", b"mod"),
            ],
        ))
        .unwrap();
    assert!(m.files.is_empty());
}
#[test]
fn metadata_edition_and_supported_builds_are_enforced() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    for (name, json) in [
        ("enhanced.zip", r#"{"name":"Test","edition":"Enhanced"}"#),
        (
            "build.zip",
            r#"{"name":"Test","edition":"Legacy","supported_builds":["1.0.1.0"]}"#,
        ),
    ] {
        let m = e
            .import(&f.zip(
                name,
                &[("Example.asi", b"mod"), ("zoxryn.json", json.as_bytes())],
            ))
            .unwrap();
        assert!(!e.prepare(&m.id, "install").unwrap().blockers.is_empty());
    }
}
#[test]
fn library_and_game_settings_persist_across_restart() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    perform(&mut e, &m.id, "install");
    e.set_setting("show_tips", "false").unwrap();
    let path = e.selected_game().unwrap().unwrap().path;
    drop(e);
    let e = Engine::open(&f.data).unwrap();
    assert_eq!(e.record(&m.id).unwrap().status, "enabled");
    assert_eq!(e.selected_game().unwrap().unwrap().path, path);
    assert_eq!(e.json_setting("show_tips").unwrap(), Some("false".into()));
    assert_eq!(e.snapshot().unwrap().backups.len(), 1);
}
#[test]
fn second_manager_instance_cannot_open_same_database() {
    let f = Fixture::new();
    let _e = f.engine();
    assert!(Engine::open(&f.data).is_err());
}
#[test]
fn changed_cached_archive_is_never_installed() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let p = e.prepare(&m.id, "install").unwrap();
    fs::write(&m.archive_path, b"tampered archive").unwrap();
    assert!(e.apply(&p.token, true, true).is_err());
    assert!(!f.root.join("Example.asi").exists());
}
#[test]
fn changed_backup_blob_cannot_overwrite_game_file() {
    let f = Fixture::new();
    f.prereqs();
    fs::write(f.root.join("Example.ini"), b"original").unwrap();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    perform(&mut e, &m.id, "install");
    let owned = e.ownership(&m.id).unwrap();
    let original = owned.iter().find_map(|o| o.original_blob.as_ref()).unwrap();
    let (context, owners, scan_revision) = e.scan_context().unwrap();
    let in_flight = crate::scanner::scan(&context, &owners).unwrap();
    fs::write(e.data.join(original), b"tampered backup").unwrap();
    let p = e.prepare(&m.id, "disable").unwrap();
    assert!(e.apply(&p.token, true, true).is_err());
    assert_eq!(fs::read(f.root.join("Example.asi")).unwrap(), b"mod");
    assert_eq!(e.record(&m.id).unwrap().status, "enabled");
    assert!(e.persist_scan(in_flight, scan_revision).unwrap().stale);
}
#[test]
fn switching_game_invalidates_plan_and_prevents_cross_game_uninstall() {
    let f = Fixture::new();
    let g = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    perform(&mut e, &m.id, "install");
    e.select_game(&g.root, "Another fixture").unwrap();
    assert!(e
        .prepare(&m.id, "remove")
        .unwrap()
        .blockers
        .iter()
        .any(|s| s.contains("another game directory")));
    assert!(f.root.join("Example.asi").exists());
}
#[test]
fn tracked_clean_state_restores_multiple_mods_and_keeps_untracked_files() {
    let f = Fixture::new();
    f.prereqs();
    fs::write(f.root.join("Example.ini"), b"original").unwrap();
    fs::write(f.root.join("untracked.asi"), b"untracked").unwrap();
    let mut e = f.engine();
    let a = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let b = e
        .import(&f.zip("second.zip", &[("scripts/Second.cs", b"// fixture")]))
        .unwrap();
    perform(&mut e, &a.id, "install");
    perform(&mut e, &b.id, "install");
    perform(&mut e, &b.id, "disable");
    let p = e.prepare_clean().unwrap();
    assert_eq!(p.files.len(), 3);
    e.apply(&p.token, true, true).unwrap();
    assert_eq!(fs::read(f.root.join("Example.ini")).unwrap(), b"original");
    assert_eq!(
        fs::read(f.root.join("untracked.asi")).unwrap(),
        b"untracked"
    );
    assert!(!f.root.join("Example.asi").exists());
    assert!(e.records().unwrap().iter().all(|m| m.status == "imported"));
}
#[test]
fn tracked_clean_state_preflights_all_mods_before_writing() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let a = e.import(&f.asi("one.zip", b"mod")).unwrap();
    let b = e
        .import(&f.zip("second.zip", &[("scripts/Second.cs", b"// fixture")]))
        .unwrap();
    perform(&mut e, &a.id, "install");
    perform(&mut e, &b.id, "install");
    let p = e.prepare_clean().unwrap();
    fs::write(f.root.join("Example.asi"), b"external edit").unwrap();
    assert!(e.apply(&p.token, true, true).is_err());
    assert!(f.root.join("scripts/Second.cs").exists());
    assert!(e.records().unwrap().iter().all(|m| m.status == "enabled"));
}
#[test]
fn hardlinked_original_is_replaced_without_changing_external_link() {
    let f = Fixture::new();
    f.prereqs();
    let outside = f.temp.path().join("external-config.ini");
    fs::write(&outside, b"original").unwrap();
    fs::hard_link(&outside, f.root.join("Example.ini")).unwrap();
    let mut e = f.engine();
    let m = e.import(&f.asi("one.zip", b"mod")).unwrap();
    perform(&mut e, &m.id, "install");
    assert_eq!(fs::read(&outside).unwrap(), b"original");
    perform(&mut e, &m.id, "remove");
    assert_eq!(fs::read(f.root.join("Example.ini")).unwrap(), b"original");
}
#[test]
fn inconsistent_zip_headers_are_rejected() {
    let f = Fixture::new();
    let p = f.asi("headers.zip", b"mod");
    let mut bytes = fs::read(&p).unwrap();
    let name = b"Example.asi";
    let pos = bytes.windows(name.len()).position(|b| b == name).unwrap();
    bytes[pos..pos + name.len()].copy_from_slice(b"Escape_.asi");
    fs::write(&p, bytes).unwrap();
    let mut e = f.engine();
    assert!(e.import(&p).is_err());
}

#[test]
#[cfg(windows)]
fn directory_guards_prevent_ancestor_replacement_during_writes() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("scripts/nested")).unwrap();
    let guard = guard_tree(&f.root.join("scripts/nested"), false).unwrap();
    assert!(fs::rename(f.root.join("scripts"), f.root.join("moved-scripts")).is_err());
    fs::write(
        f.root.join("scripts/nested/safe.txt"),
        b"allowed child write",
    )
    .unwrap();
    drop(guard);
    fs::rename(f.root.join("scripts"), f.root.join("moved-scripts")).unwrap();
}
