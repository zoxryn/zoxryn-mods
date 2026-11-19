import { test, expect } from '@playwright/test';
import { bridge } from './fixtures/organization-bridge';

test('missing custom collection members can be removed individually without file operations', async ({
  page,
}) => {
  await bridge(page);
  await page.addInitScript(() => {
    const org = (window as any).organizationState;
    const base = org.collections[0];
    org.collections.push({
      ...base,
      id: 'missing-custom',
      name: 'Old Collection',
      automatic: false,
      members: [
        {
          ...base.members[0],
          reference: { kind: 'managed', id: 'missing-record' },
          name: 'Missing old mod',
          status: 'missing',
          role: 'Missing referenced member',
          mod_id: null,
          files: [],
        },
      ],
      missing_members: 1,
    });
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'My Mods', exact: true }).click();
  await page.getByRole('button', { name: 'View collection Old Collection' }).click();
  await page.getByRole('button', { name: 'Edit collection', exact: true }).click();
  const editor = page.getByRole('dialog', { name: 'Edit collection' });
  await editor.getByRole('checkbox', { name: 'Include missing member missing-record' }).uncheck();
  await editor.getByRole('button', { name: 'Save collection' }).click();
  expect(
    await page.evaluate(
      () => (window as any).calls.findLast((c: any) => c.cmd === 'save_collection').args.members,
    ),
  ).toEqual([]);
  expect(
    await page.evaluate(() => (window as any).calls.some((c: any) => c.cmd === 'apply_plan')),
  ).toBe(false);
});

test('package collections expand into canonical components and preserve individual library navigation', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: /^My Mods/ }).click();
  await expect(page.getByRole('button', { name: 'View collection Hero Collection' })).toBeVisible();
  await page.getByText('Components (2)', { exact: true }).click();
  await expect(
    page.getByRole('article').getByText('Independently manageable suit', { exact: true }),
  ).toBeVisible();
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
  await page.getByRole('button', { name: /^My Mods/ }).click();
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
  await expect(
    page.getByRole('button', { name: 'View suit Resource: unknown_uppr.ydd' }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'Unidentified Files', exact: true }).click();
  await page.getByRole('button', { name: /unknown_uppr.ydd/ }).click();
  const detail = page.getByRole('complementary', { name: 'Unidentified file details' });
  await expect(detail).toContainText('Shared RPF contents');
  await expect(detail.getByRole('button', { name: 'Review disable suit' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
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
test('assigned local preview is lazy-loaded and label editing preserves control availability', async ({
  page,
}) => {
  await bridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Suits & Skins', exact: true }).click();
  await page.getByRole('button', { name: 'View suit Solar Suit' }).click();
  const detail = page.getByRole('complementary', { name: 'Organization details' });
  await detail.getByRole('button', { name: 'Assign preview' }).click();
  await detail.locator('.org-thumbnail').scrollIntoViewIfNeeded();
  await expect(
    detail.getByRole('img', { name: 'Solar Suit — assigned preview', exact: true }),
  ).toBeVisible();
  await detail.getByRole('button', { name: 'Edit label/category' }).click();
  const edit = page.getByRole('dialog', { name: 'Edit suit details' });
  await edit.getByLabel('Suit name', { exact: true }).fill('My Unverified Outfit');
  await edit.getByLabel('Suit editor category').selectOption('Civilian Outfits');
  await edit.getByRole('button', { name: 'Save suit details' }).click();
  await expect(detail).toContainText('My Unverified Outfit');
  await expect(detail.getByRole('button', { name: 'Review disable suit' })).toBeEnabled();
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
  await expect(page.getByText('Superhero Suits · original placeholder').first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.screenshot({ path: 'test-results/suits-synthetic-minimum.png', fullPage: true });
  await page.setViewportSize({ width: 1320, height: 880 });
  await page.getByRole('button', { name: 'View suit Solar Suit' }).click();
  await page.screenshot({ path: 'test-results/suits-synthetic-details.png', fullPage: true });
  await page.getByRole('button', { name: /^My Mods/ }).click();
  await page.getByRole('button', { name: 'View collection Hero Collection' }).click();
  await page.screenshot({ path: 'test-results/collections-synthetic-details.png', fullPage: true });
});
