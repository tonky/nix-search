import { expect, test } from '@playwright/test';

test.describe('browser console warnings', () => {
  test('does not emit reactive tracking warning on startup', async ({ page }) => {
    const warnings: string[] = [];

    page.on('console', (msg) => {
      if (msg.type() === 'warning' || msg.type() === 'error') {
        warnings.push(msg.text());
      }
    });

    await page.goto('/');
    await expect(page.getByRole('searchbox')).toBeVisible();

    await page.waitForTimeout(800);

    const reactiveWarnings = warnings.filter((w) =>
      w.toLowerCase().includes('outside a reactive tracking context'),
    );

    expect(reactiveWarnings).toEqual([]);
  });
});
