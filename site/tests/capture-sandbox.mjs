// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
import { chromium } from '@playwright/test';

const output = resolve(process.env.SANDBOX_CAPTURE_DIR ?? 'test-results/sandbox-frames');
mkdirSync(output, { recursive: true });
const site = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const server = spawn(process.execPath, ['tests/server.mjs'], { cwd: site, stdio: 'inherit' });

for (let attempt = 0; attempt < 50; attempt += 1) {
  try {
    const response = await fetch('http://127.0.0.1:4173/quickstart.html');
    if (response.ok) break;
  } catch {
    // The server is still starting.
  }
  await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
}

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto('http://127.0.0.1:4173/quickstart.html');
  const panel = page.locator('[data-sandbox]');
  const rows = panel.locator('.xterm-accessibility-tree');
  await rows.getByText(/Hello\. Press q to quit\./).waitFor();
  await panel.screenshot({ path: join(output, '01-minimal.png') });

  await panel.locator('[data-sandbox-template="list"]').click();
  await rows.getByText(/Gauge/).waitFor();
  await page.keyboard.press('ArrowDown');
  await panel.screenshot({ path: join(output, '02-list.png') });

  await panel.locator('[data-sandbox-template="form"]').click();
  await rows.getByText(/Ada/).waitFor();
  await page.keyboard.type('Lovelace');
  await rows.getByText(/AdaLovelace/).waitFor();
  await panel.screenshot({ path: join(output, '03-form.png') });

  writeFileSync(join(output, 'frames.txt'), [
    "file '01-minimal.png'",
    'duration 3',
    "file '02-list.png'",
    'duration 3',
    "file '03-form.png'",
    'duration 3',
    "file '03-form.png'",
    '',
  ].join('\n'));
} finally {
  await browser.close();
  server.kill();
}
