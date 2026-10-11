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

echo "check-release-text-selftest: ok"
