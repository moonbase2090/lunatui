// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const mimeTypes = new Map([
  ['.css', 'text/css; charset=utf-8'],
  ['.html', 'text/html; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.png', 'image/png'],
  ['.wasm', 'application/wasm'],
  ['.woff', 'font/woff'],
  ['.woff2', 'font/woff2'],
]);

createServer(async (request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1:4173');
  const pathname = decodeURIComponent(url.pathname);
  const requested = pathname === '/' ? 'index.html' : pathname.replace(/^\/+/, '');
  const file = resolve(root, requested);
  if (file !== root && !file.startsWith(`${root}${sep}`)) {
    response.writeHead(403).end();
    return;
  }

  try {
    const body = await readFile(file);
    response.writeHead(200, {
      'Content-Type': mimeTypes.get(extname(file)) ?? 'application/octet-stream',
      'Cache-Control': 'no-store',
    });
    response.end(body);
  } catch {
    response.writeHead(404).end();
  }
}).listen(4173, '127.0.0.1');
