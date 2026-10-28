#!/usr/bin/env bash
# Records parity widgets side by side; CASE=image records the Lunatui Image
# cell fallback as a standalone gallery clip. Requires asciinema 3, agg, ffmpeg.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
if [[ ! "$CASE" =~ ^[a-z][a-z-]*$ ]]; then
  echo "record-widget: CASE must be a lowercase case name" >&2
  exit 2
fi
if [[ "$CASE" == "image" ]]; then
  WIDTH="${WIDTH:-120}"
  HEIGHT="${HEIGHT:-36}"
  FRAMES="${FRAMES:-200}"
  FPS="${FPS:-30}"
else
  WIDTH="${WIDTH:-80}"
  HEIGHT="${HEIGHT:-24}"
  FRAMES="${FRAMES:-60}"
  FPS="${FPS:-30}"
fi
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
OUT="${OUT:-$ROOT/target/widget-recordings}"
MAX_BYTES="${MAX_BYTES:-2000000}"
NAME="${CASE}-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS"; do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "record-widget: sizes, frames and fps must be positive integers" >&2
    exit 2
  fi
done
case "$PROFILE" in truecolor | ansi256 | ansi16) ;; *) echo "record-widget: unknown PROFILE" >&2; exit 2 ;; esac
case "$THEME" in dark | light) ;; *) echo "record-widget: unknown THEME" >&2; exit 2 ;; esac
if [[ "$CASE" == "image" ]]; then
  if (( WIDTH < 120 || HEIGHT < 36 || FRAMES < 6 * FPS || FRAMES > 12 * FPS )); then
    echo "record-widget: image showcase needs 120x36+ and 6 to 12 seconds of motion" >&2
    exit 2
  fi
  if [[ "$PROFILE" != "truecolor" || "$THEME" != "dark" ]]; then
    echo "record-widget: image showcase uses the truecolor dark theme" >&2
    exit 2
  fi
  if [[ ! "$MAX_BYTES" =~ ^[1-9][0-9]*$ ]] || (( MAX_BYTES > 2000000 )); then
    echo "record-widget: MAX_BYTES must be between 1 and 2000000" >&2
    exit 2
  fi
  NAME="image-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
  cargo build -q --release -p lunatui --features image-decode --example image_showcase
  BIN="$ROOT/target/release/examples/image_showcase"
  mkdir -p "$ROOT/target/widget-recordings"
  CAST="$ROOT/target/widget-recordings/${NAME}.cast"
  GIF="${OUT}/${NAME}.gif"
  PNG="${OUT}/${NAME}.png"
  asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "lunatui image showcase" \
    -c "$(printf '%q ' "$BIN" --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES" --fps "$FPS")" \
    "$CAST"
  python3 "$ROOT/scripts/trim-cast.py" "$CAST"
  agg --fps-cap 15 --font-size 14 "$CAST" "$GIF"
  BYTES="$(wc -c < "$GIF" | awk '{print $1}')"
  if [[ ! "$BYTES" =~ ^[0-9]+$ ]] || (( BYTES > MAX_BYTES )); then
    echo "record-widget: ${GIF} exceeds the ${MAX_BYTES}-byte limit" >&2
    exit 3
  fi
  DURATION="$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$GIF")"
  if ! awk -v duration="$DURATION" 'BEGIN { exit !(duration >= 6 && duration <= 12) }'; then
    echo "record-widget: GIF duration ${DURATION}s is outside the 6 to 12 second range" >&2
    exit 3
  fi
  ffmpeg -loglevel error -y -i "$GIF" -frames:v 1 "$PNG"
  echo "Image showcase: ${GIF} (${BYTES} bytes, ${DURATION}s), screenshot: ${PNG}"
  exit 0
fi
cargo build -q --release -p lunatui --features color-profiles --example widget_preview
BIN="$ROOT/target/release/examples/widget_preview"
ARGS=(--case "$CASE" --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES" --fps "$FPS" --profile "$PROFILE" --theme "$THEME")
for side in lunatui ratatui; do
  asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "${side} ${CASE}" \
    -c "$(printf '%q ' "$BIN" --side "$side" "${ARGS[@]}")" "${OUT}/${NAME}-${side}.cast"
  python3 "$ROOT/scripts/trim-cast.py" "${OUT}/${NAME}-${side}.cast"
  agg --font-size 14 "${OUT}/${NAME}-${side}.cast" "${OUT}/${NAME}-${side}.gif"
done
ffmpeg -loglevel error -y -i "${OUT}/${NAME}-lunatui.gif" -i "${OUT}/${NAME}-ratatui.gif" \
  -filter_complex "[0:v][1:v]hstack=inputs=2,pad=ceil(iw/2)*2:ceil(ih/2)*2,format=yuv420p" \
  -c:v libx264 -crf 28 -movflags +faststart \
  "${OUT}/${NAME}-side-by-side.mp4"
echo "Recording: ${OUT}/${NAME}-side-by-side.mp4"
