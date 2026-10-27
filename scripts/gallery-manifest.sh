#!/usr/bin/env bash
# Record, for every site widget scene, the hash of what the widgets render and
# the hash of the committed PNG made from it. Run it right after
# scripts/capture-site-widgets.sh so the two stay paired; the test in
# lunatui/tests/site_gallery_repro.rs re-renders each scene and compares.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CAPTURE="scripts/capture-site-widgets.sh"
MEDIA="${OUT:-site/assets/media}"
MANIFEST="${MANIFEST:-site/proof/gallery-manifest.txt}"

if command -v sha256sum >/dev/null; then
  sha() { sha256sum | cut -d' ' -f1; }
else
  sha() { shasum -a 256 | cut -d' ' -f1; }
fi

cases="$(sed -nE 's/^CASES=\((.*)\)$/\1/p' "$CAPTURE")"
if [[ -z "$cases" ]]; then
  echo "gallery-manifest: no CASES list in $CAPTURE" >&2
  exit 2
fi

mkdir -p "$(dirname "$MANIFEST")"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
{
  echo "# scene render sha256, PNG sha256. Written by scripts/gallery-manifest.sh."
  for name in $cases; do
    scene="$(cargo run -q -p lunatui --example gallery -- "--site-widget=$name" | sha)"
    png="$(sha <"$MEDIA/$name.png")"
    echo "$name $scene $png"
  done
} >"$tmp"
mv "$tmp" "$MANIFEST"
trap - EXIT
echo "gallery-manifest: wrote $MANIFEST"
