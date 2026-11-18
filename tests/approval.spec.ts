import { test, expect, type Page } from '@playwright/test';

async function bridge(page: Page, blocked = false) {
  await page.addInitScript(
    ({ blocked }) => {
      const w = window as any;
      w.isTauri = true;
      const mod = {
        id: 'fixture',
        name: 'Synthetic ASI',
        author: 'Test',
        version: '1.0',
        category: 'Gameplay',
        source: 'Synthetic archive',
        imported_at: 1700000000,
        installed_at: null,
        status: 'imported',
        archive_path: 'C:/fixture/archive.zip',
        archive_hash: 'abc',
        kind: 'ASI',
        edition: 'Legacy',
        requirements: ['shv'],
        supported_builds: [],
        warnings: [],
        instructions: 'Synthetic instructions',
        files: [{ archive_entry: 'Fixture.asi', target: 'Fixture.asi', hash: 'abc', size: 10 }],
      };
      const game = {
        path: 'C:/DisposableMockGame',
        edition: 'Legacy',
        launcher: 'Synthetic fixture',
        version: null,
        prerequisites: [],
        warnings: [],
      };
      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: any) => {
          if (cmd === 'snapshot')
            return {
              game,
              mods: [mod],
              backups: [],
              recovery: [],
              data_path: 'C:/fixture/data',
              storage_bytes: 0,
              archive_bytes: 0,
              backup_bytes: 0,
            };
          if (cmd === 'detect_games') return [];
          if (cmd === 'prepare_operation')
            return {
              token: 'exact-plan-token',
              mod_id: mod.id,
              operation: args.operation,
              game,
              files: [
                {
                  target: 'Fixture.asi',
                  size: 10,
                  action: 'Create',
                  before_hash: null,
                  after_hash: 'abc',
                },
              ],
              blockers: blocked ? ['Missing dependency: Script Hook V'] : [],
              warnings: [],
            };
          if (cmd === 'apply_plan') {
            w.lastApproval = args;
            mod.status = 'enabled';
            return {
              id: 'fixture-report',
              message: 'Synthetic operation report',
              files: ['Fixture.asi'],
            };
          }
          throw new Error(`Unexpected command ${cmd}`);
        },
      };
    },
    { blocked },
  );
}

test('install requires both approvals and dispatches the reviewed token', async ({ page }) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Review installation of Synthetic ASI' }).click();
  const dialog = page.getByRole('dialog', { name: 'Review file changes' });
  await expect(dialog).toContainText('Fixture.asi');
  const submit = dialog.getByRole('button', { name: 'Approve & install' });
  await expect(submit).toBeDisabled();
  await dialog.getByRole('checkbox').nth(0).check();
  await expect(submit).toBeDisabled();
  await dialog.getByRole('checkbox').nth(1).check();
  await expect(submit).toBeEnabled();
  await submit.click();
  await expect(page.getByRole('dialog', { name: 'Operation report' })).toBeVisible();
  expect(await page.evaluate(() => (window as any).lastApproval)).toEqual({
    token: 'exact-plan-token',
    approved: true,
    compatibilityReviewed: true,
  });
});
test('dependency blockers prevent submission even with both acknowledgments', async ({ page }) => {
  await bridge(page, true);
  await page.goto('/');
  await page.getByRole('button', { name: 'Review installation of Synthetic ASI' }).click();
  const dialog = page.getByRole('dialog', { name: 'Review file changes' });
  await expect(dialog).toContainText('Missing dependency: Script Hook V');
  await dialog.getByRole('checkbox').nth(0).check();
  await dialog.getByRole('checkbox').nth(1).check();
  await expect(dialog.getByRole('button', { name: 'Approve & install' })).toBeDisabled();
  expect(await page.evaluate(() => (window as any).lastApproval)).toBeUndefined();
});
