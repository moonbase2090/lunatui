#!/usr/bin/env bash
# Capture the site widget gallery as PNG stills.
#
# Each scene is rendered by the gallery example, then rasterized by
# scripts/rasterize-cells.py inside a pinned container (image digest, Debian
# packages, Pillow, and font files are all fixed), so the same widget output
# always gives the same PNG bytes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CASE="${1:-all}"
OUT="${OUT:-$ROOT/site/assets/media}"
SHEET="${SHEET:-$ROOT/site/proof/widget-contact-sheet.png}"
CASES=(bigtext block calendar gauge lightcycle list overlay paragraph padding scrollbar sparkline spinner statusbar statusdot table tabs textinput borders)

IMAGE="python@sha256:b3575dcf816cdacc5292df77a6e3e59b8941631078373a045efa2ac1e9c5c3b2"
APT_PACKAGES="libraqm0=0.10.2-1 fonts-dejavu-core=2.37-8"
PIP_PACKAGES="pillow==12.0.0 fonttools==4.60.1"
JETBRAINS_URL="https://raw.githubusercontent.com/google/fonts/2e05c1cf00a6e4f40a4b931600a90881c26e15cd/ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf"
FONT_CACHE="${LUNATUI_CAPTURE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/lunatui-site-capture}"

if ! command -v docker >/dev/null || ! command -v ffmpeg >/dev/null; then
  echo "capture-site-widgets requires docker and ffmpeg." >&2
  exit 2
fi

if [[ "$CASE" == all ]]; then
  names=("${CASES[@]}")
else
  valid=false
  for name in "${CASES[@]}"; do
    if [[ "$CASE" == "$name" ]]; then valid=true; break; fi
  done
  if [[ "$valid" != true ]]; then
    printf 'unknown widget %q\nvalid cases: %s\n' "$CASE" "${CASES[*]}" >&2
    exit 2
  fi
  names=("$CASE")
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/ansi" "$TMP/png" "$TMP/fonts" "$FONT_CACHE" "$OUT" "$(dirname "$SHEET")"

if [[ ! -f "$FONT_CACHE/JetBrainsMono-wght.ttf" ]]; then
  curl -fsSL -o "$FONT_CACHE/JetBrainsMono-wght.ttf.part" "$JETBRAINS_URL"
  mv "$FONT_CACHE/JetBrainsMono-wght.ttf.part" "$FONT_CACHE/JetBrainsMono-wght.ttf"
fi
cp "$FONT_CACHE/JetBrainsMono-wght.ttf" "$TMP/fonts/"

pairs=()
for name in "${names[@]}"; do
  cargo run -q -p lunatui --example gallery -- "--site-widget=$name" > "$TMP/ansi/$name.ansi"
  pairs+=("/work/ansi/$name.ansi" "/work/png/$name.png")
done

docker run --rm \
  -v "$TMP:/work" \
  -v "$ROOT/scripts/rasterize-cells.py:/rasterize-cells.py:ro" \
  "$IMAGE" sh -euc "
    apt-get update -qq >/dev/null
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends $APT_PACKAGES >/dev/null
    pip install -q --root-user-action=ignore --disable-pip-version-check $PIP_PACKAGES
    cp /usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf /work/fonts/
    python /rasterize-cells.py --fonts /work/fonts $(printf '%q ' "${pairs[@]}")
  "

for name in "${names[@]}"; do
  cp "$TMP/png/$name.png" "$OUT/$name.png"
  echo "captured $name"
done

ffmpeg -loglevel error -y -pattern_type glob -framerate 1 -i "$OUT/*.png" \
  -vf 'scale=288:216:flags=lanczos,tile=4x5:padding=16:margin=16:color=0x0A0D1C' \
  -frames:v 1 "$SHEET"
echo "contact sheet: $SHEET"
