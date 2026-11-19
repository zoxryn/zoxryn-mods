//! Relationships are references, never a second file ownership ledger.
use crate::safety::relative_name;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SUIT_CATEGORIES: &[&str] = &[
    "Superhero Suits",
    "Police Uniforms",
    "Custom Peds",
    "Character Replacements",
    "Civilian Outfits",
    "Other / Unidentified",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PackageOrganization {
    pub collection_name: Option<String>,
    pub preview: Option<String>,
    pub suits: Vec<SuitDeclaration>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SuitDeclaration {
    pub key: String,
    pub name: String,
    pub category: String,
    pub model: Option<String>,
    pub files: Vec<String>,
    pub standalone: bool,
    pub preview: Option<String>,
}
impl PackageOrganization {
    pub fn validate(&mut self, archive_paths: &[String]) -> Result<()> {
        let all: HashSet<_> = archive_paths.iter().map(|p| p.to_lowercase()).collect();
        if self.suits.len() > 64 {
            bail!("A package can declare at most 64 suits");
        }
        if let Some(name) = &self.collection_name {
            clean_name(name)?;
        }
        let mut keys = HashSet::new();
        let preview = |p: &str| -> Result<()> {
            let p = relative_name(p)?;
            if !all.contains(&p.to_lowercase())
                || !["png", "jpg", "jpeg", "webp"]
                    .contains(&p.rsplit('.').next().unwrap_or("").to_lowercase().as_str())
            {
                bail!("Declared preview must be a PNG, JPEG or WebP file in this archive");
            }
            Ok(())
        };
        if let Some(p) = &self.preview {
            preview(p)?;
        }
        for suit in &mut self.suits {
            if suit.category.is_empty() {
                suit.category = "Other / Unidentified".into();
            }
            clean_name(&suit.name)?;
            if suit.key.is_empty()
                || suit.key.len() > 64
                || !suit
                    .key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                || !keys.insert(suit.key.clone())
            {
                bail!("Suit keys must be unique short identifiers");
            }
            if !SUIT_CATEGORIES.contains(&suit.category.as_str()) {
                bail!("Unknown suit category");
            }
            if suit.model.as_ref().is_some_and(|n| {
                n.is_empty()
                    || n.len() > 100
                    || !n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            }) {
                bail!("Invalid suit model identifier");
            }
            if suit.files.is_empty() || suit.files.len() > 256 {
                bail!("A suit must declare 1 to 256 exact files");
            }
            let mut seen = HashSet::new();
            for file in &mut suit.files {
                *file = relative_name(file)?;
                if !all.contains(&file.to_lowercase()) || !seen.insert(file.to_lowercase()) {
                    bail!("Suit file references must be unique existing archive entries");
                }
            }
            if let Some(p) = &suit.preview {
                preview(p)?;
            }
        }
        Ok(())
    }
}
pub fn clean_name(value: &str) -> Result<String> {
    let v = value.trim();
    if v.is_empty() || v.len() > 120 || v.chars().any(char::is_control) {
        bail!("Use a name of 1 to 120 characters without control characters");
    }
    Ok(v.into())
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct MemberRef {
    pub kind: String,
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub reference: MemberRef,
    pub name: String,
    pub category: String,
    pub status: String,
    pub source: String,
    pub location: String,
    pub files: Vec<String>,
    pub dependencies: Vec<String>,
    pub role: String,
    pub control_reason: String,
    pub mod_id: Option<String>,
    pub preview: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suit {
    pub id: String,
    pub name: String,
    pub category: String,
    pub model: Option<String>,
    pub parent: String,
    pub source: String,
    pub status: String,
    pub files: Vec<String>,
    pub dependencies: Vec<String>,
    pub conflicts: Vec<String>,
    pub notes: Vec<String>,
    pub manageable: bool,
    pub control_reason: String,
    pub mod_id: Option<String>,
    pub preview: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub automatic: bool,
    pub relationship: String,
    pub status: String,
    pub preview: Option<String>,
    pub members: Vec<Component>,
    pub unique_files: usize,
    pub missing_members: usize,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrganizationSnapshot {
    pub collections: Vec<Collection>,
    pub suits: Vec<Suit>,
    pub available_members: Vec<Component>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub candidates: Vec<Suit>,
    #[serde(default)]
    pub unidentified: Vec<Component>,
    #[serde(default)]
    pub health: HealthReport,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthReport {
    pub checked_at: u64,
    pub complete: bool,
    pub issues: Vec<HealthIssue>,
    pub compatibility: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    pub severity: String,
    pub subject: String,
    pub detail: String,
}

pub fn aggregate_status<'a>(statuses: impl IntoIterator<Item = &'a str>) -> String {
    let values: HashSet<_> = statuses.into_iter().collect();
    if values.is_empty() {
        "empty"
    } else if values.contains("needs_review") || values.contains("missing") {
        "needs_review"
    } else if values.contains("unknown") {
        "unknown"
    } else if values.len() > 1 {
        "partial"
    } else {
        *values.iter().next().unwrap()
    }
    .into()
}
pub fn is_character_resource(path: &str) -> bool {
    let n = path.to_lowercase();
    n.ends_with(".ydd")
        || n.ends_with(".ymt")
        || (n.ends_with(".yft")
            && (n.contains("ped")
                || n.contains("player_")
                || n.contains("mp_m_")
                || n.contains("mp_f_")))
        || (n.ends_with(".ytd")
            && (n.contains("cloth")
                || n.contains("uppr_")
                || n.contains("lowr_")
                || n.contains("mp_m_")
                || n.contains("mp_f_")))
}

pub fn parse_ped_metadata(bytes: &[u8]) -> Result<Vec<String>> {
    if bytes.len() > 256 * 1024 {
        bail!("Ped metadata exceeds 256 KiB");
    }
    let text = std::str::from_utf8(bytes)?;
    if text.to_ascii_uppercase().contains("<!DOCTYPE")
        || text.to_ascii_uppercase().contains("<!ENTITY")
    {
        bail!("DTD and entity declarations are not accepted");
    }
    let mut options = roxmltree::ParsingOptions::default();
    options.allow_dtd = false;
    options.nodes_limit = 16000;
    let doc = roxmltree::Document::parse_with_options(text, options)?;
    if doc.root_element().tag_name().name() != "CPedModelInfo__InitDataList" {
        bail!("Unknown ped metadata schema; registry relationships were not assumed");
    }
    let mut models = Vec::new();
    let mut nodes = 0;
    for item in doc.descendants() {
        nodes += 1;
        if nodes > 16000 || item.ancestors().take(66).count() > 65 {
            bail!("Ped metadata structure exceeds its budget");
        }
        if item.has_tag_name("Item") && item.parent().is_some_and(|p| p.has_tag_name("InitDatas")) {
            let name = item
                .children()
                .find(|n| n.has_tag_name("Name"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .trim();
            if name.is_empty()
                || name.len() > 100
                || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                bail!("Invalid model name in ped metadata");
            }
            if models.len() >= 256 {
                bail!("Ped metadata exceeds 256 registered models");
            }
            models.push(name.to_string());
        }
    }
    models.sort();
    models.dedup();
    Ok(models)
}
