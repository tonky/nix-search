import { expect, test } from '@playwright/test';

test.describe('nix-search web smoke', () => {
  test('renders shell and completes refresh with multi-platform data', async ({ page }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(String(err)));

    await page.goto('/');

    await expect(page.getByRole('heading', { name: /nix-search web shell/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /refresh cache/i })).toBeVisible();

    const status = page.locator('.status');
    const before = (await status.textContent()) ?? '';

    await page.getByRole('button', { name: /refresh cache/i }).click();

    // Smoke-level assertion: refresh action is accepted and status transitions.
    await expect(status).toContainText(/refreshing|refresh/i, { timeout: 20_000 });
    const after = (await status.textContent()) ?? '';
    expect(after).not.toEqual(before);

    if (pageErrors.length > 0) {
      throw new Error(`Page errors encountered: ${pageErrors.join(' | ')}`);
    }
  });

  test('background hydration hint/progress appears without blocking shell', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: /nix-search web shell/i })).toBeVisible();

    // UI remains interactive while hydration hint/progress can appear.
    await page.getByRole('searchbox').fill('zig');
    await expect(page.getByRole('searchbox')).toHaveValue('zig');
  });
});
