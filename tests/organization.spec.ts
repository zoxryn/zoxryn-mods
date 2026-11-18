import { test, expect, type Page } from '@playwright/test';
async function bridge(page: Page, large = false) {
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
        category: 'Superheroes',
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
              ...unknown,
              id: `outfit-${i}`,
              name: `Outfit ${String(i).padStart(4, '0')}`,
              preview: String(i).padStart(64, '0'),
            }))
          : [solar, unknown],
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
              game,
              mods: [mod],
              backups: [],
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
test('package collections expand into canonical components and preserve individual library navigation', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: /^My library/ }).click();
  await expect(page.getByRole('button', { name: 'View collection Hero Collection' })).toBeVisible();
  await page.getByText('Components (2)', { exact: true }).click();
  await expect(page.getByText('Independently manageable suit', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'View collection Hero Collection' }).click();
  await expect(page.getByRole('complementary', { name: 'Organization details' })).toContainText(
    'Imported archive and canonical package ownership',
  );
  await page.getByRole('tab', { name: 'Individual mods' }).click();
  await expect(page.getByRole('textbox', { name: 'Search mods' })).toBeVisible();
});
test('custom collection create rename membership and remove never dispatch game operations', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: /^My library/ }).click();
  await page.getByRole('button', { name: 'New collection' }).click();
  const editor = page.getByRole('dialog', { name: 'Create collection' });
  await editor.getByLabel('Collection name', { exact: true }).fill('Weekend Heroes');
  await editor.getByRole('checkbox').first().check();
  await editor.getByRole('button', { name: 'Save collection' }).click();
  await page.getByRole('button', { name: 'View collection Weekend Heroes' }).click();
  const detail = page.getByRole('complementary', { name: 'Organization details' });
  await detail.getByRole('button', { name: 'Edit collection' }).click();
  const edit = page.getByRole('dialog', { name: 'Edit collection' });
  await edit.getByLabel('Collection name', { exact: true }).fill('Renamed Heroes');
  await edit.getByRole('button', { name: 'Save collection' }).click();
  await expect(detail).toContainText('Renamed Heroes');
  await detail.getByRole('button', { name: 'Remove collection', exact: true }).click();
  await page
    .getByRole('dialog', { name: 'Remove collection' })
    .getByRole('button', { name: 'Remove collection', exact: true })
    .click();
  await expect(page.getByRole('button', { name: 'View collection Renamed Heroes' })).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      (window as any).calls.filter((c: any) =>
        ['apply_plan', 'prepare_operation', 'prepare_suit'].includes(c.cmd),
      ),
    ),
  ).toEqual([]);
});
test('suit filters grid list and right details explain protected OpenIV resources', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await page.getByLabel('Suit category', { exact: true }).selectOption('Other / Unidentified');
  await page.getByRole('button', { name: 'View suit Resource: unknown_uppr.ydd' }).click();
  const detail = page.getByRole('complementary', { name: 'Organization details' });
  await expect(detail).toContainText('Shared RPF contents');
  await expect(detail.getByRole('button', { name: 'Review disable suit' })).toBeDisabled();
  await page.getByRole('button', { name: 'List view', exact: true }).click();
  await expect(page.locator('.org-items')).toHaveClass(/list/);
  await page.getByLabel('Suit category', { exact: true }).selectOption('Superhero Suits');
  await expect(page.getByRole('button', { name: 'View suit Solar Suit' })).toBeVisible();
  await page.getByLabel('Search suits').fill('player_zero');
  await expect(page.getByRole('button', { name: 'View suit Solar Suit' })).toHaveCount(1);
});
test('individual suit control requires the existing explicit operation approvals', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await page.getByRole('button', { name: 'View suit Solar Suit' }).click();
  await page.getByRole('button', { name: 'Review disable suit' }).click();
  const plan = page.getByRole('dialog', { name: 'Review file changes' });
  await expect(plan).toContainText('scripts/Solar.cs');
  const approve = plan.getByRole('button', { name: 'Approve & disable' });
  await expect(approve).toBeDisabled();
  await plan.getByRole('checkbox').nth(0).check();
  await expect(approve).toBeDisabled();
  await plan.getByRole('checkbox').nth(1).check();
  await expect(approve).toBeDisabled();
  await plan.getByRole('checkbox').nth(2).check();
  await approve.click();
  await expect(page.getByRole('dialog', { name: 'Operation report' })).toBeVisible();
  expect(await page.evaluate(() => (window as any).suitApproval)).toEqual({
    token: 'reviewed-suit-token',
    approved: true,
    compatibilityReviewed: true,
    ownershipReviewed: true,
  });
});
test('assigned local preview is lazy-loaded and labeling does not unlock unsafe controls', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await page.getByRole('button', { name: 'View suit Resource: unknown_uppr.ydd' }).click();
  const detail = page.getByRole('complementary', { name: 'Organization details' });
  await detail.getByRole('button', { name: 'Assign preview' }).click();
  await detail.locator('.org-thumbnail').scrollIntoViewIfNeeded();
  await expect(
    detail.getByRole('img', { name: 'Resource: unknown_uppr.ydd — assigned preview', exact: true }),
  ).toBeVisible();
  await detail.getByRole('button', { name: 'Edit label/category' }).click();
  const edit = page.getByRole('dialog', { name: 'Edit suit details' });
  await edit.getByLabel('Suit name', { exact: true }).fill('My Unverified Outfit');
  await edit.getByLabel('Suit editor category').selectOption('Civilian Outfits');
  await edit.getByRole('button', { name: 'Save suit details' }).click();
  await expect(detail).toContainText('My Unverified Outfit');
  await expect(detail.getByRole('button', { name: 'Review disable suit' })).toBeDisabled();
  const assigned = await page.evaluate(() =>
    (window as any).calls.find((c: any) => c.cmd === 'assign_preview'),
  );
  expect(assigned.args.path).toBe('C:/SyntheticArtwork/solar.png');
});
test('a thousand candidates keep DOM bounded and thumbnail requests lazy while search stays responsive', async ({
  page,
}) => {
  await bridge(page, true);
  await page.goto('/');
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await expect(page.locator('.org-card')).toHaveCount(24);
  expect(
    await page.evaluate(
      () => (window as any).calls.filter((c: any) => c.cmd === 'read_preview').length,
    ),
  ).toBeLessThan(24);
  await page.getByLabel('Search suits').fill('Outfit 0999');
  await expect(page.locator('.org-card')).toHaveCount(1);
  await expect(page.getByRole('button', { name: 'View suit Outfit 0999' })).toBeVisible();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Application updates', exact: true }),
  ).toBeVisible();
});
test('collections and suits fit the minimum window and expose no invented preview artwork', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1000, height: 700 });
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await expect(page.getByRole('button', { name: 'View suit Solar Suit' })).toBeVisible();
  await expect(page.getByText('No preview assigned').first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.screenshot({ path: 'test-results/suits-synthetic-minimum.png', fullPage: true });
  await page.setViewportSize({ width: 1320, height: 880 });
  await page.getByRole('button', { name: 'View suit Solar Suit' }).click();
  await page.screenshot({ path: 'test-results/suits-synthetic-details.png', fullPage: true });
  await page.getByRole('button', { name: /^My library/ }).click();
  await page.getByRole('button', { name: 'View collection Hero Collection' }).click();
  await page.screenshot({ path: 'test-results/collections-synthetic-details.png', fullPage: true });
});
