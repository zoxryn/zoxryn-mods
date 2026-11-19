use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    pub id: String,
    pub name: String,
    pub present: bool,
    pub detail: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub path: String,
    pub edition: String,
    pub launcher: String,
    pub version: Option<String>,
    pub prerequisites: Vec<Prerequisite>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    pub archive_entry: String,
    pub target: String,
    pub hash: String,
    pub size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModRecord {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub category: String,
    pub source: String,
    pub imported_at: u64,
    pub installed_at: Option<u64>,
    pub status: String,
    pub archive_path: String,
    pub archive_hash: String,
    pub kind: String,
    pub edition: String,
    pub requirements: Vec<String>,
    pub warnings: Vec<String>,
    pub instructions: String,
    pub files: Vec<ModFile>,
    pub supported_builds: Vec<String>,
    #[serde(default)]
    pub game_path: Option<String>,
    #[serde(default)]
    pub adopted_from: Option<String>,
    #[serde(default = "zoxryn_source")]
    pub origin: String,
    #[serde(default)]
    pub organization: crate::organization::PackageOrganization,
}
fn zoxryn_source() -> String {
    "Zoxryn".into()
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Manifest {
    pub name: String,
    pub author: String,
    pub version: String,
    pub category: String,
    pub source: String,
    pub edition: String,
    pub requirements: Vec<String>,
    pub supported_builds: Vec<String>,
    pub organization: crate::organization::PackageOrganization,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedFile {
    pub target: String,
    pub size: u64,
    pub action: String,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub token: String,
    pub mod_id: String,
    pub operation: String,
    #[serde(default)]
    pub independent_scope: Option<String>,
    pub game: Game,
    pub files: Vec<PlannedFile>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub id: String,
    pub mod_name: String,
    pub operation: String,
    pub created_at: u64,
    pub status: String,
    pub files: usize,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub message: String,
    pub files: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub game: Option<Game>,
    pub mods: Vec<ModRecord>,
    pub backups: Vec<Backup>,
    pub recovery: Vec<String>,
    pub data_path: String,
    pub storage_bytes: u64,
    pub archive_bytes: u64,
    pub backup_bytes: u64,
    pub inventory: Option<ScanInventory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScannedFile {
    pub path: String,
    pub size: u64,
    pub modified_ns: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RpfSummary {
    pub state: String,
    pub entries: u32,
    pub bytes_read: u64,
    pub categories: std::collections::BTreeMap<String, u32>,
    pub samples: Vec<String>,
    pub notes: Vec<String>,
    #[serde(default)]
    pub character_resources: Vec<String>,
    #[serde(default)]
    pub character_resources_truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredMod {
    pub id: String,
    pub game_path: String,
    pub name: String,
    pub category: String,
    pub status: String,
    pub source: String,
    pub confidence: String,
    pub kind: String,
    pub location: String,
    pub files: Vec<ScannedFile>,
    pub dependencies: Vec<String>,
    pub conflicts: Vec<String>,
    pub notes: Vec<String>,
    pub adoptable: bool,
    pub control_reason: String,
    pub managed_id: Option<String>,
    pub rpf: Option<RpfSummary>,
    #[serde(default)]
    pub character_models: Vec<String>,
    #[serde(default)]
    pub package_evidence: Option<crate::grouping::PackageEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedInstalled {
    pub id: String,
    pub name: String,
    pub category: String,
    pub status: String,
    pub source: String,
    pub files: Vec<String>,
    pub dependencies: Vec<String>,
    pub notes: Vec<String>,
    pub adopted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInventory {
    pub game_path: String,
    pub scanned_at: u64,
    pub elapsed_ms: u64,
    pub visited_files: u64,
    pub visited_entries: u64,
    pub bytes_read: u64,
    pub hashed_bytes: u64,
    pub complete: bool,
    pub stale: bool,
    pub skipped: u64,
    pub warnings: Vec<String>,
    pub items: Vec<DiscoveredMod>,
    #[serde(default)]
    pub managed: Vec<ManagedInstalled>,
}
