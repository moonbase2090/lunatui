// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { defineConfig, devices } from '@playwright/test';

const siteTestPort = Number(process.env.LUNATUI_SITE_TEST_PORT ?? 4173);

export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: `http://127.0.0.1:${siteTestPort}`,
  },
  projects: [
    {
      name: 'desktop-firefox',
      testMatch: /terminal-alignment\.spec\.js/,
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'desktop-webkit',
      testMatch: /terminal-alignment\.spec\.js/,
      use: { ...devices['Desktop Safari'] },
    },
    {
      name: 'desktop-chromium',
      testIgnore: /mobile-sandbox\.spec\.js/,
      use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 800 } },
    },
    {
      name: 'android-chrome',
      testMatch: /mobile-sandbox\.spec\.js/,
      use: { ...devices['Pixel 7'], browserName: 'chromium' },
    },
    {
      name: 'iphone-safari',
      testMatch: /mobile-sandbox\.spec\.js/,
      use: { ...devices['iPhone 13'], browserName: 'webkit' },
    },
  ],
  webServer: {
    command: 'node tests/server.mjs',
    url: `http://127.0.0.1:${siteTestPort}`,
    reuseExistingServer: !process.env.CI,
  },
});
