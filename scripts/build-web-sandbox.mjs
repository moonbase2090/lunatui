// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

import { copyFileSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { homedir } from 'node:os';

const WASM_BINDGEN_VERSION = '0.2.129';
const WASM_GZIP_BUDGET = 400 * 1024;
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const site = join(root, 'site');
const output = join(site, 'assets', 'sandbox');
const skipRust = process.argv.includes('--skip-rust');

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, ...options.env },
    stdio: options.capture ? 'pipe' : 'inherit',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    if (options.capture) process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
  return (result.stdout ?? '').trim();
}

mkdirSync(output, { recursive: true });
if (!skipRust) {
  const installedVersion = run('wasm-bindgen', ['--version'], { capture: true });
  if (installedVersion !== `wasm-bindgen ${WASM_BINDGEN_VERSION}`) {
    throw new Error(`expected wasm-bindgen ${WASM_BINDGEN_VERSION}, found ${installedVersion}`);
  }

  const cargoHome = resolve(process.env.CARGO_HOME || join(homedir(), '.cargo'));
  const rustflags = [
    process.env.RUSTFLAGS?.trim(),
    `--remap-path-prefix=${root}=.`,
    `--remap-path-prefix=${cargoHome}=cargo-home`,
  ].filter(Boolean).join(' ');
  run('cargo', [
    'build',
    '--locked',
    '--release',
    '--package',
    'lunatui-web',
    '--target',
    'wasm32-unknown-unknown',
  ], { env: { RUSTFLAGS: rustflags } });

  const wasm = join(root, 'target', 'wasm32-unknown-unknown', 'release', 'lunatui_web.wasm');
  run('wasm-bindgen', [
    wasm,
    '--target',
    'web',
    '--no-typescript',
    '--out-dir',
    output,
    '--out-name',
    'lunatui_web',
  ]);
}

for (const [source, destination] of [
  ['node_modules/@xterm/xterm/lib/xterm.js', 'xterm.js'],
  ['node_modules/@xterm/xterm/css/xterm.css', 'xterm.css'],
  ['node_modules/@xterm/xterm/LICENSE', 'xterm-LICENSE.txt'],
  ['node_modules/@xterm/addon-fit/lib/addon-fit.js', 'addon-fit.js'],
  ['node_modules/@xterm/addon-fit/LICENSE', 'addon-fit-LICENSE.txt'],
]) {
  copyFileSync(join(site, source), join(output, destination));
}

const generatedWasm = join(output, 'lunatui_web_bg.wasm');
const rawBytes = readFileSync(generatedWasm);
const gzipBytes = gzipSync(rawBytes, { level: 9 }).byteLength;
if (gzipBytes > WASM_GZIP_BUDGET) {
  throw new Error(`WASM gzip size ${gzipBytes} exceeds ${WASM_GZIP_BUDGET} bytes`);
}

console.log(`${relative(root, generatedWasm)}: ${rawBytes.byteLength} bytes raw, ${gzipBytes} bytes gzip`);
