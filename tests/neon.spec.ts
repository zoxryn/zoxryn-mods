import { test, expect } from '@playwright/test';
import { neonBridge } from './fixtures/neon-bridge';
import { bridge } from './fixtures/organization-bridge';
import { writeFile } from 'node:fs/promises';

test('category hints work with keyboard and selection stays visible in compact windows', async ({
  page,
}) => {
  await neonBridge(page);
  await page.setViewportSize({ width: 1000, height: 700 });
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'My Mods', exact: true }).click();
  const police = page.getByRole('button', { name: 'Show Police & LSPDFR', exact: true });
  await police.focus();
  await expect(police).toHaveAccessibleDescription('Police & LSPDFR');
  expect(await police.getAttribute('aria-describedby')).not.toMatch(/\s/);
  await expect(page.getByRole('tooltip', { name: 'Police & LSPDFR', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('tooltip', { name: 'Police & LSPDFR', exact: true })).toHaveCount(0);
  await police.click();
  await expect(police).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.selected-category')).toHaveText('Category: Police & LSPDFR');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('character browsing expands only declared outfit components and preserves safe restrictions', async ({
  page,
}, info) => {
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    const w = window as any;
    const first = w.organizationState.suits.find((s: any) => s.parent === 'Hulk Mod');
    w.organizationState.suits.push({
      ...first,
      id: 'hulk-second',
      name: 'Hulk · Tactical',
      files: [
        'scripts/HULK v3 Files/Suits/Tactical.ini',
        'scripts/HULK v3 Files/Models/Tactical.ydd',
      ],
    });
  });
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  const group = page.getByRole('group', { name: 'Characters and parent packages' });
  await group.getByRole('button', { name: 'Browse Hulk Mod', exact: true }).click();
  await expect(page.getByRole('article')).toHaveCount(2);
  await page.getByRole('button', { name: 'View suit Hulk · Tactical', exact: true }).click();
  const card = page
    .getByRole('article')
    .filter({ has: page.getByRole('button', { name: 'View suit Hulk · Tactical', exact: true }) });
  await card.getByText('Outfit components (2)', { exact: true }).click();
  await expect(card.locator('.file-groups')).toContainText('Suits & ped models');
  const inspector = page.getByRole('complementary', { name: 'Organization details' });
  await expect(
    inspector.getByRole('button', { name: 'Review disable suit', exact: true }),
  ).toBeDisabled();
  await expect(inspector.locator('#suit-management-reason')).toContainText(
    'ownership has not been reviewed',
  );
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => /execute|prepare_suit/.test(c.cmd)),
    ),
  ).toBe(false);
  await page.screenshot({
    path: info.outputPath('focused-character-components.png'),
    fullPage: true,
  });
});

test('category placeholders are original and assigned local previews still load', async ({
  page,
}, info) => {
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    const w = window as any;
    w.organizationState.collections[0].preview = 'verified-local-art';
  });
  await page.getByRole('button', { name: 'My Mods', exact: true }).click();
  await page
    .getByRole('button', { name: 'View collection Spider-Man Mod', exact: true })
    .scrollIntoViewIfNeeded();
  await expect(
    page
      .getByRole('button', { name: 'View collection Spider-Man Mod', exact: true })
      .getByRole('img'),
  ).toBeVisible();
  await expect(page.locator('[data-placeholder-category="police"]').first()).toBeVisible();
  await expect(page.locator('[data-placeholder-category="vehicles"]').first()).toBeVisible();
  await page.getByRole('button', { name: 'Show Police & LSPDFR', exact: true }).focus();
  await page.screenshot({ path: info.outputPath('focused-category-artwork-tooltips.png') });
});

test('detected state explains that presence does not prove enabled or compatibility', async ({
  page,
}) => {
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    (window as any).organizationState.collections[0].status = 'unknown';
  });
  await page.getByRole('button', { name: 'My Mods', exact: true }).click();
  const card = page.getByRole('button', { name: 'View collection Spider-Man Mod', exact: true });
  await expect(card.locator('.badge')).toHaveText('Detected');
  await expect(card.locator('.badge')).toHaveAttribute(
    'aria-label',
    /enabled state and compatibility are unverified/,
  );
  await card.click();
  await expect(page.getByRole('complementary', { name: 'Organization details' })).toContainText(
    'Inferred grouping never establishes ownership',
  );
});

test('every redesigned tab remains navigable and captures synthetic page evidence', async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 1674, height: 941 });
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(
    () =>
      new Promise<void>((resolve, reject) => {
        const image = new Image();
        image.onload = () => resolve();
        image.onerror = () => reject(Error('Original skyline failed to load'));
        image.src = '/neon-skyline.svg';
      }),
  );
  const tabs = [
    'Overview',
    'My Mods',
    'Installed Mods',
    'Suits & Skins',
    'Unidentified Files',
    'Discover mods',
    'Game & dependencies',
    'Backups & recovery',
    'Settings',
  ];
  for (const [i, name] of tabs.entries()) {
    await page.getByRole('button', { name, exact: true }).click();
    await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: info.outputPath(
        `${String(i + 1).padStart(2, '0')}-${name.replace(/[^a-z0-9]+/gi, '-')}.png`,
      ),
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  }
  await page.getByRole('button', { name: 'Installed Mods', exact: true }).click();
  await page.getByRole('tab', { name: 'Advanced File Inspector' }).click();
  await expect(page.getByRole('button', { name: 'Rescan', exact: true })).toBeVisible();
  await page.screenshot({ path: info.outputPath('10-Advanced-File-Inspector.png') });
});
test('parent groups support grid list filters global search details and virtual corrections', async ({
  page,
}) => {
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'My Mods', exact: true }).click();
  await expect(page.getByRole('article')).toHaveCount(12);
  await page.getByRole('button', { name: 'List view', exact: true }).click();
  await expect(page.locator('.org-items')).toHaveClass(/list/);
  await page.getByRole('button', { name: 'Grid view', exact: true }).click();
  await page.getByLabel('Collection category', { exact: true }).selectOption('Police & LSPDFR');
  await expect(page.getByRole('article')).toHaveCount(4);
  await page.getByLabel('Police package section').selectOption('Police Callout Packs');
  await expect(page.getByRole('article')).toHaveCount(2);
  await page.getByLabel('Search collections').fill('Harbor');
  await expect(page.getByRole('article')).toHaveCount(1);
  await page.getByRole('button', { name: 'View collection Harbor Callouts' }).click();
  const detail = page.getByRole('complementary', { name: 'Organization details' });
  await expect(detail).toContainText('compatibility unverified');
  await expect(detail.getByRole('button', { name: 'Review disable suit' })).toHaveCount(0);
  await detail.getByRole('button', { name: 'Correct virtual folder' }).click();
  const edit = page.getByRole('dialog', { name: 'Edit collection' });
  await edit.getByLabel('Collection name', { exact: true }).fill('Harbor Police Package');
  await edit.getByRole('button', { name: 'Save collection' }).click();
  await page.getByLabel('Global search').fill('Batman');
  await page
    .getByRole('region', { name: 'Global search results' })
    .getByRole('button', { name: /Batman Mod/ })
    .first()
    .click();
  await expect(detail).toContainText('Batman Mod');
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) =>
        ['apply_plan', 'prepare_operation', 'prepare_suit'].includes(c.cmd),
      ),
    ),
  ).toBe(false);
});
test('unknown resource identity confirmation never grants safe file control', async ({ page }) => {
  await bridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Unidentified Files', exact: true }).click();
  await page.getByRole('button', { name: /unknown_uppr.ydd/ }).click();
  const detail = page.getByRole('complementary', { name: 'Unidentified file details' });
  await detail.getByRole('button', { name: 'Confirm an outfit identity' }).click();
  await detail.getByLabel('Confirmed outfit name').fill('Documented Custom Outfit');
  await expect(detail.getByRole('button', { name: 'Save identified outfit' })).toBeDisabled();
  await detail.getByRole('checkbox').check();
  await detail.getByRole('button', { name: 'Save identified outfit' }).click();
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await page.getByRole('button', { name: 'View suit Documented Custom Outfit' }).click();
  await expect(page.getByRole('complementary', { name: 'Organization details' })).toContainText(
    'cannot be safely isolated',
  );
  await expect(page.getByRole('button', { name: 'Review disable suit' })).toBeDisabled();
});
test('appearance respects OS and explicit reduced motion and keyboard search navigation', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByLabel('Glow strength').selectOption('quiet');
  await page.getByLabel('Reduced motion', { exact: true }).selectOption('reduced');
  await expect(page.locator('html')).toHaveAttribute('data-glow', 'quiet');
  await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduced');
  expect(
    await page.evaluate(() =>
      [...document.querySelectorAll('*')].every(
        (e) => getComputedStyle(e).animationName === 'none',
      ),
    ),
  ).toBe(true);
  await page.getByLabel('Global search').focus();
  await page.keyboard.type('Hulk');
  await page.keyboard.press('Tab');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('complementary', { name: 'Organization details' })).toContainText(
    'Hulk Mod',
  );
  await page.getByLabel('Global search').fill('Batman');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('region', { name: 'Global search results' })).toHaveCount(0);
});
test('compact window keeps navigation inspector and outfit parent filters usable', async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 1000, height: 700 });
  await neonBridge(page);
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await page.getByLabel('Parent character or package').selectOption('Hulk Mod');
  await expect(page.getByRole('article')).toHaveCount(1);
  await page.getByRole('button', { name: 'View suit Hulk · Classic' }).click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: info.outputPath('compact-suits-1000x700.png') });
});

test('large library records bounded rendering and measured interaction latency', async ({
  page,
}, info) => {
  await bridge(page, true);
  const started = performance.now();
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await expect(page.getByRole('article')).toHaveCount(24);
  const navigationMs = performance.now() - started;
  const samples: number[] = [];
  for (let i = 0; i < 8; i++) {
    const start = performance.now();
    await page.getByLabel('Search suits').fill(`Outfit ${String(100 + i).padStart(4, '0')}`);
    await expect(page.getByRole('article')).toHaveCount(1);
    await page.evaluate(
      () =>
        new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        ),
    );
    samples.push(performance.now() - start);
  }
  await writeFile(
    info.outputPath('large-library-performance.json'),
    JSON.stringify(
      {
        fixture:
          '1000 identified synthetic outfits; mocked IPC; headless Edge; Playwright command-to-render timing includes automation overhead',
        navigationMs,
        searchSamplesMs: samples,
        medianSearchMs:
          ([...samples].sort((a, b) => a - b)[3] + [...samples].sort((a, b) => a - b)[4]) / 2,
        renderedCards: await page.getByRole('article').count(),
        thumbnailRequests: await page.evaluate(
          () => (window as any).calls.filter((c: any) => c.cmd === 'read_preview').length,
        ),
      },
      null,
      2,
    ),
  );
});

test('update notification opens the existing signed offer without downloading', async ({
  page,
}, info) => {
  await neonBridge(page);
  await page.addInitScript(() => {
    (window as any).fixtureUpdate = {
      installed_version: '0.1.8',
      configured: true,
      phase: 'available',
      available_version: '0.1.9',
      release_notes: 'Synthetic update offer for UI verification only.',
      size_bytes: 12000000,
      downloaded_bytes: 0,
      total_bytes: null,
      error: null,
      can_install: true,
      eligibility_message: 'Synthetic isolated test',
      approval_token: 'fixture-offer',
      metadata_verified: true,
      last_checked_at: 1700000000,
    };
  });
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await expect(
    page.getByRole('button', { name: 'Application update available 0.1.9' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Application update available 0.1.9' }).click();
  await expect(
    page.getByRole('button', { name: 'Update & Restart', exact: true }).first(),
  ).toBeVisible();
  await page.screenshot({ path: info.outputPath('11-Application-Updates.png') });
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.cmd === 'approve_app_update'),
    ),
  ).toBe(false);
});
