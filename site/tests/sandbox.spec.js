// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { expect, test } from '@playwright/test';

test('the live quickstart loads, accepts input, and resizes', async ({ page }) => {
  await page.goto('/quickstart.html');
  const panel = page.locator('[data-sandbox]');
  const accessibleRows = panel.locator('.xterm-accessibility-tree');

  await expect(panel.locator('.xterm')).toBeVisible();
  await expect(panel.locator('[data-sandbox-fallback]')).toBeHidden();
  await expect(accessibleRows).toContainText('Hello. Press q to quit.');

  const directFormFrame = await page.evaluate(async () => {
    const wasm = await import('/assets/sandbox/lunatui_web.js');
    await wasm.default();
    const session = new wasm.Session('form', 56, 12);
    const frame = new TextDecoder().decode(session.tick(0));
    session.free();
    return frame;
  });
  expect(directFormFrame).toContain('Ada');

  const formButton = panel.locator('[data-sandbox-template="form"]');
  await formButton.click();
  await expect(formButton).toHaveAttribute('aria-pressed', 'true');
  await expect(panel.locator('[data-sandbox-status]')).toHaveText(/Live form template/);
  await expect(accessibleRows).toContainText('Ada');
  await page.keyboard.type('Z');
  await expect(accessibleRows).toContainText('AdaZ');

  const originalColumns = await panel.getAttribute('data-cols');
  await page.setViewportSize({ width: 700, height: 800 });
  await expect.poll(() => panel.getAttribute('data-cols')).not.toBe(originalColumns);
  await expect(accessibleRows).toContainText('AdaZ');
});

test('the static frame works without scripts', async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto('/quickstart.html');

  await expect(page.locator('[data-sandbox-fallback]')).toBeVisible();
  await expect(page.locator('[data-sandbox-terminal]')).toBeHidden();
  await context.close();
});
