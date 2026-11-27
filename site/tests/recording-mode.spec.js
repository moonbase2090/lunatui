// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { expect, test } from '@playwright/test';

for (const width of [390, 1280]) {
  test('gallery mode and quickstart at ' + width + 'px', async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/#widgets');
    const cell = page.getByRole('radio', { name: 'Cell', exact: true });
    const graphite = page.getByRole('radio', { name: 'Graphite', exact: true });
    await cell.focus();
    await page.keyboard.press('ArrowRight');
    await expect(graphite).toBeChecked();
    await expect(page.locator('#w-form figcaption')).toContainText('Cell fallback');
    await expect(page.locator('#w-sparkline .mode-recording img')).toHaveAttribute('src', /sparkline-graphite.png$/);
    await page.getByLabel('Play recordings').check();
    for (const image of await page.locator('.mode-recording img').all()) {
      await image.scrollIntoViewIfNeeded();
      await expect(image).toHaveAttribute('src', /\.webp$/);
      await expect.poll(() => image.evaluate(el => el.naturalWidth)).toBeGreaterThan(0);
    }
    await page.reload();
    await expect(graphite).toBeChecked();
    await page.goto('/quickstart.html#try-modes');
    await expect(graphite).toBeChecked();
    await cell.check();
    await expect(page.locator('#try-modes img')).toHaveAttribute('src', /monitor-cell.png$/);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  });
}
