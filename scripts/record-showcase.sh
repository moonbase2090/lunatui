#!/usr/bin/env bash
# Records one widget showcase, lunatui only, for the README gallery: 120x36 by default, a
# motion run of about 8 seconds at 30 fps, a dark theme, at 2x device scale (about 2056x1450),
# as a lossless animated WebP that must stay under 3 MB. Also writes a PNG screenshot of the
# first frame. The frames come straight from the recorded terminal output, each kept on its slot
# of the fixed FPS timestep and shown for its exact milliseconds (GIF delays only count
# hundredths). LINE_HEIGHT is the rendered line height (1.4 by default; 1.0 keeps Braille dots
# evenly spaced). AGG names the agg binary, which must be the pinned build from
# moonbase2090/agg (AGG_PIN below): stock agg 1.9 draws rounded corners, tees, double lines and
# octants from the font, so they miss their lines. scripts/check-box-joins.py checks the renderer
# and then every border join in the recorded case. Requires asciinema 3, agg, img2webp (libwebp)
# and ffmpeg with ffprobe.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
WIDTH="${WIDTH:-120}"
HEIGHT="${HEIGHT:-36}"
FRAMES="${FRAMES:-240}"
FPS="${FPS:-30}"
LAST_FRAME_DURATION="${LAST_FRAME_DURATION:-2}"
LINE_HEIGHT="${LINE_HEIGHT:-1.4}"
SCALE="${SCALE:-2}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
HARD_MAX_BYTES=3000000
MAX_BYTES="${MAX_BYTES:-$HARD_MAX_BYTES}"
OUT="${OUT:-$ROOT/target/widget-showcase}"
AGG="${AGG:-agg}"
AGG_PIN="agg 1.9.0+lunatui.2"
AGG_INSTALL="cargo install --git https://github.com/moonbase2090/agg --rev 42d70620cc4f24f5754df4c57b27081350fad118 --locked"
if [[ "$("$AGG" --version 2>/dev/null)" != "$AGG_PIN" ]]; then
  echo "record-showcase: $AGG is not $AGG_PIN; install it with: $AGG_INSTALL" >&2
  exit 2
fi
if [[ ! "$CASE" =~ ^[a-z][a-z-]*$ ]]; then echo "record-showcase: bad CASE" >&2; exit 2; fi
for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS" "$MAX_BYTES" "$SCALE"; do
  if [[ ! "$value" =~ ^[1-9][0-9]{0,9}$ ]]; then echo "record-showcase: sizes and counts must be positive integers" >&2; exit 2; fi
done
if (( 10#$MAX_BYTES > HARD_MAX_BYTES )); then
  echo "record-showcase: MAX_BYTES may not exceed ${HARD_MAX_BYTES} (the gallery limit)" >&2
  exit 2
fi
if [[ ! "$LINE_HEIGHT" =~ ^[1-2](\.[0-9]{1,2})?$ ]]; then echo "record-showcase: LINE_HEIGHT must be between 1 and 2.99" >&2; exit 2; fi
case "$PROFILE" in truecolor | ansi256 | ansi16) ;; *) echo "record-showcase: unknown PROFILE" >&2; exit 2 ;; esac
case "$THEME" in dark | light) ;; *) echo "record-showcase: unknown THEME" >&2; exit 2 ;; esac
NAME="${CASE}-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
python3 -I "$ROOT/scripts/check-box-joins.py" "$AGG" "$LINE_HEIGHT"
cargo build -q --release -p lunatui --features color-profiles --example widget_preview
BIN="$ROOT/target/release/examples/widget_preview"
cast="$OUT/${NAME}.cast"
asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "lunatui ${CASE}" \
  -c "$(printf '%q ' "$BIN" --side lunatui --case "$CASE" --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES" --fps "$FPS" --profile "$PROFILE" --theme "$THEME")" \
  "$cast"
gif="$OUT/${NAME}.gif"
webp="$OUT/${NAME}.webp"
python3 "$ROOT/scripts/trim-cast.py" "$cast" "$FPS" "$FRAMES"
python3 -I "$ROOT/scripts/check-box-joins.py" "$AGG" "$LINE_HEIGHT" --demo "$CASE" --preview "$BIN" \
  --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES"
# agg writes no GIF delay under 20 ms, so above 30 fps it renders the cast slowed to 30 fps
# and gif-to-webp puts each frame back on its real slot.
TIMELINE_FPS=$(( 10#$FPS < 30 ? 10#$FPS : 30 ))
SPEED=$(awk -v t="$TIMELINE_FPS" -v f="$FPS" 'BEGIN { printf "%.6f", t / f }')
# Frames now sit exactly 1/FPS apart; a cap at FPS itself would drop some to rounding.
"$AGG" --speed "$SPEED" --fps-cap "$((10#$FPS * 2))" --font-size $((14 * 10#$SCALE)) --line-height "$LINE_HEIGHT" \
  --last-frame-duration "$LAST_FRAME_DURATION" "$cast" "$gif"
python3 "$ROOT/scripts/gif-to-webp.py" "$gif" "$webp" "$FPS" "$LAST_FRAME_DURATION" "$TIMELINE_FPS"
bytes=$(wc -c < "$webp" | awk '{print $1}')
if [[ ! "$bytes" =~ ^[0-9]+$ ]]; then
  echo "record-showcase: could not read the size of ${webp}" >&2
  exit 3
fi
if (( 10#$bytes > 10#$MAX_BYTES )); then
  echo "record-showcase: ${webp} is ${bytes} bytes, over the ${MAX_BYTES}-byte limit; shorten FRAMES" >&2
  exit 3
fi
ffmpeg -loglevel error -y -i "$gif" -frames:v 1 "$OUT/${NAME}.png"
rm -f "$gif"
echo "Showcase: ${webp} (${bytes} bytes), screenshot: ${OUT}/${NAME}.png"
