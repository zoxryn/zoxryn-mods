import { type Page } from '@playwright/test';
export async function bridge(page: Page, large = false) {
  await page.addInitScript(
    ({ large }) => {
      const w = window as any;
      w.isTauri = true;
      w.calls = [];
      let callback = 0;
      const game = {
        path: 'C:/SyntheticGame',
        edition: 'Legacy',
        launcher: 'Synthetic fixture',
        version: null,
        prerequisites: [
          { id: 'shvdn', name: 'ScriptHookVDotNet', present: true, detail: 'Synthetic', url: '' },
        ],
        warnings: [],
      };
      const mod = {
        id: 'solar-parent',
        name: 'Solar Package',
        author: 'Synthetic fixture',
        version: '1',
        category: 'Characters',
        source: 'Local archive',
        imported_at: 1700000000,
        installed_at: 1700000000,
        status: 'enabled',
        archive_path: 'C:/SyntheticData/package.zip',
        archive_hash: 'fixture',
        kind: 'Script',
        edition: 'Legacy',
        requirements: ['shvdn'],
        supported_builds: [],
        warnings: [],
        instructions: 'Synthetic fixture only',
        files: [
          {
            archive_entry: 'scripts/Solar.cs',
            target: 'scripts/Solar.cs',
            hash: 'fixture',
            size: 10,
          },
        ],
      };
      const main = {
        reference: { kind: 'managed', id: mod.id },
        name: mod.name,
        category: 'Characters',
        status: 'enabled',
        source: 'Zoxryn',
        location: game.path,
        files: ['scripts/Solar.cs'],
        dependencies: ['shvdn'],
        role: 'Managed package',
        control_reason: 'Reviewed canonical ownership',
        mod_id: mod.id,
        preview: null,
      };
      const solar = {
        id: 'solar-suit',
        name: 'Solar Suit',
        category: 'Superhero Suits',
        model: 'player_zero',
        parent: mod.name,
        source: 'Zoxryn',
        status: 'enabled',
        files: ['scripts/Solar.cs'],
        dependencies: ['shvdn'],
        conflicts: [],
        notes: ['Synthetic suit metadata'],
        manageable: true,
        control_reason:
          'Independent script-based suit package; complete package is reviewed. This does not equip a suit.',
        mod_id: mod.id,
        preview: null,
      };
      const unknown = {
        ...solar,
        id: 'archive-suit',
        name: 'Resource: unknown_uppr.ydd',
        category: 'Other / Unidentified',
        model: null,
        parent: 'Shared OpenIV archive',
        source: 'OpenIV',
        status: 'needs_review',
        files: ['mods/update/x64/dlcpacks/fixture/dlc.rpf :: unknown_uppr.ydd'],
        manageable: false,
        control_reason: 'Shared RPF contents and AddonPeds registration cannot be safely isolated.',
        mod_id: null,
      };
      const child = {
        reference: { kind: 'suit', id: solar.id },
        name: solar.name,
        category: solar.category,
        status: 'enabled',
        source: 'Zoxryn',
        location: solar.files[0],
        files: solar.files,
        dependencies: solar.dependencies,
        role: 'Independently manageable suit',
        control_reason: solar.control_reason,
        mod_id: mod.id,
        preview: null,
      };
      const collection = {
        id: 'package:solar-parent',
        name: 'Hero Collection',
        description: 'An explicitly declared package boundary',
        category: 'Superhero Mods',
        automatic: true,
        relationship: 'Imported archive and canonical package ownership',
        status: 'enabled',
        preview: null,
        members: [main, child],
        unique_files: 1,
        missing_members: 0,
      };
      const org = {
        collections: [collection],
        suits: large
          ? Array.from({ length: 1000 }, (_, i) => ({
              ...solar,
              id: `outfit-${i}`,
              name: `Outfit ${String(i).padStart(4, '0')}`,
              preview: String(i).padStart(64, '0'),
            }))
          : [solar],
        candidates: [unknown],
        unidentified: [],
        health: {
          checked_at: 1700000000,
          complete: true,
          issues: [],
          compatibility: 'Compatibility unverified; synthetic fixture only',
        },
        available_members: [main, child],
        warnings: [],
      };
      w.organizationState = org;
      w.__TAURI_INTERNALS__ = {
        transformCallback: () => ++callback,
        unregisterCallback: () => {},
        invoke: async (cmd: string, args: any) => {
          w.calls.push({ cmd, args });
          if (cmd === 'plugin:event|listen') return 1;
          if (cmd === 'plugin:event|unlisten') return;
          if ((cmd === 'check_app_update' || cmd === 'update_status') && w.fixtureUpdate)
            return w.fixtureUpdate;
          if (cmd === 'check_app_update' || cmd === 'update_status')
            return {
              installed_version: '0.1.8',
              configured: true,
              phase: 'up_to_date',
              available_version: null,
              release_notes: '',
              size_bytes: null,
              downloaded_bytes: 0,
              total_bytes: null,
              error: null,
              can_install: true,
              eligibility_message: 'Synthetic installed fixture',
              approval_token: null,
              metadata_verified: true,
              last_checked_at: 1700000000,
            };
          if (cmd === 'snapshot')
            return structuredClone({
              game: w.fixtureGame ?? game,
              mods: w.fixtureMods ?? [mod],
              backups: w.fixtureBackups ?? [],
              recovery: [],
              data_path: 'C:/SyntheticData',
              storage_bytes: 0,
              archive_bytes: 0,
              backup_bytes: 0,
              inventory: null,
            });
          if (cmd === 'scan_installed')
            return {
              game_path: game.path,
              scanned_at: 1700000000,
              elapsed_ms: 1,
              visited_files: 1,
              visited_entries: 1,
              bytes_read: 0,
              hashed_bytes: 0,
              complete: true,
              stale: false,
              skipped: 0,
              warnings: [],
              items: [],
              managed: [],
            };
          if (cmd === 'mod_health_check') return org.health;
          if (cmd === 'save_virtual_group') {
            const group = org.collections.find((c) => c.id === args.id);
            if (group)
              Object.assign(group, {
                name: args.name,
                category: args.category,
                members: args.members.map(
                  (r: any) =>
                    org.available_members.find(
                      (m) => m.reference.kind === r.kind && m.reference.id === r.id,
                    ) ??
                    group.members.find(
                      (m) => m.reference.kind === r.kind && m.reference.id === r.id,
                    ),
                ),
              });
            return;
          }
          if (cmd === 'confirm_skin_identity') {
            const entry = org.candidates.find((s) => s.id === args.id);
            if (entry) {
              Object.assign(entry, { name: args.name, category: args.category });
              org.suits.push(entry);
              org.candidates = org.candidates.filter((s) => s.id !== args.id);
            }
            return;
          }
          if (cmd === 'organization_snapshot') return structuredClone(org);
          if (cmd === 'detect_games') return [];
          if (cmd === 'save_collection') {
            const previous = org.collections.find((c) => c.id === args.id);
            const next = {
              ...collection,
              id: args.id ?? 'custom-created',
              name: args.name,
              description: args.description,
              category: args.category,
              automatic: false,
              relationship: 'User-selected references',
              members: args.members.map((r: any) =>
                org.available_members.find(
                  (m) => m.reference.kind === r.kind && m.reference.id === r.id,
                ),
              ),
              unique_files: 1,
            };
            if (previous) Object.assign(previous, next);
            else org.collections.push(next);
            return next.id;
          }
          if (cmd === 'remove_collection') {
            org.collections = org.collections.filter((c) => c.id !== args.id);
            return;
          }
          if (cmd === 'plugin:dialog|open') return 'C:/SyntheticArtwork/solar.png';
          if (cmd === 'assign_preview') {
            const target =
              args.kind === 'suit'
                ? org.suits.find((s) => s.id === args.id)
                : org.collections.find((c) => c.id === args.id);
            if (target) target.preview = args.path ? 'a'.repeat(64) : null;
            return;
          }
          if (cmd === 'read_preview') {
            const canvas = document.createElement('canvas');
            canvas.width = 2;
            canvas.height = 2;
            const context = canvas.getContext('2d')!;
            context.fillStyle = '#7342a4';
            context.fillRect(0, 0, 2, 2);
            return canvas.toDataURL('image/png');
          }
          if (cmd === 'rename_suit') {
            const suit = org.suits.find((s) => s.id === args.id);
            if (suit) {
              suit.name = args.name;
              suit.category = args.category;
            }
            return;
          }
          if (cmd === 'prepare_suit')
            return {
              token: 'reviewed-suit-token',
              independent_scope: 'Solar Suit',
              mod_id: mod.id,
              operation: args.operation,
              game,
              files: [
                {
                  target: 'scripts/Solar.cs',
                  size: 10,
                  action: args.operation === 'disable' ? 'Remove' : 'Create',
                  before_hash: 'fixture',
                  after_hash: null,
                },
              ],
              blockers: [],
              warnings: ['Complete standalone package; installation state only.'],
            };
          if (cmd === 'apply_plan') {
            w.suitApproval = args;
            mod.status = mod.status === 'enabled' ? 'disabled' : 'enabled';
            solar.status = mod.status;
            collection.status = mod.status;
            child.status = mod.status;
            main.status = mod.status;
            return {
              id: 'synthetic-backup',
              message: 'Synthetic suit transaction completed',
              files: ['scripts/Solar.cs'],
            };
          }
          throw new Error('Unexpected synthetic command ' + cmd);
        },
      };
    },
    { large },
  );
}
