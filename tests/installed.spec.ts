import { test, expect, type Page } from '@playwright/test';
async function bridge(page: Page, delayed = false) {
  await page.addInitScript(
    ({ delayed }) => {
      const w = window as any;
      w.isTauri = true;
      w.commands = [];
      const game = {
        path: 'C:/SyntheticLegacy',
        edition: 'Legacy',
        launcher: 'Synthetic test',
        version: null,
        prerequisites: [
          { id: 'shv', name: 'Script Hook V', present: true, detail: 'Synthetic', url: '' },
          { id: 'asi', name: 'ASI Loader', present: true, detail: 'Synthetic', url: '' },
        ],
        warnings: [],
      };
      const base = {
        game_path: game.path,
        category: 'Scripts',
        status: 'enabled',
        source: 'Manual',
        confidence: 'identified',
        kind: 'Loadable component',
        files: [],
        dependencies: ['asi', 'shv'],
        conflicts: [],
        notes: ['Filename is a component clue; authorship is unknown.'],
        adoptable: true,
        control_reason: 'Standalone independent entrypoint. Confirm ownership before adoption.',
        managed_id: null,
        rpf: null,
      };
      const items = [
        {
          ...base,
          id: 'asi',
          name: 'Traffic',
          location: 'Traffic.asi',
          files: [{ path: 'Traffic.asi', size: 22, modified_ns: '100' }],
        },
        {
          ...base,
          id: 'disabled',
          name: 'OldScript',
          location: 'scripts/OldScript.cs.disabled',
          status: 'disabled',
          adoptable: false,
          files: [{ path: 'scripts/OldScript.cs.disabled', size: 12, modified_ns: '100' }],
        },
        {
          ...base,
          id: 'rpf',
          name: 'x64e.rpf · RPF container',
          category: 'Unknown',
          source: 'OpenIV',
          confidence: 'possible',
          status: 'needs_review',
          kind: 'OpenIV RPF container',
          location: 'mods/x64e.rpf',
          adoptable: false,
          control_reason:
            'A container may contain multiple unrelated mods. Safe individual isolation is unavailable; controls are disabled.',
          files: [{ path: 'mods/x64e.rpf', size: 4000000000, modified_ns: '100' }],
          rpf: {
            state: 'Readable TOC',
            entries: 8,
            bytes_read: 512,
            categories: { Vehicles: 2, Characters: 1, Weapons: 1, Maps: 1 },
            samples: ['adder.yft', 'w_pistol.ytd', 'city.ymap', 'mp_m_test.ydd'],
            notes: [
              'Resource names are clues, not individual mod identities or proof of replacement.',
            ],
          },
        },
        {
          ...base,
          id: 'unknown',
          name: 'Mystery',
          category: 'Unknown',
          source: 'Unknown',
          confidence: 'unknown',
          status: 'unknown',
          kind: 'Unidentified file',
          location: 'Mystery.bin',
          adoptable: false,
          files: [{ path: 'Mystery.bin', size: 42, modified_ns: '100' }],
        },
        {
          ...base,
          id: 'loader',
          name: 'ASI Loader (possible)',
          category: 'Unknown',
          kind: 'Shared prerequisite',
          location: 'dinput8.dll',
          adoptable: false,
          control_reason: 'Shared prerequisite required by other mods; adoption is blocked.',
        },
      ];
      let adopted = false;
      let state = 'enabled';
      const mod = () => ({
        id: 'adopted',
        name: 'Traffic',
        author: 'Unknown (external file)',
        version: 'Unknown',
        category: 'Scripts',
        source: 'Existing Manual file',
        origin: 'Manual',
        adopted_from: 'asi',
        game_path: game.path,
        imported_at: 1700000000,
        installed_at: 1700000000,
        status: state,
        archive_path: 'C:/SyntheticData/blobs/backup',
        archive_hash: 'fixture-hash',
        kind: 'Adopted entrypoint',
        edition: 'Legacy',
        requirements: ['asi', 'shv'],
        supported_builds: [],
        warnings: [],
        instructions: 'Independent synthetic entrypoint.',
        files: [
          { archive_entry: 'Traffic.asi', target: 'Traffic.asi', hash: 'fixture-hash', size: 22 },
        ],
      });
      const inventory = () => ({
        game_path: game.path,
        scanned_at: 1700000000,
        elapsed_ms: 17,
        visited_files: 7,
        visited_entries: 12,
        bytes_read: 512,
        hashed_bytes: 0,
        complete: true,
        stale: false,
        skipped: 0,
        warnings: [],
        items: items.filter((i) => !adopted || i.id !== 'asi'),
        managed: adopted
          ? [
              {
                id: 'adopted',
                name: 'Traffic',
                category: 'Scripts',
                status: state,
                source: 'Manual',
                files: ['Traffic.asi'],
                dependencies: ['asi', 'shv'],
                notes: [],
                adopted: true,
              },
            ]
          : [],
      });
      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: any) => {
          w.commands.push(cmd);
          if (cmd === 'snapshot')
            return {
              game,
              mods: adopted ? [mod()] : [],
              backups: [],
              recovery: [],
              data_path: 'C:/SyntheticData',
              storage_bytes: 0,
              archive_bytes: 0,
              backup_bytes: 0,
              inventory: inventory(),
            };
          if (cmd === 'scan_installed') {
            if (delayed)
              await new Promise((resolve) => {
                w.releaseScan = resolve;
              });
            return inventory();
          }
          if (cmd === 'prepare_adoption' || cmd === 'prepare_operation')
            return {
              token: 'reviewed-synthetic-plan',
              mod_id: 'adopted',
              operation: cmd === 'prepare_adoption' ? 'adopt' : args.operation,
              game,
              files: [
                {
                  target: 'Traffic.asi',
                  size: 22,
                  action: 'Verify backup & adopt (game file unchanged)',
                  before_hash: 'fixture-hash',
                  after_hash: 'fixture-hash',
                },
              ],
              blockers: [],
              warnings: [],
            };
          if (cmd === 'apply_plan') {
            w.lastApproval = args;
            adopted = true;
            state = 'enabled';
            return {
              id: 'synthetic-report',
              message: 'Synthetic adoption verified',
              files: ['Traffic.asi'],
            };
          }
          if (cmd === 'detect_games') return [];
          throw new Error(`Unexpected mocked command ${cmd}`);
        },
      };
    },
    { delayed },
  );
}
test('installed inventory distinguishes components, containers, unknown files and protected loaders', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Traffic Scripts', exact: false })).toBeVisible();
  await expect(page.getByText('Unknown files', { exact: true })).toBeVisible();
  await expect(page.getByText('Container cannot be safely isolated')).toBeVisible();
  await page.screenshot({ path: 'test-results/installed-mods-preview.png', fullPage: true });
  await page.getByRole('button', { name: 'Details for installed ASI Loader (possible)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Installed mod details' });
  await expect(dialog).toContainText('Shared prerequisite required by other mods');
  await expect(dialog.getByRole('button', { name: 'Review adoption' })).toBeDisabled();
});
test('search categories source status and sorting inspect actual inventory metadata', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  await page.getByLabel('Search installed mods').fill('adder.yft');
  await page.getByLabel('Installed category').selectOption('Vehicles');
  await page.getByLabel('Installation source').selectOption('OpenIV');
  await page.getByLabel('Installed status').selectOption('needs_review');
  await page.getByLabel('Sort installed mods').selectOption('location');
  await expect(page.locator('.installed-table tbody tr')).toHaveCount(1);
  await expect(page.locator('.installed-table')).toContainText('x64e.rpf');
  await page
    .getByRole('button', { name: 'Details for installed x64e.rpf · RPF container' })
    .click();
  const dialog = page.getByRole('dialog', { name: 'Installed mod details' });
  await expect(dialog).toContainText('Characters: 1');
  await expect(dialog).toContainText('Weapons: 1');
  await expect(dialog).toContainText('not individual mod identities');
  await expect(dialog.getByRole('button', { name: 'Review adoption' })).toBeDisabled();
});
test('adoption needs independent ownership approval and keeps Manual origin after management', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  await page
    .getByRole('row')
    .filter({ hasText: 'Traffic' })
    .getByRole('button', { name: 'Review adoption' })
    .click();
  const dialog = page.getByRole('dialog', { name: 'Review file changes' });
  const submit = dialog.getByRole('button', { name: 'Approve & adopt' });
  await dialog.getByRole('checkbox').nth(1).check();
  await dialog.getByRole('checkbox').nth(2).check();
  await expect(submit).toBeDisabled();
  await dialog.getByRole('checkbox').nth(0).check();
  await submit.click();
  await expect(page.getByRole('dialog', { name: 'Operation report' })).toBeVisible();
  expect(await page.evaluate(() => (window as any).lastApproval)).toEqual({
    token: 'reviewed-synthetic-plan',
    approved: true,
    compatibilityReviewed: true,
    ownershipReviewed: true,
  });
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.installed-table')).toContainText('Manual · managed');
  await expect(page.getByRole('button', { name: 'Review disable', exact: true })).toBeEnabled();
});
test('a pending scan does not block navigation and rescan invokes only read-only scanning', async ({
  page,
}) => {
  await bridge(page, true);
  await page.goto('/');
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Scanning…', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Approval before every change' })).toBeVisible();
  await page.evaluate(() => (window as any).releaseScan());
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Rescan', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Rescan', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Scanning…', exact: true })).toBeVisible();
  await page.evaluate(() => (window as any).releaseScan());
  const commands = await page.evaluate(() => (window as any).commands);
  expect(commands.filter((x: string) => x === 'scan_installed').length).toBe(2);
  expect(commands).not.toContain('apply_plan');
});
test('installed page fits minimum window width and has a bounded table page', async ({ page }) => {
  await bridge(page);
  await page.setViewportSize({ width: 1000, height: 700 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(page.getByText('items · showing up to 50 per page', { exact: false })).toBeVisible();
  await page.screenshot({ path: 'test-results/installed-mods-minimum.png', fullPage: true });
});
