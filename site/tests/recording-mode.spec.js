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
      await image.evaluate(el => el.complete ? undefined : new Promise((resolve, reject) => {
        el.addEventListener('load', resolve, { once: true });
        el.addEventListener('error', () => reject(new Error('recording failed to load')), { once: true });
      }));
      expect(await image.evaluate(el => el.naturalWidth)).toBeGreaterThan(0);
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

test('every recording describes what it shows in both modes', async ({ page }) => {
  for (const path of ['/#widgets', '/quickstart.html#try-modes']) {
    await page.goto(path);
    for (const mode of ['Cell', 'Graphite']) {
      await page.getByRole('radio', { name: mode, exact: true }).check();
      for (const image of await page.locator('.mode-recording img').all()) {
        const alt = await image.getAttribute('alt');
        expect(alt.length, alt).toBeGreaterThan(150);
      }
    }
  }
});

test('the activity monitor stays readable on a narrow screen', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/#widgets');
  const scroller = page.locator('.mode-stage .mode-scroll');
  const image = scroller.locator('img');
  await image.scrollIntoViewIfNeeded();
  await image.evaluate(el => el.complete ? undefined : new Promise((resolve, reject) => {
    el.addEventListener('load', resolve, { once: true });
    el.addEventListener('error', () => reject(new Error('recording failed to load')), { once: true });
  }));
  const shown = await image.evaluate(el => el.getBoundingClientRect().width / el.naturalWidth);
  expect(shown).toBeGreaterThanOrEqual(1);
  expect(await scroller.evaluate(el => el.scrollWidth > el.clientWidth)).toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
  await scroller.focus();
  const scrolled = scroller.evaluate(el => new Promise(resolve => {
    el.addEventListener('scroll', () => resolve(el.scrollLeft), { once: true });
  }));
  await page.keyboard.press('ArrowRight');
  expect(await scrolled).toBeGreaterThan(0);
  await page.setViewportSize({ width: 1280, height: 900 });
  expect(await scroller.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
});
