//! Non-installed UI test product. A compile-time, marker-protected sandbox only.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use zoxryn_core::safety::check_tree;
static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
pub fn start() {
    let _ = START.set(std::time::Instant::now());
}
pub fn record_page_load() -> Result<()> {
    let elapsed = START
        .get()
        .context("QA startup timing unavailable")?
        .elapsed()
        .as_secs_f64()
        * 1000.0;
    std::fs::write(
        root()?.join("native-page-load.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"native_main_to_page_load_ms":elapsed,"milestone":"Tauri WebView page finished loading; not a cold disk-cache or React data-ready claim","version":env!("CARGO_PKG_VERSION"),"qa_only":true}),
        )?,
    )?;
    Ok(())
}

pub fn root() -> Result<PathBuf> {
    let path =
        PathBuf::from(option_env!("ZOXRYN_UI_QA_ROOT").context("UI QA root was not compiled in")?);
    check_tree(&path)?;
    let path = std::fs::canonicalize(path)?;
    if std::fs::read(path.join("SYNTHETIC-UI-QA"))? != b"ZOXRYN SYNTHETIC UI QA ONLY" {
        bail!("UI QA marker mismatch");
    }
    Ok(path)
}
pub fn validate_path(path: &Path) -> Result<()> {
    check_tree(path)?;
    let absolute = std::fs::canonicalize(path)?;
    if !absolute.starts_with(root()?) {
        bail!("UI QA may only access its synthetic workspace");
    }
    Ok(())
}
fn validate_json(value: &serde_json::Value, key: &str) -> Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                validate_json(value, key)?;
            }
        }
        serde_json::Value::Array(list) => {
            for value in list {
                validate_json(value, key)?;
            }
        }
        serde_json::Value::String(path)
            if [
                "path",
                "game",
                "game_path",
                "archive_path",
                "original_blob",
                "installed_blob",
                "before_blob",
                "after_blob",
            ]
            .contains(&key) =>
        {
            if Path::new(path).is_absolute() {
                validate_path(Path::new(path))?;
            }
        }
        _ => (),
    }
    Ok(())
}
/// Validate before Engine::open can perform journal recovery.
pub fn data() -> Result<PathBuf> {
    let data = root()?.join("data");
    validate_path(&data)?;
    let db_path = data.join("zoxryn.sqlite3");
    validate_path(&db_path)?;
    let db =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    for table in ["mods", "owned", "journals", "scan_runs", "external_mods"] {
        let mut statement = db.prepare(&format!("SELECT json FROM {table}"))?;
        for row in statement.query_map([], |r| r.get::<_, String>(0))? {
            validate_json(&serde_json::from_str(&row?)?, "")?;
        }
    }
    let selected: Option<String> = db
        .query_row(
            "SELECT value FROM settings WHERE key='game_path'",
            [],
            |r| r.get(0),
        )
        .ok();
    if let Some(path) = selected {
        validate_path(Path::new(&path))?;
    }
    // All serialized absolute paths, including future settings, must be local.
    let mut statement = db.prepare("SELECT value FROM settings")?;
    for row in statement.query_map([], |r| r.get::<_, String>(0))? {
        let value = row?;
        if Path::new(&value).is_absolute() {
            validate_path(Path::new(&value))?;
        }
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&value) {
            validate_json(&json, "")?;
        }
    }
    Ok(data)
}
