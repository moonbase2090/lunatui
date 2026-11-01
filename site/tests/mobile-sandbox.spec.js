// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { expect, test } from '@playwright/test';

test('phone input, composition, touch keys, and resizing preserve the session', async ({ page }, testInfo) => {
  await page.goto('/quickstart.html');
  const panel = page.locator('[data-sandbox]');
  const rows = panel.locator('.xterm-accessibility-tree');
  const input = panel.locator('[data-sandbox-input]');

  await expect(panel.locator('.xterm')).toBeVisible();
  await panel.locator('[data-sandbox-template="form"]').tap();
  await expect(rows).toContainText('Ada');
  await expect(input).toBeVisible();
  await expect(input).toHaveAttribute('autocomplete', 'off');
  await expect(input).toHaveAttribute('autocapitalize', 'none');
  await expect(input).toHaveAttribute('autocorrect', 'off');
  await expect(input).toHaveAttribute('spellcheck', 'false');

  await input.tap();
  await input.fill('Z');
  await expect(rows).toContainText('AdaZ');

  await input.press('Backspace');
  await expect(rows).toContainText('Ada');
  await expect(rows).not.toContainText('AdaZ');

  await input.evaluate((element) => {
    element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: 'X' }));
    element.value = 'X';
    element.dispatchEvent(new InputEvent('input', {
      bubbles: true,
      data: 'X',
      inputType: 'insertCompositionText',
      isComposing: true,
    }));
    element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: 'X' }));
  });
  await expect(rows).toContainText('AdaX');
  await expect(rows).not.toContainText('AdaXX');

  await panel.locator('[data-sandbox-key="Tab"]').tap();
  await input.fill('Dev');
  await expect(rows).toContainText('Dev');

  await panel.locator('[data-sandbox-key="ArrowUp"]').tap();
  await input.fill('Y');
  await expect(rows).toContainText('AdaXY');

  await panel.locator('[data-sandbox-key="Enter"]').tap();
  await expect(input).toBeFocused();

  const originalColumns = await panel.getAttribute('data-cols');
  await page.setViewportSize({ width: 340, height: 600 });
  await input.focus();
  await page.evaluate(() => window.visualViewport?.dispatchEvent(new Event('resize')));
  if (testInfo.project.name === 'android-chrome') {
    await expect(panel).toHaveClass(/sandbox-keyboard-open/);
  }
  await expect.poll(() => panel.getAttribute('data-cols')).not.toBe(originalColumns);
  await expect(rows).toContainText('AdaXY');
  await expect(rows).toContainText('Dev');
  expect(await page.evaluate(() => document.body.scrollWidth <= window.innerWidth)).toBe(true);
});

test('very narrow screens keep the static fallback', async ({ page }) => {
  await page.setViewportSize({ width: 280, height: 640 });
  await page.goto('/quickstart.html');
  const panel = page.locator('[data-sandbox]');

  await expect(panel.locator('[data-sandbox-fallback]')).toBeVisible();
  await expect(panel.locator('[data-sandbox-terminal]')).toBeHidden();
  await expect(panel.locator('[data-sandbox-status]')).toContainText('Static preview');
  expect(await page.evaluate(() => document.body.scrollWidth <= window.innerWidth)).toBe(true);
});
