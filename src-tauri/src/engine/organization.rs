use super::*;
use crate::{organization::*, previews, scanner};
use std::collections::{BTreeMap, HashSet};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Visual {
    preview: Option<String>,
    name: Option<String>,
    category: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Custom {
    name: String,
    description: String,
    category: String,
}
fn suit_id(mod_id: &str, key: &str) -> String {
    hash_bytes(format!("suit:{mod_id}:{key}").as_bytes())
}
fn normalized_status(status: &str) -> String {
    if ["enabled", "disabled", "imported", "needs_review", "unknown"].contains(&status) {
        status
    } else {
        "unknown"
    }
    .into()
}
fn file_key(file: &str) -> String {
    file.split(" :: ").next().unwrap_or(file).to_lowercase()
}

impl Engine {
    fn visual(&self, kind: &str, id: &str) -> Result<Visual> {
        let json: Option<String> = self
            .db
            .query_row(
                "SELECT json FROM visual_metadata WHERE kind=? AND item_id=?",
                params![kind, id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(json
            .map(|j| serde_json::from_str(&j))
            .transpose()?
            .unwrap_or_default())
    }
    fn write_visual(&self, kind: &str, id: &str, visual: &Visual) -> Result<()> {
        self.db.execute(
            "INSERT OR REPLACE INTO visual_metadata VALUES (?,?,?)",
            params![kind, id, serde_json::to_string(visual)?],
        )?;
        Ok(())
    }
    pub(super) fn import_package_previews(&self, record: &ModRecord) -> Result<()> {
        let mut requested = Vec::new();
        if let Some(path) = &record.organization.preview {
            requested.push(("managed", record.id.clone(), path));
        }
        for suit in &record.organization.suits {
            if let Some(path) = &suit.preview {
                requested.push(("suit", suit_id(&record.id, &suit.key), path));
            }
        }
        if requested.is_empty() {
            return Ok(());
        }
        let file = open_read_no_follow(Path::new(&record.archive_path))?;
        let mut zip = zip::ZipArchive::new(file)?;
        for (kind, id, path) in requested {
            let mut image = zip.by_name(path)?;
            if image.size() > previews::MAX_IMAGE {
                bail!("Declared preview exceeds the image budget");
            }
            use std::io::Read;
            let mut bytes = Vec::new();
            image
                .by_ref()
                .take(previews::MAX_IMAGE + 1)
                .read_to_end(&mut bytes)?;
            let key = previews::store(&self.data, &bytes)?;
            let mut visual = self.visual(kind, &id)?;
            visual.preview = Some(key);
            self.write_visual(kind, &id, &visual)?;
        }
        Ok(())
    }
    fn standalone_suit_reason(
        &self,
        record: &ModRecord,
        declared: &SuitDeclaration,
    ) -> Result<Option<String>> {
        if record.origin != "Zoxryn" || record.adopted_from.is_some() {
            return Ok(Some("External suit packages require verified adoption of their complete installation; this layout is read-only.".into()));
        }
        if record.organization.suits.len() != 1 || !declared.standalone {
            return Ok(Some("Individual files/configuration cannot be isolated from this multi-component or shared package. Manage its parent package only after a separate review.".into()));
        }
        let declared_files: HashSet<_> = declared.files.iter().map(|f| f.to_lowercase()).collect();
        let operative: HashSet<_> = record
            .files
            .iter()
            .map(|f| f.target.to_lowercase())
            .collect();
        if operative.is_empty() || operative != declared_files {
            return Ok(Some("Independent management requires the suit to declare every operative file in its parent package.".into()));
        }
        let entrypoints: Vec<_> = operative
            .iter()
            .filter(|p| p.ends_with(".asi") || p.ends_with(".cs") || p.ends_with(".vb"))
            .collect();
        if entrypoints.len() != 1 {
            return Ok(Some("Only a standalone ASI or direct ScriptHookVDotNet source-script outfit package is supported.".into()));
        }
        let entry = entrypoints[0];
        let path = Path::new(entry);
        let parent = path.parent().unwrap_or(Path::new(""));
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let valid_entry = (!entry.contains('/') && entry.ends_with(".asi"))
            || (parent == Path::new("scripts")
                && (entry.ends_with(".cs") || entry.ends_with(".vb")));
        let sidecars = operative.iter().all(|f| {
            let p = Path::new(f);
            f == entry
                || (p.parent() == Some(parent)
                    && p.file_stem().unwrap_or_default().to_string_lossy() == stem
                    && ["ini", "cfg", "json", "xml", "dat"]
                        .contains(&p.extension().unwrap_or_default().to_string_lossy().as_ref()))
        });
        if !valid_entry || !sidecars {
            return Ok(Some("Only one independent loader and its exact matching local configuration files are supported. Model/RPF resources and registration edits require manual management.".into()));
        }
        if !["enabled", "disabled"].contains(&record.status.as_str()) {
            return Ok(Some("This package is imported but not installed. Review installation in My library first.".into()));
        }
        let game = self.selected_game()?;
        if game.as_ref().map(|g| g.path.as_str()) != record.game_path.as_deref() {
            return Ok(Some(
                "Select the package's original game directory before managing this suit.".into(),
            ));
        }
        let owned = self.ownership(&record.id)?;
        if owned.len() != operative.len()
            || owned.iter().any(|o| o.original_hash.is_some() || o.original_blob.is_some() || o.adopted
                || record.game_path.as_deref() != Some(o.game.as_str())
                || !record.files.iter().any(|f| f.target.eq_ignore_ascii_case(&o.target) && f.hash == o.installed_hash))
        {
            return Ok(Some("Original replacements or incomplete ownership prevent safe individual suit management.".into()));
        }
        Ok(None)
    }
    pub fn organization_snapshot(&self) -> Result<OrganizationSnapshot> {
        let records = self.records()?;
        let inventory = self.inventory()?;
        let mut output = OrganizationSnapshot::default();
        let mut available = BTreeMap::<String, Component>::new();
        let mut collections = Vec::new();
        for record in &records {
            let visual = self.visual("managed", &record.id)?;
            let component=Component{reference:MemberRef{kind:"managed".into(),id:record.id.clone()},name:record.name.clone(),category:record.category.clone(),status:normalized_status(&record.status),source:record.origin.clone(),location:record.game_path.clone().unwrap_or_else(||"Imported archive".into()),files:record.files.iter().map(|f|f.target.clone()).collect(),dependencies:record.requirements.clone(),role:"Managed package".into(),control_reason:"Operations use the existing reviewed package plan and canonical ownership records.".into(),mod_id:Some(record.id.clone()),preview:visual.preview};
            available.insert(format!("managed:{}", record.id), component.clone());
            let mut members = vec![component];
            for declaration in &record.organization.suits {
                if output.suits.len() >= 2048 {
                    if !output.warnings.iter().any(|w| w.contains("2,048")) {
                        output.warnings.push("Suit discovery reached its 2,048-item budget. Some candidates were not listed.".into());
                    }
                    break;
                }
                let id = suit_id(&record.id, &declaration.key);
                let visual = self.visual("suit", &id)?;
                let reason = self.standalone_suit_reason(record, declaration)?;
                let suit=Suit{id:id.clone(),name:visual.name.unwrap_or_else(||declaration.name.clone()),category:visual.category.unwrap_or_else(||declaration.category.clone()),model:declaration.model.clone(),parent:record.name.clone(),source:record.origin.clone(),status:normalized_status(&record.status),files:declaration.files.clone(),dependencies:record.requirements.clone(),conflicts:vec![],notes:vec!["Identity and file membership are explicitly declared by the imported package metadata. This does not establish creator authenticity or in-game compatibility.".into()],manageable:reason.is_none(),control_reason:reason.unwrap_or_else(||"Independently managed script/ASI suit package. The complete package is backed up and toggled; this does not equip a suit in game.".into()),mod_id:Some(record.id.clone()),preview:visual.preview};
                let child = self.suit_component(&suit);
                available.insert(format!("suit:{id}"), child.clone());
                members.push(child);
                output.suits.push(suit);
            }
            let id = format!("package:{}", record.id);
            let override_visual = self.visual("collection", &id)?;
            let unique_files = members
                .iter()
                .flat_map(|m| m.files.iter().map(|f| file_key(f)))
                .collect::<HashSet<_>>()
                .len();
            collections.push(Collection{id,name:override_visual.name.unwrap_or_else(||record.organization.collection_name.clone().unwrap_or_else(||record.name.clone())),description:"One imported package and its explicitly declared components; file ownership is retained by its original mod record.".into(),category:override_visual.category.unwrap_or_else(||record.category.clone()),automatic:true,relationship:"Imported archive and canonical package ownership".into(),status:aggregate_status(members.iter().map(|m|m.status.as_str())),preview:override_visual.preview.or_else(||members[0].preview.clone()),members,unique_files,missing_members:0});
        }
        if let Some(inventory) = &inventory {
            if !inventory.complete || inventory.stale {
                output.warnings.push("Installed inventory is partial or stale. Rescan to refresh detection; unsafe controls remain blocked.".into());
            }
            let mut external_collections = BTreeMap::<String, Collection>::new();
            let dlc_roots: HashSet<_> = inventory
                .items
                .iter()
                .filter(|i| i.kind == "DLC package")
                .map(|i| i.location.to_lowercase())
                .collect();
            for item in &inventory.items {
                if item.managed_id.is_some() {
                    continue;
                }
                let component = Component {
                    reference: MemberRef {
                        kind: "external".into(),
                        id: item.id.clone(),
                    },
                    name: item.name.clone(),
                    category: item.category.clone(),
                    status: normalized_status(&item.status),
                    source: item.source.clone(),
                    location: item.location.clone(),
                    files: item.files.iter().map(|f| f.path.clone()).collect(),
                    dependencies: item.dependencies.clone(),
                    role: if item.kind == "Shared prerequisite" {
                        "Required dependency"
                    } else {
                        "External component; ownership unverified"
                    }
                    .into(),
                    control_reason: item.control_reason.clone(),
                    mod_id: None,
                    preview: None,
                };
                available.insert(format!("external:{}", item.id), component);
                let prefix = item
                    .location
                    .to_lowercase()
                    .split('/')
                    .take(5)
                    .collect::<Vec<_>>()
                    .join("/");
                if item.kind == "DLC package"
                    || (item.kind == "OpenIV RPF container" && !dlc_roots.contains(&prefix))
                {
                    let id = format!("container:{}", item.id);
                    let visual = self.visual("collection", &id)?;
                    external_collections.insert(item.location.to_lowercase(),Collection{id,name:visual.name.unwrap_or_else(||item.name.clone()),description:"Container membership is a structural relationship. A container can hold multiple unrelated mods; it is not an identified single mod.".into(),category:visual.category.unwrap_or_else(||item.category.clone()),automatic:true,relationship:"Verified folder/archive boundary; individual identity unverified".into(),status:"needs_review".into(),preview:visual.preview,members:vec![],unique_files:0,missing_members:0});
                }
            }
            let mut seen = HashSet::new();
            let mut candidates = Vec::new();
            for item in &inventory.items {
                if output.suits.len() + candidates.len() >= 2048 {
                    if !output.warnings.iter().any(|w| w.contains("2,048")) {
                        output.warnings.push("Suit discovery reached its 2,048-item budget. Some candidates were not listed.".into());
                    }
                    break;
                }
                if item.managed_id.is_some() {
                    continue;
                }
                for model in &item.character_models {
                    if output.suits.len() + candidates.len() >= 2048 {
                        break;
                    }
                    let id = hash_bytes(format!("ped-model:{}:{model}", item.id).as_bytes());
                    let visual = self.visual("suit", &id)?;
                    let suit=Suit{id:id.clone(),name:visual.name.unwrap_or_else(||model.clone()),category:visual.category.unwrap_or_else(||"Custom Peds".into()),model:Some(model.clone()),parent:item.name.clone(),source:item.source.clone(),status:"needs_review".into(),files:vec![item.location.clone()],dependencies:vec!["openiv".into()],conflicts:item.conflicts.clone(),notes:vec!["Model name is declared in bounded CPedModelInfo metadata. The physical model/texture bindings and AddonPeds runtime registry are unverified; related resources have not been guessed from filenames.".into()],manageable:false,control_reason:"Shared ped-registration metadata cannot be safely edited per suit. Use the original package instructions; no registry, RPF or runtime selection is changed here.".into(),mod_id:None,preview:visual.preview};
                    available.insert(format!("suit:{id}"), self.suit_component(&suit));
                    output.suits.push(suit);
                }
                if is_character_resource(&item.location) {
                    candidates.push((item, item.location.clone(), None));
                }
                if let Some(rpf) = &item.rpf {
                    if rpf.character_resources_truncated {
                        output.warnings.push(format!("Character-resource limit reached in {}; remaining resources were not listed.",item.location));
                    }
                    if rpf.state == "Readable TOC" {
                        for path in &rpf.character_resources {
                            if output.suits.len() + candidates.len() >= 2048 {
                                break;
                            }
                            candidates.push((
                                item,
                                format!("{} :: {path}", item.location),
                                Some(path.clone()),
                            ));
                        }
                    } else if item.location.to_lowercase().contains("addonpeds")
                        || rpf.categories.contains_key("Characters")
                    {
                        candidates.push((
                            item,
                            item.location.clone(),
                            Some("Unknown archive contents".into()),
                        ));
                    }
                }
            }
            for (item, location, resource) in candidates {
                if output.suits.len() >= 2048 {
                    output.warnings.push("Suit discovery reached its 2,048-item budget. Some candidates were not listed.".into());
                    break;
                }
                let id = hash_bytes(format!("resource:{}:{location}", item.id).as_bytes());
                if !seen.insert(id.clone()) {
                    continue;
                }
                let visual = self.visual("suit", &id)?;
                let name = resource
                    .as_deref()
                    .unwrap_or(&item.location)
                    .rsplit('/')
                    .next()
                    .unwrap_or("Unidentified character resource");
                let suit=Suit{id:id.clone(),name:visual.name.unwrap_or_else(||format!("Resource: {name}")),category:visual.category.unwrap_or_else(||"Other / Unidentified".into()),model:None,parent:item.name.clone(),source:item.source.clone(),status:"needs_review".into(),files:vec![location],dependencies:item.dependencies.clone(),conflicts:item.conflicts.clone(),notes:vec!["Character-resource clue only. Exact suit identity, model registration, configuration ownership and compatibility are not established. Similar names are not grouped into an asserted suit.".into()],manageable:false,control_reason:"Shared/unknown model or archive contents cannot be isolated safely. OpenIV and AddonPeds registration edits require manual management; original RPFs are never changed.".into(),mod_id:None,preview:visual.preview};
                available.insert(format!("suit:{id}"), self.suit_component(&suit));
                output.suits.push(suit);
            }
            // Each external reference belongs to its nearest physical container, not
            // every ancestor; this bounds work and avoids overlapping automatic groups.
            for component in available
                .values()
                .filter(|c| c.reference.kind != "managed" && c.mod_id.is_none())
            {
                let location = component
                    .files
                    .first()
                    .unwrap_or(&component.location)
                    .to_lowercase();
                let physical = location.split(" :: ").next().unwrap_or(&location);
                let mut key = physical.to_string();
                let mut found = None;
                loop {
                    if external_collections.contains_key(&key) {
                        found = Some(key);
                        break;
                    }
                    if let Some((parent, _)) = key.rsplit_once('/') {
                        key = parent.into();
                    } else {
                        break;
                    }
                }
                if let Some(key) = found {
                    external_collections
                        .get_mut(&key)
                        .unwrap()
                        .members
                        .push(component.clone());
                }
            }
            for (_, mut collection) in external_collections {
                collection.unique_files = collection
                    .members
                    .iter()
                    .flat_map(|m| m.files.iter().map(|f| file_key(f)))
                    .collect::<HashSet<_>>()
                    .len();
                collection.status =
                    aggregate_status(collection.members.iter().map(|m| m.status.as_str()));
                collections.push(collection);
            }
        }
        let mut stmt = self
            .db
            .prepare("SELECT id,json FROM collections ORDER BY rowid")?;
        let custom = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (id, json) in custom {
            let custom: Custom = serde_json::from_str(&json)?;
            let visual = self.visual("collection", &id)?;
            let mut stmt = self.db.prepare(
                "SELECT kind,item_id FROM collection_members WHERE collection_id=? ORDER BY rowid",
            )?;
            let refs = stmt
                .query_map([&id], |r| {
                    Ok(MemberRef {
                        kind: r.get(0)?,
                        id: r.get(1)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut missing = 0;
            let members=refs.into_iter().map(|reference|{available.get(&format!("{}:{}",reference.kind,reference.id)).cloned().unwrap_or_else(||{missing+=1;Component{reference,name:"Missing component".into(),category:"Unknown".into(),status:"missing".into(),source:"Unknown".into(),location:String::new(),files:vec![],dependencies:vec![],role:"Unavailable reference".into(),control_reason:"The reference was retained. Rescan or edit this collection to reconcile it.".into(),mod_id:None,preview:None}})}).collect::<Vec<_>>();
            let unique_files = members
                .iter()
                .flat_map(|m| m.files.iter().map(|f| file_key(f)))
                .collect::<HashSet<_>>()
                .len();
            collections.push(Collection {
                id,
                name: custom.name,
                description: custom.description,
                category: custom.category,
                automatic: false,
                relationship:
                    "User-selected references; ownership remains with original mod records".into(),
                status: aggregate_status(members.iter().map(|m| m.status.as_str())),
                preview: visual.preview,
                members,
                unique_files,
                missing_members: missing,
            });
        }
        output.collections = collections;
        output.available_members = available.into_values().collect();
        output
            .suits
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(output)
    }
    fn suit_component(&self, suit: &Suit) -> Component {
        Component {
            reference: MemberRef {
                kind: "suit".into(),
                id: suit.id.clone(),
            },
            name: suit.name.clone(),
            category: suit.category.clone(),
            status: suit.status.clone(),
            source: suit.source.clone(),
            location: suit.files.first().cloned().unwrap_or_default(),
            files: suit.files.clone(),
            dependencies: suit.dependencies.clone(),
            role: if suit.manageable {
                "Independently manageable suit"
            } else {
                "Not safely manageable individually"
            }
            .into(),
            control_reason: suit.control_reason.clone(),
            mod_id: suit.mod_id.clone(),
            preview: suit.preview.clone(),
        }
    }
    pub fn save_collection(
        &self,
        id: Option<&str>,
        name: &str,
        description: &str,
        category: &str,
        members: &[MemberRef],
    ) -> Result<String> {
        let name = clean_name(name)?;
        let category = clean_name(category)?;
        if description.len() > 2000 || members.len() > 500 {
            bail!("Collection exceeds the metadata/member budget");
        }
        let snapshot = self.organization_snapshot()?;
        if let Some(id) = id {
            if !snapshot
                .collections
                .iter()
                .any(|c| c.id == id && !c.automatic)
            {
                bail!("Only custom collections can be edited");
            }
        } else if snapshot.collections.iter().filter(|c| !c.automatic).count() >= 100 {
            bail!("Custom collection limit reached");
        }
        if snapshot.collections.iter().any(|c| {
            !c.automatic && Some(c.id.as_str()) != id && c.name.eq_ignore_ascii_case(&name)
        }) {
            bail!("A custom collection already has this name");
        }
        let valid: HashSet<_> = snapshot
            .available_members
            .iter()
            .map(|c| c.reference.clone())
            .chain(
                snapshot
                    .collections
                    .iter()
                    .filter(|c| Some(c.id.as_str()) == id)
                    .flat_map(|c| c.members.iter().map(|m| m.reference.clone())),
            )
            .collect();
        let mut seen = HashSet::new();
        for member in members {
            if !valid.contains(member) || !seen.insert(member) {
                bail!("Collection references must exist and must not be duplicated");
            }
        }
        let id = id
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let transaction = self.db.unchecked_transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO collections VALUES (?,?)",
            params![
                id,
                serde_json::to_string(&Custom {
                    name,
                    description: description.into(),
                    category
                })?
            ],
        )?;
        transaction.execute(
            "DELETE FROM collection_members WHERE collection_id=?",
            [&id],
        )?;
        for member in members {
            transaction.execute(
                "INSERT INTO collection_members VALUES (?,?,?)",
                params![id, member.kind, member.id],
            )?;
        }
        transaction.commit()?;
        Ok(id)
    }
    pub fn remove_collection(&self, id: &str) -> Result<()> {
        if self
            .db
            .execute("DELETE FROM collections WHERE id=?", [id])?
            != 1
        {
            bail!("Custom collection not found; automatic relationships cannot be removed");
        }
        Ok(())
    }
    fn check_visual_target(&self, kind: &str, id: &str) -> Result<()> {
        let snapshot = self.organization_snapshot()?;
        if (kind == "collection" && snapshot.collections.iter().any(|c| c.id == id))
            || (kind == "suit" && snapshot.suits.iter().any(|s| s.id == id))
            || (kind == "managed"
                && snapshot
                    .available_members
                    .iter()
                    .any(|c| c.reference.kind == kind && c.reference.id == id))
        {
            Ok(())
        } else {
            bail!("Preview target does not exist")
        }
    }
    pub fn assign_preview(&self, kind: &str, id: &str, path: Option<&Path>) -> Result<()> {
        self.check_visual_target(kind, id)?;
        let mut visual = self.visual(kind, id)?;
        visual.preview = path
            .map(|path| previews::import(&self.data, path))
            .transpose()?;
        self.write_visual(kind, id, &visual)
    }
    pub fn rename_suit(&self, id: &str, name: &str, category: &str) -> Result<()> {
        self.check_visual_target("suit", id)?;
        if !SUIT_CATEGORIES.contains(&category) {
            bail!("Unknown suit category");
        }
        let mut visual = self.visual("suit", id)?;
        visual.name = Some(clean_name(name)?);
        visual.category = Some(category.into());
        self.write_visual("suit", id, &visual)
    }
    pub fn read_preview(&self, key: &str) -> Result<Vec<u8>> {
        previews::read(&self.data, key)
    }
    pub fn prepare_suit(&mut self, id: &str, operation: &str) -> Result<Plan> {
        if !["enable", "disable"].contains(&operation) {
            bail!("Suit management supports reviewed enable/disable only");
        }
        let suit = self
            .organization_snapshot()?
            .suits
            .into_iter()
            .find(|s| s.id == id)
            .context("Suit not found")?;
        if !suit.manageable {
            bail!("{}", suit.control_reason);
        }
        let mod_id = suit.mod_id.context("No independently owned package")?;
        let record = self.record(&mod_id)?;
        let mut plan = self.prepare(&mod_id, operation)?;
        plan.independent_scope = Some(suit.name.clone());
        let (_, owners, _) = self.scan_context()?;
        let inventory = scanner::scan(&plan.game, &owners)?;
        if !inventory.complete {
            plan.blockers.push(
                "A complete fresh scan is required before toggling an independent suit".into(),
            );
        }
        let known: HashSet<_> = record
            .files
            .iter()
            .map(|f| f.target.to_lowercase())
            .collect();
        let entry = record
            .files
            .iter()
            .find(|f| {
                ["asi", "cs", "vb"]
                    .iter()
                    .any(|e| f.target.to_lowercase().ends_with(&format!(".{e}")))
            })
            .context("Missing standalone suit entrypoint")?;
        let stem = Path::new(&entry.target)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        for file in &record.files {
            let path = target(Path::new(&plan.game.path), &file.target)?;
            if path.exists() && link_count(&open_read_no_follow(&path)?)? != 1 {
                plan.blockers
                    .push(format!("Shared hard-linked suit file: {}", file.target));
            }
        }
        let parent = Path::new(&entry.target).parent().unwrap_or(Path::new(""));
        let parent_path = Path::new(&plan.game.path).join(parent);
        let _guard = guard_existing_tree(&parent_path)?;
        for other in fs::read_dir(parent_path)? {
            let other = other?;
            let name = other.file_name().to_string_lossy().to_string();
            let rel = if parent.as_os_str().is_empty() {
                name.clone()
            } else {
                format!("{}/{}", parent.to_string_lossy().replace('\\', "/"), name)
            };
            if !known.contains(&rel.to_lowercase())
                && (name.eq_ignore_ascii_case(&stem)
                    || Path::new(&name)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&stem))
            {
                plan.blockers.push(format!(
                    "Unlisted companion/resource has ambiguous ownership: {rel}"
                ));
            }
        }
        for other in inventory
            .items
            .iter()
            .filter(|i| !known.contains(&i.location.to_lowercase()))
        {
            if scanner::entrypoint(&other.location).is_some_and(|(p, _, e)| {
                ["asi", "dll", "cs", "vb"].contains(&e.as_str())
                    && Path::new(&p)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&stem)
            }) {
                plan.blockers
                    .push(format!("Potential shared suit loader: {}", other.location));
            }
        }
        plan.warnings.push("Changes cover this entire standalone suit package, including every listed configuration file. This changes installation state; it does not equip a suit inside GTA V.".into());
        let pending = self
            .pending
            .get_mut(&plan.token)
            .context("Suit plan expired")?;
        pending.public = plan.clone();
        pending.suit_id = Some(id.into());
        Ok(plan)
    }
}
