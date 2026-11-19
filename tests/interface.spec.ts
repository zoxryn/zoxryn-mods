import { test, expect } from '@playwright/test';

test('dashboard shows honest empty state and browser boundary', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Your modding command center.' })).toBeVisible();
  await expect(page.getByText('Browser preview ·')).toBeVisible();
  await page.getByRole('button', { name: 'Import mod', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Windows desktop app');
  expect(errors).toEqual([]);
  await page.screenshot({ path: 'test-results/browser-overview.png', fullPage: true });
});
test('navigation and library search/filter work without fake mod records', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'My Mods' }).click();
  await page.getByRole('tab', { name: 'Individual mods' }).click();
  await page.getByRole('textbox', { name: 'Search mods' }).fill('vehicle');
  await page.getByLabel('Category', { exact: true }).selectOption('Vehicles');
  await page.getByLabel('Mod status', { exact: true }).selectOption('enabled');
  await expect(page.getByRole('heading', { name: 'No matching mods' })).toBeVisible();
  await page.getByRole('button', { name: 'Backups & recovery', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Every change will have a way back' }),
  ).toBeVisible();
});
test('manual-only formats and provider limits are visible', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Discover mods' }).click();
  await expect(page.getByRole('heading', { name: 'GTA5-Mods.com' })).toBeVisible();
  await expect(page.getByText('7z / RAR', { exact: true })).toBeVisible();
  await expect(page.getByText('OIV / DLC / RPF', { exact: true })).toBeVisible();
  await expect(
    page.getByText('Authorized in-app downloads are planned for Phase 2.', { exact: false }),
  ).toBeVisible();
});
test('safety checklist covers launch, online, and prerequisite limitations', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Help & getting started' }).click();
  const dialog = page.getByRole('dialog', { name: 'Single-player safety checklist' });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('never bypasses anti-cheat');
  await expect(dialog).toContainText('does not launch GTA V');
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
});
test('game connection and settings remain readable at minimum window size', async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 700 });
  await page.goto('/');
  await page.getByRole('button', { name: 'Game & dependencies', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Modding prerequisites' })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Approval before every change' })).toBeVisible();
});
