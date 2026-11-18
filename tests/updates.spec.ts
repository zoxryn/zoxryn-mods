import { test, expect, type Page } from '@playwright/test';
async function bridge(page: Page, mode = 'available') {
  await page.addInitScript(
    ({ mode }) => {
      const w = window as any;
      w.isTauri = true;
      w.calls = [];
      let callbackId = 0;
      const callbacks = new Map();
      w.__TAURI_INTERNALS__ = {
        transformCallback: (fn: any) => {
          callbacks.set(++callbackId, fn);
          return callbackId;
        },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (cmd: string, args: any) => {
          w.calls.push({ cmd, args });
          const status = {
            installed_version: '0.1.7',
            configured: mode !== 'not_configured',
            phase: mode,
            available_version: mode === 'available' ? '0.1.8' : null,
            release_notes: 'Verified release notes.\nKeeps your library and backups.',
            size_bytes: mode === 'available' ? 3000000 : null,
            downloaded_bytes: 0,
            total_bytes: mode === 'available' ? 3000000 : null,
            error: mode === 'error' ? 'Update check failed: network unavailable' : null,
            can_install: true,
            eligibility_message: 'Verified installed copy.',
            approval_token: mode === 'available' ? 'verified-offer-token' : null,
            metadata_verified: mode === 'available',
            last_checked_at: 1795000000,
          };
          if (cmd === 'plugin:event|listen') {
            w.updateListener = args.handler;
            return 1;
          }
          if (cmd === 'plugin:event|unlisten') return;
          if (cmd === 'snapshot')
            return {
              game: null,
              mods: [],
              backups: [],
              recovery: [],
              data_path: 'C:/SyntheticData',
              storage_bytes: 0,
              archive_bytes: 0,
              backup_bytes: 0,
              inventory: null,
            };
          if (cmd === 'detect_games') return [];
          if (cmd === 'update_status' || cmd === 'check_app_update') return status;
          if (cmd === 'approve_app_update') {
            w.lastApproval = args;
            const next = { ...status, phase: 'downloading', downloaded_bytes: 1500000 };
            callbacks.get(w.updateListener)?.({ event: 'app-update-status', id: 1, payload: next });
            return;
          }
          throw new Error('Unexpected synthetic command ' + cmd);
        },
      };
    },
    { mode },
  );
}
test('startup checks only metadata and Later never downloads', async ({ page }) => {
  await bridge(page);
  await page.goto('/');
  const prompt = page.getByRole('dialog', { name: 'Application update available' });
  await expect(prompt).toBeVisible();
  await expect(prompt).toContainText('Installed 0.1.7');
  await expect(prompt).toContainText('Available 0.1.8');
  await expect(prompt).toContainText('Verified release notes');
  await page.screenshot({ path: 'test-results/updater-prompt.png' });
  await prompt.getByRole('button', { name: 'Later', exact: true }).click();
  await expect(prompt).not.toBeVisible();
  const calls = await page.evaluate(() => (window as any).calls);
  expect(calls.filter((c: any) => c.cmd === 'check_app_update')).toHaveLength(1);
  expect(calls.some((c: any) => c.cmd === 'approve_app_update')).toBe(false);
});
test('Update & Restart sends explicit approval for the signed offer and displays progress', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  const prompt = page.getByRole('dialog', { name: 'Application update available' });
  await prompt.getByRole('button', { name: 'Update & Restart', exact: true }).click();
  expect(await page.evaluate(() => (window as any).lastApproval)).toEqual({
    token: 'verified-offer-token',
    approved: true,
  });
  await expect(
    prompt.getByRole('progressbar', { name: 'Application update progress' }),
  ).toHaveAttribute('value', '50');
  await expect(prompt.getByRole('button', { name: 'Later', exact: true })).toBeDisabled();
});
test('Settings explains unconfigured feed without claiming an update exists', async ({ page }) => {
  await bridge(page, 'not_configured');
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Application updates', exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Installed v0.1.7', { exact: true })).toBeVisible();
  await expect(page.getByText('Last update check', { exact: true })).toBeVisible();
  await expect(page.getByText('No update server is contacted', { exact: false })).toBeVisible();
  await page.locator('.update-card').screenshot({ path: 'test-results/updater-settings.png' });
  await expect(page.getByRole('button', { name: 'Update & Restart', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Check for Updates', exact: true }).click();
  await expect(
    page.getByText('The release feed is not configured yet.', { exact: false }),
  ).toBeVisible();
});

test('an empty GitHub releases feed keeps the interface usable and offers a later manual check', async ({
  page,
}) => {
  await bridge(page, 'not_published');
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByText('No release published', { exact: true })).toBeVisible();
  await expect(page.getByText(/No stable signed release is published yet/)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Check for Updates', exact: true })).toBeEnabled();
  expect(
    (await page.evaluate(() => (window as any).calls)).some(
      (call: any) => call.cmd === 'approve_app_update',
    ),
  ).toBe(false);
});
test('network failure has a retry button and manual recovery guidance', async ({ page }) => {
  await bridge(page, 'error');
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('network unavailable');
  await expect(page.getByRole('button', { name: 'Check for Updates', exact: true })).toBeEnabled();
  await expect(
    page.getByText('Use the supplied recovery installer', { exact: false }),
  ).toBeVisible();
});

test('Settings Later defers an offer without downloading or restarting', async ({ page }) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Dismiss update for later' }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.locator('.update-card').getByRole('button', { name: 'Later', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Update deferred' })).toBeVisible();
  await expect(
    page.locator('.update-card').getByRole('button', { name: 'Update & Restart', exact: true }),
  ).toBeEnabled();
  expect(
    (await page.evaluate(() => (window as any).calls)).some(
      (call: any) => call.cmd === 'approve_app_update',
    ),
  ).toBe(false);
  await page
    .locator('.update-card')
    .screenshot({ path: 'test-results/github-updates-settings.png' });
});
