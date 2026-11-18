use super::*;

#[test]
fn update_checkpoint_preserves_settings_records_archives_and_backups() {
    let f = Fixture::new();
    f.prereqs();
    let mut e = f.engine();
    let m = e
        .import(&f.asi("update-data.zip", b"synthetic mod"))
        .unwrap();
    perform(&mut e, &m.id, "install");
    e.scan_installed().unwrap();
    assert!(
        e.db.query_row("SELECT count(*) FROM external_mods", [], |r| r
            .get::<_, i64>(0))
            .unwrap()
            > 0
    );
    assert!(
        e.db.query_row("SELECT count(*) FROM scan_runs", [], |r| r.get::<_, i64>(0))
            .unwrap()
            > 0
    );
    e.set_setting("show_tips", "false").unwrap();
    let before = e.snapshot().unwrap();
    let original = fs::read(f.root.join("update/update.rpf")).unwrap();
    let archive = f
        .data
        .join("archives")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let archive_bytes = fs::read(&archive).unwrap();
    let install = f.temp.path().join("Installed App");
    fs::create_dir_all(&install).unwrap();
    let backup = e.checkpoint_for_app_update(&install).unwrap();
    let copy = Connection::open(backup).unwrap();
    assert_eq!(
        copy.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    for table in [
        "settings",
        "mods",
        "owned",
        "journals",
        "scan_runs",
        "external_mods",
    ] {
        let sql = format!("SELECT * FROM {table} ORDER BY 1");
        let rows = |db: &Connection| {
            let mut stmt = db.prepare(&sql).unwrap();
            let count = stmt.column_count();
            let values = stmt
                .query_map([], |r| {
                    Ok((0..count)
                        .map(|i| r.get::<_, rusqlite::types::Value>(i).unwrap())
                        .collect::<Vec<_>>())
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            values
        };
        assert_eq!(rows(&e.db), rows(&copy), "{table}");
    }
    assert_eq!(e.snapshot().unwrap().mods.len(), before.mods.len());
    assert_eq!(e.snapshot().unwrap().backups.len(), before.backups.len());
    assert_eq!(
        fs::read(f.root.join("update/update.rpf")).unwrap(),
        original
    );
    assert_eq!(fs::read(archive).unwrap(), archive_bytes);
}

#[test]
fn update_checkpoint_rejects_game_or_data_overlap_and_unfinished_journals() {
    let f = Fixture::new();
    let e = f.engine();
    assert!(e.checkpoint_for_app_update(&f.root).is_err());
    assert!(e.checkpoint_for_app_update(&f.data).is_err());
    let install = f.temp.path().join("Installed App");
    fs::create_dir_all(&install).unwrap();
    e.db.execute(
        "INSERT INTO journals VALUES ('unfinished','pending','{}')",
        [],
    )
    .unwrap();
    assert!(e.checkpoint_for_app_update(&install).is_err());
    assert!(!f.data.join("app-update-backups").exists());
}

#[test]
fn update_checkpoint_does_not_probe_the_saved_game() {
    let f = Fixture::new();
    let e = f.engine();
    // The saved game can be offline or disconnected. App updates must not open it.
    fs::rename(&f.root, f.temp.path().join("Offline Game")).unwrap();
    let install = f.temp.path().join("Installed App");
    fs::create_dir_all(&install).unwrap();
    assert!(e.checkpoint_for_app_update(&install).unwrap().is_file());
}
