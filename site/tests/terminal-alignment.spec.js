// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { test, expect } from '@playwright/test';

for (const width of [390, 1280]) {
  test(`quickstart template glyphs share one cell grid at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto('/quickstart.html');
    await page.evaluate(() => document.fonts.ready);

    for (const id of ['minimal', 'list', 'form', 'dashboard', 'popover', 'layout']) {
      const measurements = await page.locator(`#${id} .term`).evaluate((pre) => {
        const style = getComputedStyle(pre);
        const canvas = document.createElement('canvas');
        const context = canvas.getContext('2d');
        context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
        const cellWidth = context.measureText('M').width;
        const origin = pre.getBoundingClientRect().left;
        const walker = document.createTreeWalker(pre, NodeFilter.SHOW_TEXT);
        let column = 0;
        let row = 0;
        let maxDrift = 0;
        let worstCell;
        while (walker.nextNode()) {
          const node = walker.currentNode;
          for (let offset = 0; offset < node.length; offset += 1) {
            const char = node.textContent[offset];
            if (char === '\n') {
              column = 0;
              row += 1;
              continue;
            }
            const range = document.createRange();
            range.setStart(node, offset);
            range.setEnd(node, offset + 1);
            const drift = Math.abs(range.getBoundingClientRect().left - origin - column * cellWidth);
            if (drift > maxDrift) {
              maxDrift = drift;
              worstCell = { row, column, char };
            }
            column += 1;
          }
        }
        return { maxDrift, worstCell };
      });
      expect(measurements.maxDrift, `${id}: ${JSON.stringify(measurements)}`).toBeLessThan(0.2);
    }

    await testInfo.attach(`dashboard-${width}`, {
      body: await page.locator('#dashboard .tpl-render').screenshot(),
      contentType: 'image/png',
    });
  });
}
