#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

set -euo pipefail

version="0.2.129"
archive="wasm-bindgen-${version}-x86_64-unknown-linux-musl.tar.gz"
checksum="82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e"
download="https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}"
temporary="$(mktemp -d)"
trap 'rm -r -- "$temporary"' EXIT

curl --proto '=https' --tlsv1.2 -fsSL "${download}/${archive}" -o "${temporary}/${archive}"
printf '%s  %s\n' "$checksum" "${temporary}/${archive}" | sha256sum --check -
tar -xzf "${temporary}/${archive}" -C "$temporary"
install -m 0755 \
  "${temporary}/wasm-bindgen-${version}-x86_64-unknown-linux-musl/wasm-bindgen" \
  "${CARGO_HOME:-$HOME/.cargo}/bin/wasm-bindgen"
