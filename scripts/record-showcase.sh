#!/usr/bin/env bash
# Records one widget showcase, lunatui only, for the README gallery: 120x36 by default, a
# motion run of about 8 seconds at 30 fps, a dark theme, at 2x device scale (about 2056x1450),
# as a lossless animated WebP that must stay under 3 MB. Also writes a PNG screenshot of the
# first frame. The frames come straight from the recorded terminal output. Requires asciinema 3,
# agg, gif2webp (libwebp) and ffmpeg.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
WIDTH="${WIDTH:-120}"
HEIGHT="${HEIGHT:-36}"
FRAMES="${FRAMES:-240}"
FPS="${FPS:-30}"
SCALE="${SCALE:-2}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
HARD_MAX_BYTES=3000000
MAX_BYTES="${MAX_BYTES:-$HARD_MAX_BYTES}"
OUT="${OUT:-$ROOT/target/widget-showcase}"
if [[ ! "$CASE" =~ ^[a-z][a-z-]*$ ]]; then echo "record-showcase: bad CASE" >&2; exit 2; fi
for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS" "$MAX_BYTES" "$SCALE"; do
  if [[ ! "$value" =~ ^[1-9][0-9]{0,9}$ ]]; then echo "record-showcase: sizes and counts must be positive integers" >&2; exit 2; fi
done
if (( 10#$MAX_BYTES > HARD_MAX_BYTES )); then
  echo "record-showcase: MAX_BYTES may not exceed ${HARD_MAX_BYTES} (the gallery limit)" >&2
  exit 2
fi
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
webp="$OUT/${NAME}.webp"
python3 "$ROOT/scripts/trim-cast.py" "$cast"
agg --fps-cap 15 --font-size $((14 * 10#$SCALE)) --last-frame-duration 2 "$cast" "$gif"
gif2webp -q 100 -m 6 -mt "$gif" -o "$webp" > /dev/null
bytes=$(wc -c < "$webp" | awk '{print $1}')
if [[ ! "$bytes" =~ ^[0-9]+$ ]]; then
  echo "record-showcase: could not read the size of ${webp}" >&2
  exit 3
fi
if (( 10#$bytes > 10#$MAX_BYTES )); then
  echo "record-showcase: ${webp} is ${bytes} bytes, over the ${MAX_BYTES}-byte limit; shorten FRAMES or lower FPS" >&2
  exit 3
fi
ffmpeg -loglevel error -y -i "$gif" -frames:v 1 "$OUT/${NAME}.png"
rm -f "$gif"
echo "Showcase: ${webp} (${bytes} bytes), screenshot: ${OUT}/${NAME}.png"
