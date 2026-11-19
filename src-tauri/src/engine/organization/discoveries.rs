use super::*;
use crate::grouping;

impl Engine {
    // Only an explicit validated package declaration selects artwork. Never
    // execute SVG/HTML, guess from unrelated files, or follow game-tree links.
    fn local_package_preview(
        &self,
        inventory: &ScanInventory,
        relative: Option<&str>,
        budget: &mut (u64, usize),
    ) -> Result<Option<String>> {
        use std::io::Read;
        let Some(relative) = relative else {
            return Ok(None);
        };
        if inventory.stale || !inventory.complete {
            return Ok(None);
        }
        let path = target(Path::new(&inventory.game_path), relative)?;
        let _guard = guard_existing_tree(path.parent().unwrap())?;
        let file = open_read_no_follow(&path)?;
        let metadata = file.metadata()?;
        if metadata.len() > previews::MAX_IMAGE {
            bail!("Declared artwork exceeds 8 MiB");
        }
        let id =
            hash_bytes(format!("{}:{relative}", inventory.game_path.to_lowercase()).as_bytes());
        let fingerprint = format!("{}:{:?}", metadata.len(), metadata.modified()?);
        let mut cached = self.visual("local_artwork", &id)?;
        if cached.name.as_deref() == Some(&fingerprint) && cached.preview.is_some() {
            return Ok(cached.preview);
        }
        if budget.1 == 0 || metadata.len() + 1 > budget.0 {
            return Ok(None);
        }
        budget.1 -= 1;
        let mut bytes = Vec::new();
        file.take(metadata.len() + 1).read_to_end(&mut bytes)?;
        budget.0 = budget.0.saturating_sub(bytes.len() as u64);
        if bytes.len() as u64 != metadata.len() {
            bail!("Declared artwork changed while reading");
        }
        let key = previews::store(&self.data, &bytes)?;
        cached.name = Some(fingerprint);
        cached.preview = Some(key.clone());
        self.write_visual("local_artwork", &id, &cached)?;
        Ok(Some(key))
    }
    pub(super) fn organize_discoveries(
        &self,
        inventory: &ScanInventory,
        collections: &mut Vec<Collection>,
        available: &mut BTreeMap<String, Component>,
        output: &mut OrganizationSnapshot,
    ) -> Result<()> {
        let hints = grouping::hints(inventory);
        let mut groups = BTreeMap::<String, Collection>::new();
        let mut artwork_budget = (16 * 1024 * 1024_u64, 32);
        let declared_artwork: BTreeMap<_, _> = inventory
            .items
            .iter()
            .filter_map(|i| i.package_evidence.as_ref())
            .map(|p| (p.root.to_lowercase(), p.preview.as_deref()))
            .collect();
        let group_id = |root: &str| {
            format!(
                "folder:{}",
                hash_bytes(
                    format!(
                        "{}:{}",
                        inventory.game_path.to_lowercase(),
                        root.to_lowercase()
                    )
                    .as_bytes()
                )
            )
        };
        for (key, hint) in &hints {
            if *key != hint.root.to_lowercase() {
                continue;
            } // Loader aliases reference one group.
            let id = group_id(key);
            let description = if hint.category == "Police & LSPDFR" || key == "@dependency/rph" {
                format!(
                    "{} · {}",
                    grouping::police_section(&hint.root, &hint.name),
                    hint.evidence
                )
            } else {
                hint.evidence.clone()
            };
            groups.insert(
                key.clone(),
                Collection {
                    id,
                    name: hint.name.clone(),
                    description,
                    category: hint.category.clone(),
                    automatic: true,
                    relationship: hint.evidence.clone(),
                    status: "unknown".into(),
                    preview: self
                        .local_package_preview(
                            inventory,
                            declared_artwork.get(key).copied().flatten(),
                            &mut artwork_budget,
                        )
                        .ok()
                        .flatten(),
                    members: vec![],
                    unique_files: 0,
                    missing_members: 0,
                },
            );
        }
        let existing: HashSet<_> = collections
            .iter()
            .flat_map(|c| c.members.iter().map(|m| m.reference.clone()))
            .collect();
        for item in &inventory.items {
            if item.managed_id.is_some() {
                continue;
            }
            let reference = MemberRef {
                kind: "external".into(),
                id: item.id.clone(),
            };
            let key = format!("external:{}", item.id);
            if let Some(component) = available.get_mut(&key) {
                component.role = if grouping::dependency(&item.location).is_some() {
                    "Detected shared tool; compatibility unverified".into()
                } else {
                    grouping::role(&item.location).into()
                };
                if let Some(hint) = grouping::group_for(&item.location, &hints) {
                    groups
                        .get_mut(&hint.root.to_lowercase())
                        .unwrap()
                        .members
                        .push(component.clone());
                } else if !existing.contains(&reference) {
                    output.unidentified.push(component.clone());
                }
            }
        }
        // Explicit manifests identify outfits; generic model/texture extensions never do.
        let mut named = output.suits.iter().filter(|s| s.mod_id.is_some()).count();
        for item in &inventory.items {
            let Some(package) = &item.package_evidence else {
                continue;
            };
            for declaration in &package.suits {
                if named >= 2048 {
                    output.warnings.push("Named outfit budget reached (2,048); remaining declarations are retained in the raw inspector.".into());
                    break;
                }
                named += 1;
                let id =
                    hash_bytes(format!("external-suit:{}:{}", item.id, declaration.key).as_bytes());
                let visual = self.visual("suit", &id)?;
                let declared_preview = self
                    .local_package_preview(
                        inventory,
                        declaration.preview.as_deref(),
                        &mut artwork_budget,
                    )
                    .ok()
                    .flatten();
                let suit=Suit{id:id.clone(),name:visual.name.unwrap_or_else(||declaration.name.clone()),category:visual.category.unwrap_or_else(||declaration.category.clone()),model:declaration.model.clone(),parent:package.name.clone(),source:item.source.clone(),status:"needs_review".into(),files:declaration.files.clone(),dependencies:package.requirements.clone(),conflicts:item.conflicts.clone(),notes:vec!["Explicit external package declaration with existing bounded file references. Declared identity is not proof of registration, compatibility or in-game operation.".into()],manageable:false,control_reason:"This external outfit is identified for browsing, but its complete installation/registration has not been adopted into supported transactional ownership. Follow the author's instructions; no RPF or registration edit is automated.".into(),mod_id:None,preview:visual.preview.or(declared_preview)};
                let component = self.suit_component(&suit);
                available.insert(format!("suit:{id}"), component.clone());
                if let Some(group) = groups.get_mut(&package.root.to_lowercase()) {
                    group.members.push(component);
                }
                output.suits.push(suit);
            }
        }
        // Nested package groups use references; raw files retain one canonical discovery ID.
        let mut keys: Vec<_> = groups.keys().cloned().collect();
        keys.sort_by_key(|key| std::cmp::Reverse(key.matches('/').count()));
        for key in &keys {
            if key.starts_with('@') {
                continue;
            }
            let mut parent = key.as_str();
            while let Some((next, _)) = parent.rsplit_once('/') {
                if let Some(child) = groups
                    .get(key)
                    .cloned()
                    .filter(|_| groups.contains_key(next))
                {
                    let reference = MemberRef {
                        kind: "collection".into(),
                        id: child.id.clone(),
                    };
                    let component=Component{reference:reference.clone(),name:child.name.clone(),category:child.category.clone(),status:aggregate_status(child.members.iter().map(|m|m.status.as_str())),source:"Detected folder relationship".into(),location:key.clone(),files:child.members.iter().flat_map(|m|m.files.clone()).collect(),dependencies:vec![],role:"Nested package group; virtual reference".into(),control_reason:"Nested grouping does not merge package ownership or permit coordinated file operations.".into(),mod_id:None,preview:child.preview};
                    available.insert(format!("collection:{}", child.id), component.clone());
                    groups.get_mut(next).unwrap().members.push(component);
                    break;
                }
                parent = next;
            }
        }
        // Dependency membership is a reference, never ownership or duplicated file storage.
        let dependencies: BTreeMap<_, _> = inventory
            .items
            .iter()
            .filter_map(|i| {
                grouping::dependency(&i.location).map(|dep| (dep.to_string(), i.id.clone()))
            })
            .collect();
        for group in groups
            .values_mut()
            .filter(|g| g.category != "Tools & Dependencies")
        {
            let mut required: HashSet<String> = group
                .members
                .iter()
                .flat_map(|m| m.dependencies.clone())
                .collect();
            if group.members.iter().any(|m| {
                let path = m.location.to_ascii_lowercase();
                path.starts_with("plugins/lspdfr/")
                    || path.starts_with("lspdfr/")
                    || path == "lspdfr.asi"
            }) {
                required.insert("rph".into());
            }
            for dependency in required {
                if let Some(component) = dependencies
                    .get(&dependency)
                    .and_then(|id| available.get(&format!("external:{id}")))
                {
                    if !group
                        .members
                        .iter()
                        .any(|m| m.reference == component.reference)
                    {
                        let mut member = component.clone();
                        member.role = "Shared dependency reference".into();
                        member.files.clear();
                        group.members.push(member);
                    }
                }
            }
        }
        for (root, group) in &groups {
            available.insert(
                format!("collection:{}", group.id),
                Component {
                    reference: MemberRef {
                        kind: "collection".into(),
                        id: group.id.clone(),
                    },
                    name: group.name.clone(),
                    category: group.category.clone(),
                    status: aggregate_status(group.members.iter().map(|m| m.status.as_str())),
                    source: "Virtual folder".into(),
                    location: root.clone(),
                    files: group
                        .members
                        .iter()
                        .filter(|m| m.role != "Shared dependency reference")
                        .flat_map(|m| m.files.clone())
                        .collect(),
                    dependencies: vec![],
                    role: "Package group reference".into(),
                    control_reason:
                        "Virtual organization only. No ownership or file operation is granted."
                            .into(),
                    mod_id: None,
                    preview: group.preview.clone(),
                },
            );
        }
        collections.extend(groups.into_values());
        let mut issues = Vec::new();
        let mut push = |severity: &str, subject: String, detail: String| {
            if issues.len() < 128 {
                issues.push(HealthIssue {
                    severity: severity.into(),
                    subject,
                    detail,
                });
            }
        };
        if !inventory.complete || inventory.stale {
            push("warning","Scan coverage".into(),"Evidence is incomplete or stale; run a fresh health check. No operation is enabled by a partial scan.".into());
        }
        for item in &inventory.items {
            for conflict in &item.conflicts {
                push("warning", item.name.clone(), conflict.clone());
            }
        }
        for item in &inventory.managed {
            if item.status == "needs_review" {
                for note in &item.notes {
                    push("warning", item.name.clone(), note.clone());
                }
            }
        }
        let game = self.selected_game()?;
        for item in &inventory.items {
            if let Some(package) = &item.package_evidence {
                for dependency in &package.requirements {
                    if !game.as_ref().is_some_and(|g| {
                        g.prerequisites
                            .iter()
                            .any(|p| p.id == *dependency && p.present)
                    }) {
                        push(
                            "warning",
                            package.name.clone(),
                            format!(
                                "Declared dependency not detected or unsupported: {dependency}"
                            ),
                        );
                    }
                }
            }
        }
        for warning in &self.recovery {
            push("error", "Recovery".into(), warning.clone());
        }
        output.health=HealthReport{checked_at:inventory.scanned_at,complete:inventory.complete&&!inventory.stale,issues,compatibility:"File presence and grouping are detected evidence. Runtime compatibility and in-game operation are unverified; current hashes and backups are checked before an approved change.".into()};
        Ok(())
    }
    pub fn mod_health_check(&mut self) -> Result<HealthReport> {
        self.scan_installed()?;
        Ok(self.organization_snapshot()?.health)
    }
    pub fn save_virtual_group(
        &self,
        id: &str,
        name: &str,
        category: &str,
        members: &[MemberRef],
        confirmed: bool,
    ) -> Result<()> {
        let snapshot = self.organization_snapshot()?;
        let original = snapshot
            .collections
            .iter()
            .find(|c| c.id == id && c.automatic)
            .context("Automatic group not found")?;
        if members.len() > 5000 {
            bail!("Virtual group exceeds its reference budget");
        }
        let valid: HashSet<_> = snapshot
            .available_members
            .iter()
            .map(|m| m.reference.clone())
            .chain(original.members.iter().map(|m| m.reference.clone()))
            .collect();
        let mut seen = HashSet::new();
        for member in members {
            if member.kind == "collection"
                && !original.members.iter().any(|m| m.reference == *member)
            {
                bail!("New nested package links cannot be added here; avoid virtual-folder cycles");
            }
            if !valid.contains(member) || !seen.insert(member) {
                bail!("Virtual group references must exist and be unique");
            }
        }
        let mut visual = self.visual("collection", id)?;
        visual.name = Some(clean_name(name)?);
        visual.category = Some(clean_name(category)?);
        visual.members = Some(members.to_vec());
        visual.relationship_confirmed = confirmed;
        self.write_visual("collection", id, &visual)
    }
    pub fn confirm_skin_identity(
        &self,
        id: &str,
        name: &str,
        category: &str,
        confirmed: bool,
    ) -> Result<()> {
        self.check_visual_target("suit", id)?;
        if !SUIT_CATEGORIES.contains(&category) {
            bail!("Choose a supported outfit category");
        }
        let mut visual = self.visual("suit", id)?;
        visual.name = Some(clean_name(name)?);
        visual.category = Some(category.into());
        visual.confirmed_identity = confirmed;
        self.write_visual("suit", id, &visual)
    }
}
