use crate::{archive, game, model::*, safety::*};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Owned {
    game: String,
    target: String,
    mod_id: String,
    installed_hash: String,
    original_hash: Option<String>,
    original_blob: Option<String>,
    installed_blob: String,
    #[serde(default)]
    adopted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Change {
    target: String,
    before_hash: Option<String>,
    after_hash: Option<String>,
    before_blob: Option<String>,
    after_blob: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Journal {
    id: String,
    game: String,
    mod_name: String,
    operation: String,
    created_at: u64,
    before_record: ModRecord,
    after_record: ModRecord,
    before_owned: Vec<Owned>,
    after_owned: Vec<Owned>,
    changes: Vec<Change>,
}
#[derive(Clone)]
struct Pending {
    public: Plan,
    record: ModRecord,
    restore: Option<Journal>,
    adoption: Option<DiscoveredMod>,
    suit_id: Option<String>,
}

pub struct Engine {
    db: Connection,
    data: PathBuf,
    _lock: File,
    pending: HashMap<String, Pending>,
    batches: HashMap<String, Vec<Plan>>,
    recovery: Vec<String>,
    revision: u64,
}

impl Engine {
    pub fn open(data: &Path) -> Result<Self> {
        check_tree(data)?;
        fs::create_dir_all(data)?;
        let data = fs::canonicalize(data)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(data.join("manager.lock"))?;
        lock.try_lock_exclusive()
            .context("Zoxryn Mods is already running with this data folder")?;
        let db = Connection::open(data.join("zoxryn.sqlite3"))?;
        let schema: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if schema > 3 {
            bail!("This database was created by a newer Zoxryn Mods version. Downgrading is blocked to preserve records.");
        }
        if schema < 3 && db.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get::<_,i64>(0))?>0 {
            let dir = data.join("schema-migrations");
            let _guard = guard_tree(&dir, true)?;
            let backup = dir.join(format!(
                "schema-{schema}-to-3-{}.sqlite3",
                uuid::Uuid::new_v4()
            ));
            db.execute("VACUUM INTO ?", [backup.to_string_lossy().as_ref()])?;
            File::options().write(true).open(&backup)?.sync_all()?;
        }
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS mods(id TEXT PRIMARY KEY, json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS owned(game TEXT COLLATE NOCASE, target TEXT COLLATE NOCASE, mod_id TEXT NOT NULL, json TEXT NOT NULL, PRIMARY KEY(game,target)); CREATE TABLE IF NOT EXISTS journals(id TEXT PRIMARY KEY, status TEXT NOT NULL, json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS scan_runs(game TEXT COLLATE NOCASE PRIMARY KEY,json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS external_mods(game TEXT COLLATE NOCASE,id TEXT,json TEXT NOT NULL,PRIMARY KEY(game,id)); CREATE TABLE IF NOT EXISTS collections(id TEXT PRIMARY KEY,json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS collection_members(collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,kind TEXT NOT NULL,item_id TEXT NOT NULL,PRIMARY KEY(collection_id,kind,item_id)); CREATE TABLE IF NOT EXISTS visual_metadata(kind TEXT NOT NULL,item_id TEXT NOT NULL,json TEXT NOT NULL,PRIMARY KEY(kind,item_id)); PRAGMA user_version=3; COMMIT;")?;
        for dir in ["archives", "blobs", "journals"] {
            fs::create_dir_all(data.join(dir))?;
        }
        let mut engine = Self {
            db,
            data,
            _lock: lock,
            pending: HashMap::new(),
            batches: HashMap::new(),
            recovery: Vec::new(),
            revision: 0,
        };
        engine.recover()?;
        Ok(engine)
    }
    pub fn data_path(&self) -> &Path {
        &self.data
    }
    /// Application update preparation never probes or changes the game folder.
    pub fn checkpoint_for_app_update(&self, install_directory: &Path) -> Result<PathBuf> {
        if !self.recovery.is_empty() {
            bail!("Resolve interrupted mod recovery before updating the application");
        }
        let pending: i64 = self.db.query_row(
            "SELECT count(*) FROM journals WHERE status='pending'",
            [],
            |r| r.get(0),
        )?;
        if pending != 0 {
            bail!("An unfinished file transaction must recover before the app can update");
        }
        check_tree(install_directory)?;
        let install = fs::canonicalize(install_directory)?;
        let overlaps = |a: &Path, b: &Path| {
            let a = a
                .to_string_lossy()
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase();
            let b = b
                .to_string_lossy()
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase();
            a == b || a.starts_with(&(b.clone() + "\\")) || b.starts_with(&(a + "\\"))
        };
        if overlaps(&install, &self.data) {
            bail!("Application installation and user data directories must be separate");
        }
        if let Some(saved) = self.json_setting("game")? {
            let game: Game = serde_json::from_str(&saved)?;
            if overlaps(&install, Path::new(&game.path)) {
                bail!("An application update must never target a game directory");
            }
        }
        let integrity: String = self.db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            bail!("Database integrity check failed; use manual recovery before updating");
        }
        self.db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        let folder = self.data.join("app-update-backups");
        let _guard = guard_tree(&folder, true)?;
        let backup = folder.join(format!(
            "schema-3-before-update-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        self.db
            .execute("VACUUM INTO ?", [backup.to_string_lossy().as_ref()])?;
        File::options().write(true).open(&backup)?.sync_all()?;
        Ok(backup)
    }
    fn json_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        if !["game", "show_tips"].contains(&key) {
            bail!("Unknown setting");
        }
        self.db.execute(
            "INSERT OR REPLACE INTO settings VALUES (?,?)",
            params![key, value],
        )?;
        Ok(())
    }
    pub fn select_game(&mut self, path: &Path, launcher: &str) -> Result<Game> {
        if !self.recovery.is_empty() {
            bail!("Resolve recovery issues before changing game folders");
        }
        let game = game::inspect(path, launcher)?;
        if self.data.starts_with(Path::new(&game.path))
            || Path::new(&game.path).starts_with(&self.data)
        {
            bail!("Game folder and manager data folder must be separate");
        }
        self.set_setting("game", &serde_json::to_string(&game)?)?;
        self.pending.clear();
        self.batches.clear();
        self.revision += 1;
        if game.edition == "Legacy" {
            self.scan_installed()?;
        }
        Ok(game)
    }
    pub fn selected_game(&self) -> Result<Option<Game>> {
        self.json_setting("game")?
            .map(|json| -> Result<Game> {
                let saved: Game = serde_json::from_str(&json)?;
                game::inspect(Path::new(&saved.path), &saved.launcher)
            })
            .transpose()
    }
    pub fn records(&self) -> Result<Vec<ModRecord>> {
        let mut stmt = self
            .db
            .prepare("SELECT json FROM mods ORDER BY rowid DESC")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    fn record(&self, id: &str) -> Result<ModRecord> {
        Ok(serde_json::from_str(&self.db.query_row(
            "SELECT json FROM mods WHERE id=?",
            [id],
            |r| r.get::<_, String>(0),
        )?)?)
    }
    fn save_record(&self, record: &ModRecord) -> Result<()> {
        self.db.execute(
            "INSERT OR REPLACE INTO mods VALUES (?,?)",
            params![record.id, serde_json::to_string(record)?],
        )?;
        Ok(())
    }
    fn ownership(&self, mod_id: &str) -> Result<Vec<Owned>> {
        let mut stmt = self
            .db
            .prepare("SELECT json FROM owned WHERE mod_id=? ORDER BY target")?;
        let rows = stmt.query_map([mod_id], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    fn owner(&self, game: &str, path: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .query_row(
                "SELECT mod_id FROM owned WHERE game=? AND target=?",
                params![game, path],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn import(&mut self, path: &Path) -> Result<ModRecord> {
        let _data_guard = guard_tree(&self.data.join("archives"), false)?;
        let mut record = archive::inspect(path)?;
        if let Some(existing) = self
            .records()?
            .into_iter()
            .find(|r| r.archive_hash == record.archive_hash)
        {
            return Ok(existing);
        }
        let ext = path.extension().unwrap().to_string_lossy().to_lowercase();
        let dest = self
            .data
            .join("archives")
            .join(format!("{}.{}", record.id, ext));
        copy_verified(path, &dest, &record.archive_hash)?;
        record.archive_path = dest.to_string_lossy().into();
        if let Err(error) = self.import_package_previews(&record) {
            record
                .warnings
                .push(format!("Preview unavailable: {error}"));
        }
        self.save_record(&record)?;
        Ok(record)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut stmt = self
            .db
            .prepare("SELECT json,status FROM journals ORDER BY rowid DESC")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut backups = Vec::new();
        for row in rows {
            let (json, status) = row?;
            let j: Journal = serde_json::from_str(&json)?;
            let bytes = j
                .changes
                .iter()
                .filter_map(|c| c.before_blob.as_ref())
                .filter_map(|p| fs::metadata(self.data.join(p)).ok())
                .map(|m| m.len())
                .sum();
            backups.push(Backup {
                id: j.id,
                mod_name: j.mod_name,
                operation: j.operation,
                created_at: j.created_at,
                status,
                files: j.changes.len(),
                bytes,
            });
        }
        let (selected, mut recovery) = (self.selected_game(), self.recovery.clone());
        let game = match selected {
            Ok(g) => g,
            Err(e) => {
                recovery.push(format!("Saved game folder is unavailable: {e}"));
                None
            }
        };
        let directory_bytes = |dir: &str| -> u64 {
            fs::read_dir(self.data.join(dir))
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| entry.metadata().ok())
                .filter(|meta| meta.is_file())
                .map(|meta| meta.len())
                .sum()
        };
        let archive_bytes = directory_bytes("archives");
        let backup_bytes = directory_bytes("blobs");
        let database_bytes: u64 = ["zoxryn.sqlite3", "zoxryn.sqlite3-wal", "zoxryn.sqlite3-shm"]
            .iter()
            .filter_map(|name| fs::metadata(self.data.join(name)).ok())
            .map(|m| m.len())
            .sum();
        Ok(Snapshot {
            game,
            mods: self.records()?,
            backups,
            recovery,
            data_path: self.data.to_string_lossy().into(),
            storage_bytes: archive_bytes
                + backup_bytes
                + database_bytes
                + directory_bytes("thumbnails"),
            archive_bytes,
            backup_bytes,
            inventory: self.inventory()?,
        })
    }
    pub fn prepare(&mut self, id: &str, operation: &str) -> Result<Plan> {
        self.batches.clear();
        let record = self.record(id)?;
        let game = self
            .selected_game()?
            .context("Select a GTA V installation first")?;
        let mut plan = Plan {
            token: uuid::Uuid::new_v4().to_string(),
            mod_id: id.into(),
            operation: operation.into(),
            independent_scope: None,
            game: game.clone(),
            files: Vec::new(),
            blockers: self.recovery.clone(),
            warnings: record.warnings.clone(),
        };
        if record.adopted_from.is_some() && ["enable", "disable"].contains(&operation) {
            plan.blockers
                .extend(self.adopted_scope_conflicts(&game, &record)?);
        }
        if record.adopted_from.is_some() && operation == "remove" {
            plan.warnings.push("Release of control restores this external entrypoint to its adoption-time bytes and relinquishes ownership. It does not uninstall the externally installed mod; retained backups remain available.".into());
        }
        if game.edition != "Legacy" {
            plan.blockers.push("Only GTA V Legacy is supported".into());
        }
        if game::game_running() {
            plan.blockers
                .push("Close GTA V and RAGE Plugin Hook before changing mod files".into());
        }
        let owned = self.ownership(id)?;
        if owned
            .iter()
            .any(|o| !o.game.eq_ignore_ascii_case(&game.path))
        {
            plan.blockers.push(
                "This mod belongs to another game directory. Select its original game folder."
                    .into(),
            );
        }
        if operation == "install" {
            if record.adopted_from.is_some() {
                plan.blockers.push("Released adoptions require a new Installed Mods scan and adoption approval; they cannot be installed as archive imports.".into());
            }
            if record.files.is_empty() {
                plan.blockers.push("This archive requires manual installation. Read its instructions; no target paths have been guessed.".into());
            }
            if !["Unspecified", "Legacy", "Both"].contains(&record.edition.as_str()) {
                plan.blockers.push(format!(
                    "Archive edition '{}' does not declare Legacy support",
                    record.edition
                ));
            }
        }
        let mut add = |target_name: &str,
                       after_hash: Option<String>,
                       expected: Option<Option<String>>|
         -> Result<()> {
            let path = target(Path::new(&game.path), target_name)?;
            let before_hash = current_hash(&path)?;
            if let Some(expected) = expected {
                if before_hash != expected {
                    plan.blockers.push(format!("External modification detected: {target_name}. Preserve or reconcile it manually before proceeding."));
                }
            }
            if let Some(owner) = self.owner(&game.path, target_name)? {
                if owner != id {
                    plan.blockers
                        .push(format!("File is owned by another mod: {target_name}"));
                }
            }
            let action = match (&before_hash, &after_hash) {
                (None, Some(_)) => "Create",
                (Some(_), None) => "Remove",
                (Some(a), Some(b)) if a == b => "Verify unchanged",
                (None, None) => "Verify absent",
                _ => "Back up & replace",
            };
            let size = record
                .files
                .iter()
                .find(|f| f.target.eq_ignore_ascii_case(target_name))
                .map(|f| f.size)
                .unwrap_or(0);
            plan.files.push(PlannedFile {
                target: target_name.into(),
                size,
                action: action.into(),
                before_hash,
                after_hash,
            });
            Ok(())
        };
        match operation {
            "install" if record.status == "imported" => {
                for f in &record.files {
                    add(&f.target, Some(f.hash.clone()), None)?;
                }
            }
            "enable" if record.status == "disabled" => {
                for o in &owned {
                    add(
                        &o.target,
                        Some(o.installed_hash.clone()),
                        Some(o.original_hash.clone()),
                    )?;
                }
            }
            "disable" if record.status == "enabled" => {
                for o in &owned {
                    add(
                        &o.target,
                        o.original_hash.clone(),
                        Some(Some(o.installed_hash.clone())),
                    )?;
                }
            }
            "remove" if record.status == "enabled" || record.status == "disabled" => {
                for o in &owned {
                    add(
                        &o.target,
                        if o.adopted {
                            Some(o.installed_hash.clone())
                        } else {
                            o.original_hash.clone()
                        },
                        Some(if record.status == "enabled" {
                            Some(o.installed_hash.clone())
                        } else {
                            o.original_hash.clone()
                        }),
                    )?;
                }
            }
            _ => bail!(
                "Operation {operation} is not valid for status {}",
                record.status
            ),
        }
        if operation == "install" || operation == "enable" {
            for req in &record.requirements {
                match game.prerequisites.iter().find(|p| &p.id == req) {
                    Some(p) if p.present => {}
                    Some(p) => plan
                        .blockers
                        .push(format!("Missing dependency: {}", p.name)),
                    None => plan.blockers.push(format!(
                        "Unrecognized dependency: {req}; manual verification required"
                    )),
                }
            }
            if !record.supported_builds.is_empty()
                && !game
                    .version
                    .as_ref()
                    .is_some_and(|v| record.supported_builds.contains(v))
            {
                plan.blockers.push(
                    "Game build is unknown or not in the archive's declared supported_builds"
                        .into(),
                );
            }
        }
        if operation == "disable" || operation == "remove" {
            // Do not disable an owned prerequisite needed by another enabled mod.
            for p in &game.prerequisites {
                let markers = match p.id.as_str() {
                    "asi" => vec!["dinput8.dll"],
                    "shv" => vec!["scripthookv.dll"],
                    "shvdn" => vec!["scripthookvdotnet.asi"],
                    "openiv" => vec!["openiv.asi"],
                    "rph" => vec!["ragepluginhook.exe"],
                    "lspdfr" => vec!["plugins/lspd first response.dll"],
                    _ => vec![],
                };
                if owned
                    .iter()
                    .any(|o| markers.contains(&o.target.to_lowercase().as_str()))
                    && self.records()?.iter().any(|r| {
                        r.id != id && r.status == "enabled" && r.requirements.contains(&p.id)
                    })
                {
                    plan.blockers
                        .push(format!("{} is required by another enabled mod", p.name));
                }
            }
        }
        self.pending.clear();
        self.pending.insert(
            plan.token.clone(),
            Pending {
                public: plan.clone(),
                record,
                restore: None,
                adoption: None,
                suit_id: None,
            },
        );
        Ok(plan)
    }
    pub fn prepare_restore(&mut self, backup_id: &str) -> Result<Plan> {
        self.batches.clear();
        let (status, json): (String, String) = self.db.query_row(
            "SELECT status,json FROM journals WHERE id=?",
            [backup_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if status != "committed" {
            bail!("Only completed backups can be restored");
        }
        let j: Journal = serde_json::from_str(&json)?;
        let record = self.record(&j.after_record.id)?;
        let game = self.selected_game()?.context("Select a game first")?;
        let mut blockers = self.recovery.clone();
        if game::game_running() {
            blockers.push("Close GTA V and RAGE Plugin Hook before restoring files".into());
        }
        if !game.path.eq_ignore_ascii_case(&j.game) || game.edition != "Legacy" {
            blockers.push("Select the backup's original Legacy game folder".into());
        }
        if serde_json::to_string(&record)? != serde_json::to_string(&j.after_record)?
            || self.ownership(&record.id)? != j.after_owned
        {
            blockers.push(
                "This backup is no longer the current mod state. Restore newer operations first."
                    .into(),
            );
        }
        if j.before_record.status == "enabled" {
            if j.before_record.adopted_from.is_some() {
                blockers.extend(self.adopted_scope_conflicts(&game, &j.before_record)?);
            }
            for req in &j.before_record.requirements {
                if !game.prerequisites.iter().any(|p| &p.id == req && p.present) {
                    blockers.push(format!("Missing dependency: {req}"));
                }
            }
            if !j.before_record.supported_builds.is_empty()
                && !game
                    .version
                    .as_ref()
                    .is_some_and(|v| j.before_record.supported_builds.contains(v))
            {
                blockers.push("Game build does not match the backup mod's supported builds".into());
            }
        }
        let mut files = Vec::new();
        for c in &j.changes {
            let current = current_hash(&target(Path::new(&game.path), &c.target)?)?;
            if current != c.after_hash {
                blockers.push(format!("External modification detected: {}", c.target));
            }
            if self
                .owner(&game.path, &c.target)?
                .is_some_and(|id| id != record.id)
            {
                blockers.push(format!("Another mod now owns {}", c.target));
            }
            files.push(PlannedFile {
                target: c.target.clone(),
                size: 0,
                action: "Restore backup".into(),
                before_hash: current,
                after_hash: c.before_hash.clone(),
            });
        }
        let plan = Plan { token:uuid::Uuid::new_v4().to_string(), mod_id:record.id.clone(), operation:"restore".into(), independent_scope:None, game, files, blockers, warnings:vec!["Restores the tracked file and mod state immediately before this operation. Other untracked changes are not covered.".into()] };
        self.pending.clear();
        self.pending.insert(
            plan.token.clone(),
            Pending {
                public: plan.clone(),
                record,
                restore: Some(j),
                adoption: None,
                suit_id: None,
            },
        );
        Ok(plan)
    }
    pub fn prepare_clean(&mut self) -> Result<Plan> {
        let game = self.selected_game()?.context("Select a game first")?;
        let mut plans = Vec::new();
        for record in self.records()? {
            if record.status != "imported"
                && record.adopted_from.is_none()
                && self
                    .ownership(&record.id)?
                    .iter()
                    .any(|o| o.game.eq_ignore_ascii_case(&game.path))
            {
                plans.push(self.prepare(&record.id, "remove")?);
            }
        }
        if plans.is_empty() {
            bail!("There are no tracked installed mods in this game folder");
        }
        let token = uuid::Uuid::new_v4().to_string();
        let plan = Plan { token:token.clone(), mod_id:"tracked-clean-state".into(), operation:"clean".into(), independent_scope:None, game, files:plans.iter().flat_map(|p|p.files.clone()).collect(), blockers:plans.iter().flat_map(|p|p.blockers.clone()).collect(), warnings:vec!["Restores originals or removes tracked files for Zoxryn-installed mods in this game folder. Adopted external entrypoints are excluded; manage them in Installed Mods. Archives, backups, and untracked files are retained.".into(), "Each mod has its own recoverable transaction. An interruption can leave some mods already removed; remaining records stay accurate.".into()] };
        self.pending.clear();
        self.batches.clear();
        self.batches.insert(token, plans);
        Ok(plan)
    }
    fn apply_batch(&mut self, plans: Vec<Plan>) -> Result<Report> {
        // Preflight all children before the first write, then revalidate each immediately before writing.
        for original in &plans {
            let fresh = self.prepare(&original.mod_id, "remove")?;
            if !fresh.blockers.is_empty()
                || fresh.game.path != original.game.path
                || serde_json::to_string(&fresh.files)? != serde_json::to_string(&original.files)?
            {
                bail!(
                    "Tracked clean-state plan changed. Review a new plan; no files were written."
                );
            }
        }
        let mut completed: Vec<Report> = Vec::new();
        for original in &plans {
            let result = (|| -> Result<Report> {
                let fresh = self.prepare(&original.mod_id, "remove")?;
                if !fresh.blockers.is_empty()
                    || fresh.game.path != original.game.path
                    || serde_json::to_string(&fresh.files)?
                        != serde_json::to_string(&original.files)?
                {
                    bail!("A target changed during tracked cleanup");
                }
                self.apply(&fresh.token, true, true)
            })();
            match result {
                Ok(report) => completed.push(report),
                Err(error) => {
                    let mut failures = Vec::new();
                    for report in completed.iter().rev() {
                        let rollback = self
                            .prepare_restore(&report.id)
                            .and_then(|p| self.apply(&p.token, true, true));
                        if let Err(e) = rollback {
                            failures.push(format!("{}: {e}", report.id));
                        }
                    }
                    bail!(
                        "Tracked cleanup stopped: {error}. {}",
                        if failures.is_empty() {
                            "Previously completed changes were rolled back.".into()
                        } else {
                            format!(
                                "Some rollback steps need manual review: {}",
                                failures.join("; ")
                            )
                        }
                    );
                }
            }
        }
        Ok(Report { id:uuid::Uuid::new_v4().to_string(), message:format!("Restored the tracked clean state for {} mods. Imported archives and backups retained. Untracked files were not changed.",completed.len()), files:completed.into_iter().flat_map(|r|r.files).collect() })
    }
    pub fn apply(
        &mut self,
        token: &str,
        approved: bool,
        compatibility_reviewed: bool,
    ) -> Result<Report> {
        self.apply_with_ownership(token, approved, compatibility_reviewed, false)
    }
    pub fn apply_with_ownership(
        &mut self,
        token: &str,
        approved: bool,
        compatibility_reviewed: bool,
        ownership_reviewed: bool,
    ) -> Result<Report> {
        if !approved || !compatibility_reviewed {
            bail!("Explicit plan approval and compatibility review are required");
        }
        if !self.recovery.is_empty() {
            bail!("Pending recovery must be resolved first");
        }
        if let Some(plans) = self.batches.remove(token) {
            return self.apply_batch(plans);
        }
        if self
            .pending
            .get(token)
            .is_some_and(|p| p.adoption.is_some() || p.suit_id.is_some())
            && !ownership_reviewed
        {
            bail!("Explicit independent-file ownership and registration confirmation is required");
        }
        let pending = self
            .pending
            .remove(token)
            .context("Plan expired. Review a new plan")?;
        let plan = pending.public;
        if !plan.blockers.is_empty() {
            bail!(
                "Installation plan has blockers: {}",
                plan.blockers.join("; ")
            );
        }
        // Recompute dependency, edition, ownership and file checks at approval time.
        if plan.operation == "adopt" {
            let candidate = pending
                .adoption
                .as_ref()
                .context("Missing adoption scope")?;
            let fresh = self.prepare_adoption(&candidate.id)?;
            if !fresh.blockers.is_empty()
                || fresh.game.path != plan.game.path
                || serde_json::to_string(&fresh.files)? != serde_json::to_string(&plan.files)?
            {
                bail!("Adoption scope or files changed; review a new plan");
            }
        } else if plan.operation == "restore" {
            let j = pending.restore.as_ref().unwrap();
            let fresh = self.prepare_restore(&j.id)?;
            if !fresh.blockers.is_empty() {
                bail!("Backup state changed; review a new plan");
            }
        } else {
            let fresh = if let Some(suit) = &pending.suit_id {
                self.prepare_suit(suit, &plan.operation)?
            } else {
                self.prepare(&plan.mod_id, &plan.operation)?
            };
            if !fresh.blockers.is_empty()
                || serde_json::to_string(&fresh.files)? != serde_json::to_string(&plan.files)?
                || fresh.game.path != plan.game.path
            {
                bail!("Plan changed since review. Review a new plan.");
            }
        }
        self.pending.clear();
        let root = Path::new(&plan.game.path);
        check_tree(root)?;
        let mut directories = vec![root.to_path_buf(), self.data.join("blobs")];
        for file in &plan.files {
            let path = target(root, &file.target)?;
            directories.push(path.parent().unwrap().to_path_buf());
        }
        let _directory_guards = guard_directories(&directories, false)?;
        // Invalidate in-flight scans before any filesystem or ownership change,
        // including an attempt that later fails and rolls back.
        self.revision += 1;
        self.db.execute(
            "UPDATE scan_runs SET json=json_set(json,'$.stale',json('true')) WHERE game=?",
            [&plan.game.path],
        )?;
        let before_owned = self.ownership(&pending.record.id)?;
        let mut after_record = pending.record.clone();
        let mut after_owned = before_owned.clone();
        let mut changes = Vec::new();
        let id = uuid::Uuid::new_v4().to_string();
        let mut _adoption_handles = Vec::new();
        if plan.operation == "adopt" {
            for (i, f) in pending.record.files.iter().enumerate() {
                let path = target(root, &f.target)?;
                let handle = open_read_no_follow(&path)?;
                if link_count(&handle)? != 1 {
                    bail!("Shared hard-linked files cannot be adopted");
                }
                let rel = format!("blobs/{id}-{i}-adopted");
                copy_verified(&path, &self.data.join(&rel), &f.hash)?;
                after_owned.push(Owned {
                    game: plan.game.path.clone(),
                    target: f.target.clone(),
                    mod_id: pending.record.id.clone(),
                    installed_hash: f.hash.clone(),
                    original_hash: None,
                    original_blob: None,
                    installed_blob: rel.clone(),
                    adopted: true,
                });
                after_record.archive_path = self.data.join(rel).to_string_lossy().into();
                after_record.archive_hash = f.hash.clone();
                _adoption_handles.push(handle);
            }
            after_record.status = "enabled".into();
            after_record.installed_at = Some(now());
            after_record.game_path = Some(plan.game.path.clone());
            self.save_record(&pending.record)?;
        }
        if plan.operation == "install" {
            if hash_file(Path::new(&pending.record.archive_path))? != pending.record.archive_hash {
                bail!("Cached archive changed. Import it again.");
            }
            after_owned.clear();
            for (i, f) in pending.record.files.iter().enumerate() {
                let rel = format!("blobs/{id}-{i}-installed");
                archive::extract_selected(
                    Path::new(&pending.record.archive_path),
                    f,
                    &self.data.join(&rel),
                )?;
                let original_hash = current_hash(&target(root, &f.target)?)?;
                let original_blob = if let Some(h) = &original_hash {
                    let p = format!("blobs/{id}-{i}-original");
                    copy_verified(&target(root, &f.target)?, &self.data.join(&p), h)?;
                    Some(p)
                } else {
                    None
                };
                after_owned.push(Owned {
                    game: plan.game.path.clone(),
                    target: f.target.clone(),
                    mod_id: pending.record.id.clone(),
                    installed_hash: f.hash.clone(),
                    original_hash,
                    original_blob,
                    installed_blob: rel,
                    adopted: false,
                });
            }
            after_record.status = "enabled".into();
            after_record.installed_at = Some(now());
            after_record.game_path = Some(plan.game.path.clone());
        } else if plan.operation == "enable" {
            after_record.status = "enabled".into();
        } else if plan.operation == "disable" {
            after_record.status = "disabled".into();
        } else if plan.operation == "remove" {
            after_record.status = "imported".into();
            after_owned.clear();
        } else if let Some(j) = &pending.restore {
            after_record = j.before_record.clone();
            after_owned = j.before_owned.clone();
        }
        for (i, f) in plan.files.iter().enumerate() {
            let path = target(root, &f.target)?;
            if current_hash(&path)? != f.before_hash {
                bail!("Target changed after approval: {}", f.target);
            }
            let before_blob = if let Some(h) = &f.before_hash {
                let p = format!("blobs/{id}-{i}-before");
                copy_verified(&path, &self.data.join(&p), h)?;
                Some(p)
            } else {
                None
            };
            let after_blob = if let Some(j) = &pending.restore {
                j.changes
                    .iter()
                    .find(|c| c.target == f.target)
                    .and_then(|c| c.before_blob.clone())
            } else {
                after_owned
                    .iter()
                    .chain(before_owned.iter())
                    .find(|o| o.target == f.target)
                    .and_then(|o| {
                        if f.after_hash.as_ref() == Some(&o.installed_hash) {
                            Some(o.installed_blob.clone())
                        } else {
                            o.original_blob.clone()
                        }
                    })
            };
            if f.after_hash.is_some() && after_blob.is_none() {
                bail!("Missing restore data for {}", f.target);
            }
            changes.push(Change {
                target: f.target.clone(),
                before_hash: f.before_hash.clone(),
                after_hash: f.after_hash.clone(),
                before_blob,
                after_blob,
            });
        }
        after_owned.sort_by_key(|o| o.target.to_lowercase());
        let journal = Journal {
            id: id.clone(),
            game: plan.game.path,
            mod_name: pending.record.name.clone(),
            operation: plan.operation,
            created_at: now(),
            before_record: pending.record,
            after_record,
            before_owned,
            after_owned,
            changes,
        };
        self.db.execute(
            "INSERT INTO journals VALUES (?,'pending',?)",
            params![id, serde_json::to_string(&journal)?],
        )?;
        let result = self
            .apply_files(&journal)
            .and_then(|_| self.commit(&journal));
        if let Err(e) = result {
            match self.rollback(&journal) {
                Ok(()) => {
                    self.db
                        .execute("UPDATE journals SET status='rolled_back' WHERE id=?", [&id])?;
                }
                Err(recovery) => {
                    self.recovery
                        .push(format!("Recovery required for {id}: {recovery}"));
                }
            }
            bail!(
                "Operation failed: {e}. {}",
                if self.recovery.is_empty() {
                    "All written files were rolled back."
                } else {
                    "Recovery is incomplete. Files were preserved; see recovery report."
                }
            );
        }
        Ok(Report {
            id,
            message: format!(
                "{} completed and verified for {}. {} tracked files.",
                journal.operation,
                journal.mod_name,
                journal.changes.len()
            ),
            files: journal.changes.iter().map(|c| c.target.clone()).collect(),
        })
    }
    fn write_state(
        &self,
        root: &Path,
        rel: &str,
        blob: &Option<String>,
        hash: &Option<String>,
    ) -> Result<()> {
        let path = target(root, rel)?;
        let parent = path.parent().unwrap();
        let _directory_guard = guard_tree(parent, hash.is_some())?;
        if let Some(h) = hash {
            let source = self
                .data
                .join(blob.as_ref().context("Backup blob is missing")?);
            check_tree(&source)?;
            check_tree(parent)?;
            let temp = parent.join(format!(".zoxryn-{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| -> Result<()> {
                copy_verified(&source, &temp, h)?;
                target(root, rel)?;
                fs::rename(&temp, &path)?;
                Ok(())
            })();
            if temp.exists() {
                let _ = fs::remove_file(&temp);
            }
            result?;
        } else if path.exists() {
            fs::remove_file(&path)?;
        }
        if current_hash(&path)? != *hash {
            bail!("Post-write verification failed: {rel}");
        }
        Ok(())
    }
    fn apply_files(&self, journal: &Journal) -> Result<()> {
        for c in &journal.changes {
            let root = Path::new(&journal.game);
            if current_hash(&target(root, &c.target)?)? != c.before_hash {
                bail!("Target changed during operation: {}", c.target);
            }
            if c.before_hash != c.after_hash {
                self.write_state(root, &c.target, &c.after_blob, &c.after_hash)?;
            }
        }
        Ok(())
    }
    fn commit(&mut self, j: &Journal) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO mods VALUES (?,?)",
            params![j.after_record.id, serde_json::to_string(&j.after_record)?],
        )?;
        tx.execute("DELETE FROM owned WHERE mod_id=?", [&j.after_record.id])?;
        for o in &j.after_owned {
            tx.execute(
                "INSERT INTO owned VALUES (?,?,?,?)",
                params![o.game, o.target, o.mod_id, serde_json::to_string(o)?],
            )?;
        }
        tx.execute("UPDATE journals SET status='committed' WHERE id=?", [&j.id])?;
        tx.execute(
            "UPDATE scan_runs SET json=json_set(json,'$.stale',json('true')) WHERE game=?",
            [&j.game],
        )?;
        tx.commit()?;
        self.revision += 1;
        Ok(())
    }
    fn rollback(&self, journal: &Journal) -> Result<()> {
        for c in journal.changes.iter().rev() {
            let root = Path::new(&journal.game);
            let current = current_hash(&target(root, &c.target)?)?;
            if current == c.before_hash {
                continue;
            }
            if current != c.after_hash {
                bail!(
                    "External changes at {}; recovery refuses to overwrite them",
                    c.target
                );
            }
            self.write_state(root, &c.target, &c.before_blob, &c.before_hash)?;
        }
        Ok(())
    }
    pub fn recover(&mut self) -> Result<()> {
        self.recovery.clear();
        let pending: Vec<String> = {
            let mut stmt = self
                .db
                .prepare("SELECT json FROM journals WHERE status='pending' ORDER BY rowid DESC")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        for json in pending {
            let j: Journal = serde_json::from_str(&json)?;
            self.revision += 1;
            self.db.execute(
                "UPDATE scan_runs SET json=json_set(json,'$.stale',json('true')) WHERE game=?",
                [&j.game],
            )?;
            match self.rollback(&j) {
                Ok(()) => {
                    self.db
                        .execute("UPDATE journals SET status='recovered' WHERE id=?", [&j.id])?;
                }
                Err(e) => self.recovery.push(format!("{}: {e}", j.id)),
            }
        }
        Ok(())
    }
}

mod inventory;
mod organization;
#[cfg(test)]
mod tests;
