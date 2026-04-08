import { expect, test } from '@playwright/test';

test.describe('storage diagnostics and fallback', () => {
  test('opens diagnostics panel and shows key probe fields', async ({ page }) => {
    await page.goto('/');

    await page.getByRole('button', { name: /storage diagnostics/i }).click();

    await expect(page.getByRole('heading', { name: /storage diagnostics/i })).toBeVisible();
    // Allow slower engines to finish diagnostics probes before asserting fields.
    await expect(page.locator('.diagnostics-grid')).toBeVisible({ timeout: 45_000 });
    await expect(page.locator('.diagnostics-grid')).toContainText('Current origin');
    await expect(page.locator('.diagnostics-grid')).toContainText('StorageManager API');
    await expect(page.locator('.diagnostics-grid')).toContainText('IndexedDB write probe');
  });

  test('falls back to session-only mode when IndexedDB open is blocked', async ({ page }) => {
    await page.addInitScript(() => {
      const original = window.indexedDB;
      if (!original) {
        return;
      }

      const blockedOpen = () => {
        throw new DOMException('IndexedDB blocked for test', 'SecurityError');
      };

      const proxied = new Proxy(original, {
        get(target, prop, receiver) {
          if (prop === 'open') {
            return blockedOpen;
          }
          return Reflect.get(target, prop, receiver);
        },
      });

      Object.defineProperty(window, 'indexedDB', {
        configurable: true,
        get() {
          return proxied as IDBFactory;
        },
      });
    });

    await page.goto('/');

    const status = page.locator('.status');
    await expect(status).toContainText(/browser storage unavailable|session-only/i, {
      timeout: 20_000,
    });

    await page.getByRole('button', { name: /refresh cache/i }).click();
    await expect(status).toContainText(/session-only/i, { timeout: 30_000 });
  });
});
