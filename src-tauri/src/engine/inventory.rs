use super::*;
use crate::scanner;
impl Engine {
    pub(super) fn adopted_scope_conflicts(
        &self,
        game: &Game,
        record: &ModRecord,
    ) -> Result<Vec<String>> {
        let (_, mut owners, _) = self.scan_context()?;
        for f in &record.files {
            owners.remove(&f.target.to_lowercase());
        }
        let inventory = scanner::scan(game, &owners)?;
        let mut conflicts = Vec::new();
        if !inventory.complete {
            conflicts
                .push("Cannot safely toggle an adopted entrypoint after an incomplete scan".into());
        }
        for file in &record.files {
            let disk = target(Path::new(&game.path), &file.target)?;
            if disk.exists() {
                let _guard = guard_existing_tree(disk.parent().unwrap())?;
                let handle = open_read_no_follow(&disk)?;
                if link_count(&handle)? != 1 {
                    conflicts.push(format!(
                        "Adopted file is now shared through a hard link: {}",
                        file.target
                    ));
                }
            }
            if let Some(candidate) = inventory
                .items
                .iter()
                .find(|i| i.location.eq_ignore_ascii_case(&file.target))
            {
                conflicts.extend(
                    candidate
                        .conflicts
                        .iter()
                        .filter(|c| !c.starts_with("Dependency not detected:"))
                        .cloned(),
                );
            }
            let scope = Path::new(&file.target);
            let basename = scope
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            for other in inventory
                .items
                .iter()
                .flat_map(|item| item.files.iter().map(|f| &f.path))
                .chain(owners.keys())
            {
                if other.eq_ignore_ascii_case(&file.target) {
                    continue;
                }
                if let Some((active, _, extension)) = scanner::entrypoint(other) {
                    if ["asi", "dll", "cs", "vb"].contains(&extension.as_str())
                        && Path::new(&active)
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(&basename)
                    {
                        conflicts.push(format!(
                            "Potential shared executable/script with matching stem: {other}"
                        ));
                    }
                }
            }
            let parent = scope.parent().unwrap_or(Path::new(""));
            let parent_path = Path::new(&game.path).join(parent);
            let _guards = guard_existing_tree(&parent_path)?;
            for entry in fs::read_dir(&parent_path)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.eq_ignore_ascii_case(
                    scope
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .as_ref(),
                ) {
                    continue;
                }
                let active = scanner::entrypoint(&name)
                    .map(|v| v.0)
                    .unwrap_or(name.clone());
                if name.eq_ignore_ascii_case(&basename)
                    || Path::new(&active)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&basename)
                {
                    conflicts.push(format!(
                        "New companion or shared file prevents safe toggle: {}",
                        entry.path().display()
                    ));
                }
            }
        }
        Ok(conflicts)
    }
    pub fn scan_context(&self) -> Result<(Game, HashMap<String, String>, u64)> {
        let game = self
            .selected_game()?
            .context("Select a Legacy installation first")?;
        let mut stmt = self
            .db
            .prepare("SELECT target,mod_id FROM owned WHERE game=?")?;
        let rows = stmt.query_map([&game.path], |r| {
            Ok((r.get::<_, String>(0)?.to_lowercase(), r.get(1)?))
        })?;
        Ok((
            game,
            rows.collect::<std::result::Result<_, _>>()?,
            self.revision,
        ))
    }
    pub fn scan_installed(&mut self) -> Result<ScanInventory> {
        let (game, owners, revision) = self.scan_context()?;
        let result = scanner::scan(&game, &owners)?;
        self.persist_scan(result, revision)
    }
    pub fn persist_scan(
        &mut self,
        mut inventory: ScanInventory,
        revision: u64,
    ) -> Result<ScanInventory> {
        let selected: Game =
            serde_json::from_str(&self.json_setting("game")?.context("No selected game")?)?;
        if !selected.path.eq_ignore_ascii_case(&inventory.game_path) {
            bail!("Game selection changed; this scan result was discarded");
        }
        inventory.stale = revision != self.revision;
        inventory.managed.clear();
        let mut items = std::mem::take(&mut inventory.items);
        let present_ids: std::collections::HashSet<String> =
            items.iter().map(|item| item.id.clone()).collect();
        let retained: Vec<DiscoveredMod> = {
            let mut stmt = self
                .db
                .prepare("SELECT json FROM external_mods WHERE game=?")?;
            let rows = stmt.query_map([&inventory.game_path], |r| r.get::<_, String>(0))?;
            rows.map(|r| Ok(serde_json::from_str(&r?)?))
                .collect::<Result<_>>()?
        };
        for mut previous in retained {
            if !present_ids.contains(&previous.id) {
                if let Some(owner) = self.owner(&inventory.game_path, &previous.location)? {
                    previous.managed_id = Some(owner.clone());
                    previous.status = self.record(&owner)?.status;
                    previous.adoptable = false;
                    items.push(previous);
                }
            }
        }
        let tx = self.db.transaction()?;
        tx.execute(
            "DELETE FROM external_mods WHERE game=?",
            [&inventory.game_path],
        )?;
        for item in items {
            tx.execute(
                "INSERT INTO external_mods VALUES (?,?,?)",
                params![inventory.game_path, item.id, serde_json::to_string(&item)?],
            )?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO scan_runs VALUES (?,?)",
            params![inventory.game_path, serde_json::to_string(&inventory)?],
        )?;
        tx.commit()?;
        self.inventory()?.context("Scan inventory unavailable")
    }
    pub fn inventory(&self) -> Result<Option<ScanInventory>> {
        let Some(json) = self.json_setting("game")? else {
            return Ok(None);
        };
        let game: Game = serde_json::from_str(&json)?;
        let run: Option<String> = self
            .db
            .query_row(
                "SELECT json FROM scan_runs WHERE game=?",
                [&game.path],
                |r| r.get(0),
            )
            .optional()?;
        let Some(run) = run else { return Ok(None) };
        let mut inventory: ScanInventory = serde_json::from_str(&run)?;
        let mut stmt = self
            .db
            .prepare("SELECT json FROM external_mods WHERE game=? ORDER BY id")?;
        let rows = stmt.query_map([&game.path], |r| r.get::<_, String>(0))?;
        for row in rows {
            let mut item: DiscoveredMod = serde_json::from_str(&row?)?;
            item.managed_id = self.owner(&game.path, &item.location)?;
            inventory.items.push(item);
        }
        for record in self.records()? {
            if !["enabled", "disabled"].contains(&record.status.as_str()) {
                continue;
            }
            let owned = self.ownership(&record.id)?;
            if !owned
                .iter()
                .any(|o| o.game.eq_ignore_ascii_case(&game.path))
            {
                continue;
            }
            let mut notes = Vec::new();
            let mut status = record.status.clone();
            for o in &owned {
                match target(Path::new(&game.path), &o.target).and_then(|p| Ok(p.is_file())) {
                    Ok(false) if record.status == "enabled" => {
                        status = "needs_review".into();
                        notes.push(format!("Expected managed file is missing: {}", o.target));
                    }
                    Err(e) => {
                        status = "needs_review".into();
                        notes.push(format!("Unsafe or inaccessible target: {e}"));
                    }
                    _ => {}
                }
            }
            notes.push("Managed status comes from the ownership ledger. File contents are rehashed before a change, not during routine scanning.".into());
            inventory.managed.push(ManagedInstalled {
                id: record.id,
                name: record.name,
                category: record.category,
                status,
                source: record.origin,
                files: owned.iter().map(|o| o.target.clone()).collect(),
                dependencies: record.requirements,
                notes,
                adopted: record.adopted_from.is_some(),
            });
        }
        Ok(Some(inventory))
    }
    pub fn prepare_adoption(&mut self, external_id: &str) -> Result<Plan> {
        self.pending.clear();
        self.batches.clear();
        let inventory = self.scan_installed()?;
        let candidate = inventory
            .items
            .into_iter()
            .find(|i| i.id == external_id)
            .context(
                "External file disappeared or is already managed; rescan the installed inventory",
            )?;
        let game = self.selected_game()?.context("Select a game first")?;
        let mut blockers = self.recovery.clone();
        if !candidate.adoptable || !inventory.complete || inventory.stale {
            blockers.push(candidate.control_reason.clone());
        }
        blockers.extend(candidate.conflicts.clone());
        if game::game_running() {
            blockers.push("Close GTA V and RAGE Plugin Hook before adoption".into());
        }
        if candidate.files.len() != 1 {
            blockers
                .push("Adoption requires an independently confirmed single-file entrypoint".into());
        }
        let mut files = Vec::new();
        let mut mod_files = Vec::new();
        if candidate.adoptable {
            for file in &candidate.files {
                let path = target(Path::new(&game.path), &file.path)?;
                let _guards = guard_tree(path.parent().unwrap(), false)?;
                let handle = open_read_no_follow(&path)?;
                if link_count(&handle)? != 1 {
                    blockers.push(format!("Shared hard-linked file: {}", file.path));
                }
                if file.size > MAX_FILE {
                    blockers.push("File exceeds the 256 MiB adoption limit".into());
                    continue;
                }
                if self.owner(&game.path, &file.path)?.is_some() {
                    blockers.push(format!("Another managed mod owns {}", file.path));
                }
                let hash = hash_file(&path)?;
                files.push(PlannedFile {
                    target: file.path.clone(),
                    size: file.size,
                    action: "Verify backup & adopt (game file unchanged)".into(),
                    before_hash: Some(hash.clone()),
                    after_hash: Some(hash.clone()),
                });
                mod_files.push(ModFile {
                    archive_entry: file.path.clone(),
                    target: file.path.clone(),
                    hash,
                    size: file.size,
                });
            }
        }
        let record=ModRecord{id:uuid::Uuid::new_v4().to_string(),name:candidate.name.clone(),author:"Unknown (external file)".into(),version:"Unknown".into(),category:candidate.category.clone(),source:format!("Existing {} file: {}",candidate.source,candidate.location),imported_at:now(),installed_at:None,status:"imported".into(),archive_path:String::new(),archive_hash:String::new(),kind:"Adopted entrypoint".into(),edition:"Legacy".into(),requirements:candidate.dependencies.clone(),warnings:vec!["Adoption manages only the reviewed standalone entrypoint. You must confirm it is independent and not shared; dependencies are not adopted.".into()],instructions:"Disabling removes only the approved entrypoint after a verified retained backup; enabling restores its exact bytes. Release control restores the file to its adoption-time state and retains backups. Re-adoption requires a new installed scan and approval.".into(),files:mod_files,supported_builds:vec![],game_path:Some(game.path.clone()),adopted_from:Some(candidate.id.clone()),origin:candidate.source.clone()};
        let plan=Plan{token:uuid::Uuid::new_v4().to_string(),mod_id:record.id.clone(),operation:"adopt".into(),game,files,blockers,warnings:vec!["Adoption creates a verified local backup and ownership record without changing game files. Later disable/enable operations require their own approval.".into(),"Confirm that the listed file is the complete independent entrypoint, is not a shared dependency, and has no unlisted resources requiring coordinated management. If unsure, cancel and manage it manually.".into()]};
        self.pending.insert(
            plan.token.clone(),
            Pending {
                public: plan.clone(),
                record,
                restore: None,
                adoption: Some(candidate),
            },
        );
        Ok(plan)
    }
}
