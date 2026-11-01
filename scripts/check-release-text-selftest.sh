#!/usr/bin/env bash
# Regression checks for scripts/check-release-text.sh (no denylist terms printed).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHECK="$ROOT/scripts/check-release-text.sh"
chmod +x "$CHECK"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

printf 'not a valid gzip payload' >"$tmpdir/corrupt.tar.gz"
set +e
RELEASE_DENYLIST_FILE=/nonexistent "$CHECK" --artifact-dir "$tmpdir" >/dev/null 2>&1
rc=$?
set -e
if [[ "$rc" -eq 0 ]]; then
  echo "check-release-text-selftest: corrupt .tar.gz must not exit 0" >&2
  exit 1
fi

mkdir "$tmpdir/wasm"
printf '\000asm\001\000\000\000' >"$tmpdir/wasm/clean.wasm"
chmod 0644 "$tmpdir/wasm/clean.wasm"
RELEASE_DENYLIST_FILE=/nonexistent "$CHECK" --artifact-dir "$tmpdir/wasm" >/dev/null

# A valid custom section carries a synthetic builder path; the module is not executable.
printf '\000asm\001\000\000\000\000\024\000/home/example/build' >"$tmpdir/wasm/path.wasm"
chmod 0644 "$tmpdir/wasm/path.wasm"
set +e
RELEASE_DENYLIST_FILE=/nonexistent "$CHECK" --artifact-dir "$tmpdir/wasm" >"$tmpdir/wasm-scan.log" 2>&1
rc=$?
set -e
if [[ "$rc" -ne 1 ]]; then
  echo "check-release-text-selftest: a non-executable Wasm module with a builder path must exit 1" >&2
  exit 1
fi
if grep -Fq '/home/example/build' "$tmpdir/wasm-scan.log"; then
  echo "check-release-text-selftest: the scanner must not print the matched term" >&2
  exit 1
fi

echo "check-release-text-selftest: ok"
