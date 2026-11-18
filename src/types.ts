export interface Prerequisite {
  id: string;
  name: string;
  present: boolean;
  detail: string;
  url: string;
}
export interface Game {
  path: string;
  edition: string;
  launcher: string;
  version: string | null;
  prerequisites: Prerequisite[];
  warnings: string[];
}
export interface ModFile {
  archive_entry: string;
  target: string;
  hash: string;
  size: number;
}
export interface Mod {
  id: string;
  name: string;
  author: string;
  version: string;
  category: string;
  source: string;
  imported_at: number;
  installed_at: number | null;
  status: 'imported' | 'enabled' | 'disabled';
  archive_path: string;
  archive_hash: string;
  kind: string;
  edition: string;
  requirements: string[];
  supported_builds: string[];
  game_path?: string | null;
  origin?: string;
  adopted_from?: string | null;
  warnings: string[];
  instructions: string;
  files: ModFile[];
}
export interface Backup {
  id: string;
  mod_name: string;
  operation: string;
  created_at: number;
  status: string;
  files: number;
  bytes: number;
}
export interface Snapshot {
  game: Game | null;
  mods: Mod[];
  backups: Backup[];
  recovery: string[];
  data_path: string;
  storage_bytes: number;
  archive_bytes: number;
  backup_bytes: number;
  inventory?: ScanInventory | null;
}
export interface ScannedFile {
  path: string;
  size: number;
  modified_ns: string;
}
export interface RpfSummary {
  state: string;
  entries: number;
  bytes_read: number;
  categories: Record<string, number>;
  samples: string[];
  notes: string[];
}
export interface DiscoveredMod {
  id: string;
  game_path: string;
  name: string;
  category: string;
  status: string;
  source: string;
  confidence: string;
  kind: string;
  location: string;
  files: ScannedFile[];
  dependencies: string[];
  conflicts: string[];
  notes: string[];
  adoptable: boolean;
  control_reason: string;
  managed_id: string | null;
  rpf: RpfSummary | null;
}
export interface ManagedInstalled {
  id: string;
  name: string;
  category: string;
  status: string;
  source: string;
  files: string[];
  dependencies: string[];
  notes: string[];
  adopted: boolean;
}
export interface ScanInventory {
  game_path: string;
  scanned_at: number;
  elapsed_ms: number;
  visited_files: number;
  visited_entries: number;
  bytes_read: number;
  hashed_bytes: number;
  complete: boolean;
  stale: boolean;
  skipped: number;
  warnings: string[];
  items: DiscoveredMod[];
  managed: ManagedInstalled[];
}
export interface Plan {
  token: string;
  mod_id: string;
  operation: string;
  game: Game;
  files: {
    target: string;
    size: number;
    action: string;
    before_hash: string | null;
    after_hash: string | null;
  }[];
  blockers: string[];
  warnings: string[];
}
export interface Report {
  id: string;
  message: string;
  files: string[];
}
