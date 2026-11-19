export const suitCategories = [
  'Superhero Suits',
  'Police Uniforms',
  'Custom Peds',
  'Character Replacements',
  'Civilian Outfits',
  'Other / Unidentified',
] as const;
export interface MemberRef {
  kind: 'managed' | 'external' | 'suit' | 'collection';
  id: string;
}
export interface Component {
  reference: MemberRef;
  name: string;
  category: string;
  status: string;
  source: string;
  location: string;
  files: string[];
  dependencies: string[];
  role: string;
  control_reason: string;
  mod_id: string | null;
  preview: string | null;
}
export interface Collection {
  id: string;
  name: string;
  description: string;
  category: string;
  automatic: boolean;
  relationship: string;
  status: string;
  preview: string | null;
  members: Component[];
  unique_files: number;
  missing_members: number;
}
export interface Suit {
  id: string;
  name: string;
  category: string;
  model: string | null;
  parent: string;
  source: string;
  status: string;
  files: string[];
  dependencies: string[];
  conflicts: string[];
  notes: string[];
  manageable: boolean;
  control_reason: string;
  mod_id: string | null;
  preview: string | null;
}
export interface OrganizationSnapshot {
  collections: Collection[];
  suits: Suit[];
  available_members: Component[];
  warnings: string[];
  candidates?: Suit[];
  unidentified?: Component[];
  health?: HealthReport;
}
export interface HealthReport {
  checked_at: number;
  complete: boolean;
  issues: { severity: string; subject: string; detail: string }[];
  compatibility: string;
}
export const emptyOrganization: OrganizationSnapshot = {
  collections: [],
  suits: [],
  available_members: [],
  warnings: [],
};
