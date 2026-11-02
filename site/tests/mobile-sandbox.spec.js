// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { expect, test } from '@playwright/test';

async function expectTerminalToFit(panel) {
  const edges = await panel.locator('[data-sandbox-terminal]').evaluate((host) => {
    const screen = host.querySelector('.xterm-screen');
    const hostBox = host.getBoundingClientRect();
    const hostStyle = getComputedStyle(host);
    const pixels = (value) => Number.parseFloat(value) || 0;
    return {
      screenRight: screen.getBoundingClientRect().right,
      hostContentRight: hostBox.right
        - pixels(hostStyle.borderRightWidth)
        - pixels(hostStyle.paddingRight),
    };
  });
  expect(edges.screenRight).toBeLessThanOrEqual(edges.hostContentRight + 0.5);
}

async function expectInputProxyToBeHidden(input) {
  const size = await input.evaluate((element) => {
    const box = element.getBoundingClientRect();
    return { width: box.width, height: box.height };
  });
  expect(size.width).toBeLessThanOrEqual(1);
  expect(size.height).toBeLessThanOrEqual(1);
}

async function expectControlsAtVisualViewportBottom(page) {
  await expect.poll(() => page.evaluate(() => {
    const controlsBottom = document.querySelector('[data-sandbox-mobile]').getBoundingClientRect().bottom;
    const keysBottom = document.querySelector('[data-sandbox-keys]').getBoundingClientRect().bottom;
    const visualViewportBottom = window.visualViewport.offsetTop + window.visualViewport.height;
    return controlsBottom <= visualViewportBottom + 1 && keysBottom <= visualViewportBottom + 1;
  })).toBe(true);
}

test('phone input, composition, touch keys, and resizing preserve the session', async ({ page }, testInfo) => {
  await page.goto('/quickstart.html');
  const panel = page.locator('[data-sandbox]');
  const rows = panel.locator('.xterm-accessibility-tree');
  const input = panel.locator('[data-sandbox-input]');

  await expect(panel.locator('.xterm')).toBeVisible();
  await panel.locator('[data-sandbox-template="form"]').tap();
  await expect(rows).toContainText('Ada');
  await expectInputProxyToBeHidden(input);
  await expect(input).toHaveAttribute('autocomplete', 'off');
  await expect(input).toHaveAttribute('autocapitalize', 'none');
  await expect(input).toHaveAttribute('autocorrect', 'off');
  await expect(input).toHaveAttribute('spellcheck', 'false');
  await expectTerminalToFit(panel);

  await panel.locator('[data-sandbox-terminal]').tap({ position: { x: 20, y: 20 } });
  await expect(input).toBeFocused();
  await page.keyboard.type('Z');
  await expect(rows).toContainText('AdaZ');

  await page.keyboard.press('Backspace');
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
  await page.keyboard.type('Dev');
  await expect(rows).toContainText('Dev');

  await panel.locator('[data-sandbox-key="ArrowUp"]').tap();
  await page.keyboard.type('Y');
  await expect(rows).toContainText('AdaXY');

  await panel.locator('[data-sandbox-key="Enter"]').tap();
  await expect(input).toBeFocused();

  const originalColumns = await panel.getAttribute('data-cols');
  await page.setViewportSize({ width: 340, height: 600 });
  await input.focus();
  await page.evaluate(() => window.visualViewport?.dispatchEvent(new Event('resize')));
  if (testInfo.project.name === 'android-chrome') {
    await expect(panel).toHaveClass(/sandbox-keyboard-open/);
    await expect(panel.locator('[data-sandbox-mobile]')).toHaveCSS('position', 'fixed');
    await expectControlsAtVisualViewportBottom(page);
  }
  await expect.poll(() => panel.getAttribute('data-cols')).not.toBe(originalColumns);
  await expect(rows).toContainText('AdaXY');
  await expect(rows).toContainText('Dev');
  await expectTerminalToFit(panel);
  expect(await page.evaluate(() => document.body.scrollWidth <= window.innerWidth)).toBe(true);
});

test('iPhone keyboard keeps every touch key above the visual viewport edge', async ({ browserName, page }) => {
  test.skip(browserName !== 'webkit', 'iPhone keyboard layout is a WebKit regression');

  await page.goto('/quickstart.html');
  const panel = page.locator('[data-sandbox]');
  const input = panel.locator('[data-sandbox-input]');
  const controls = panel.locator('[data-sandbox-mobile]');
  const keys = panel.locator('[data-sandbox-keys]');

  await panel.locator('[data-sandbox-template="form"]').tap();
  await expect(input).toBeFocused();
  const viewport = page.viewportSize();
  await page.setViewportSize({ width: viewport.width, height: Math.max(320, viewport.height - 300) });
  await page.evaluate(() => window.visualViewport.dispatchEvent(new Event('resize')));

  await expect(panel).toHaveClass(/sandbox-keyboard-open/);
  await expect(controls).toHaveCSS('position', 'fixed');
  await expectControlsAtVisualViewportBottom(page);
  await expect(keys.locator('button')).toHaveCount(7);
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
