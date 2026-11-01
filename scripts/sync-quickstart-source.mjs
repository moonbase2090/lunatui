// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const pagePath = join(root, 'site', 'quickstart.html');
const newline = readFileSync(pagePath, 'utf8').includes('\r\n') ? '\r\n' : '\n';
let page = readFileSync(pagePath, 'utf8');

function escapeHtml(source) {
  return source
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#x27;');
}

for (const name of ['minimal', 'list', 'form', 'dashboard', 'popover', 'layout']) {
  const sourcePath = join(root, 'examples', 'templates', `${name}.rs`);
  const source = readFileSync(sourcePath, 'utf8').replaceAll(/\r?\n/g, newline);
  const escaped = escapeHtml(source);
  const article = new RegExp(`(<article class="card" id="${name}">[\\s\\S]*?<pre><code>)[\\s\\S]*?(</code></pre>)`);
  if (!article.test(page)) throw new Error(`missing quickstart source block for ${name}`);
  page = page.replace(article, `$1${escaped}$2`);
}

if (process.argv.includes('--check')) {
  const current = readFileSync(pagePath, 'utf8');
  if (page !== current) {
    console.error('site/quickstart.html does not match examples/templates');
    process.exit(1);
  }
} else {
  writeFileSync(pagePath, page);
}
