#!/usr/bin/env bash
# Records one widget showcase, lunatui only, for the README gallery: 120x36 by default, a
# motion run of about 8 seconds at 30 fps, a dark theme, and a GIF that must stay under 2 MB.
# Also writes a PNG screenshot of the first frame. Requires asciinema 3, agg and ffmpeg.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
WIDTH="${WIDTH:-120}"
HEIGHT="${HEIGHT:-36}"
FRAMES="${FRAMES:-240}"
FPS="${FPS:-30}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
HARD_MAX_BYTES=2000000
MAX_BYTES="${MAX_BYTES:-$HARD_MAX_BYTES}"
OUT="${OUT:-$ROOT/target/widget-showcase}"
if [[ ! "$CASE" =~ ^[a-z][a-z-]*$ ]]; then echo "record-showcase: bad CASE" >&2; exit 2; fi
if (( MAX_BYTES > HARD_MAX_BYTES )); then
  echo "record-showcase: MAX_BYTES may not exceed ${HARD_MAX_BYTES} (the gallery limit)" >&2
  exit 2
fi
for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS" "$MAX_BYTES"; do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then echo "record-showcase: sizes and counts must be positive integers" >&2; exit 2; fi
done
case "$PROFILE" in truecolor | ansi256 | ansi16) ;; *) echo "record-showcase: unknown PROFILE" >&2; exit 2 ;; esac
case "$THEME" in dark | light) ;; *) echo "record-showcase: unknown THEME" >&2; exit 2 ;; esac
NAME="${CASE}-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
cargo build -q --release -p lunatui --features color-profiles --example widget_preview
BIN="$ROOT/target/release/examples/widget_preview"
cast="$OUT/${NAME}.cast"
asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "lunatui ${CASE}" \
  -c "$(printf '%q ' "$BIN" --side lunatui --case "$CASE" --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES" --fps "$FPS" --profile "$PROFILE" --theme "$THEME")" \
  "$cast"
gif="$OUT/${NAME}.gif"
agg --fps-cap 15 --font-size 14 "$cast" "$gif"
bytes=$(wc -c < "$gif")
if (( bytes > MAX_BYTES )); then
  echo "record-showcase: ${gif} is ${bytes} bytes, over the ${MAX_BYTES}-byte limit; shorten FRAMES or lower FPS" >&2
  exit 3
fi
ffmpeg -loglevel error -y -i "$gif" -frames:v 1 "$OUT/${NAME}.png"
echo "Showcase: ${gif} (${bytes} bytes), screenshot: ${OUT}/${NAME}.png"
