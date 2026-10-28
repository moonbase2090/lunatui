#!/usr/bin/env bash
# Records one parity-suite widget on both sides, or the combined release-control
# app screen for the gallery. Requires asciinema 3, agg and ffmpeg.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
if [[ ! "$CASE" =~ ^[a-z][a-z-]*$ ]]; then
  echo "record-widget: CASE must be a lowercase case name" >&2
  exit 2
fi
if [[ "$CASE" == "nightshift-board" ]]; then
  WIDTH="${WIDTH:-120}"
  HEIGHT="${HEIGHT:-36}"
  FRAMES="${FRAMES:-180}"
else
  WIDTH="${WIDTH:-80}"
  HEIGHT="${HEIGHT:-24}"
  FRAMES="${FRAMES:-60}"
fi
FPS="${FPS:-30}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
OUT="${OUT:-$ROOT/target/widget-recordings}"
NAME="${CASE}-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
FEATURES=(color-profiles)
if [[ "$CASE" == "line-gauge" ]]; then FEATURES+=(line-gauge); fi
for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS"; do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "record-widget: sizes, frames and fps must be positive integers" >&2
    exit 2
  fi
done
case "$PROFILE" in truecolor | ansi256 | ansi16) ;; *) echo "record-widget: unknown PROFILE" >&2; exit 2 ;; esac
case "$THEME" in dark | light) ;; *) echo "record-widget: unknown THEME" >&2; exit 2 ;; esac
if [[ "$CASE" == "nightshift-board" ]]; then
  if (( WIDTH != 120 || HEIGHT != 36 || 10#$FRAMES < 6 * 10#$FPS || 10#$FRAMES > 12 * 10#$FPS )); then
    echo "record-widget: nightshift-board needs 120x36 and 6 to 12 seconds of motion" >&2
    exit 2
  fi
  if [[ "$PROFILE" != "truecolor" || "$THEME" != "dark" ]]; then
    echo "record-widget: nightshift-board uses the truecolor dark theme" >&2
    exit 2
  fi
  MAX_BYTES="${MAX_BYTES:-2000000}"
  if [[ ! "$MAX_BYTES" =~ ^[1-9][0-9]*$ ]] || (( 10#$MAX_BYTES > 2000000 )); then
    echo "record-widget: MAX_BYTES must be between 1 and 2000000" >&2
    exit 2
  fi
  NAME="nightshift-board-${WIDTH}x${HEIGHT}-dark"
  cargo build -q --release -p lunatui --features color-profiles,line-gauge --example nightshift_board
  BIN="$ROOT/target/release/examples/nightshift_board"
  CAST="$OUT/${NAME}.cast"
  GIF="$OUT/${NAME}.gif"
  PNG="$OUT/${NAME}.png"
  asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "lunatui nightshift board" \
    -c "$(printf '%q ' "$BIN" --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES" --fps "$FPS")" \
    "$CAST"
  python3 "$ROOT/scripts/trim-cast.py" "$CAST"
  agg --fps-cap 15 --font-size 14 "$CAST" "$GIF"
  BYTES="$(wc -c < "$GIF" | awk '{print $1}')"
  if [[ ! "$BYTES" =~ ^[0-9]+$ ]] || (( 10#$BYTES > 10#$MAX_BYTES )); then
    echo "record-widget: ${GIF} exceeds the ${MAX_BYTES}-byte gallery limit" >&2
    exit 3
  fi
  DURATION="$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$GIF")"
  if ! awk -v duration="$DURATION" 'BEGIN { exit !(duration >= 6 && duration <= 12) }'; then
    echo "record-widget: GIF duration ${DURATION}s is outside the 6 to 12 second range" >&2
    exit 3
  fi
  ffmpeg -loglevel error -y -i "$GIF" -frames:v 1 "$PNG"
  test -s "$PNG"
  echo "App showcase: ${GIF} (${BYTES} bytes, ${DURATION}s), screenshot: ${PNG}"
  exit 0
fi
cargo build -q --release -p lunatui --features "${FEATURES[*]}" --example widget_preview
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
ffmpeg -loglevel error -y -ss 1 -i "${OUT}/${NAME}-side-by-side.mp4" -frames:v 1 \
  "${OUT}/${NAME}-side-by-side.png"
test -s "${OUT}/${NAME}-side-by-side.png"
echo "Recording: ${OUT}/${NAME}-side-by-side.mp4"
echo "Screenshot: ${OUT}/${NAME}-side-by-side.png"
