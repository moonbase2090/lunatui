#!/usr/bin/env bash
# Records one parity-suite widget on the lunatui and ratatui sides with identical
# size, frame count, fps, theme and color profile, then stacks them side by side.
# Requires asciinema 3, agg and ffmpeg. Output: OUT/<case>-<size>-<profile>-<theme>-side-by-side.mp4
# plus the per-side casts and GIFs. Keep README assets small: a short run, MP4 or WebP.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
if [[ ! "$CASE" =~ ^[a-z][a-z-]*$ ]]; then
  echo "record-widget: CASE must be a lowercase case name" >&2
  exit 2
fi
WIDTH="${WIDTH:-80}"
HEIGHT="${HEIGHT:-24}"
FRAMES="${FRAMES:-60}"
FPS="${FPS:-30}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
OUT="${OUT:-$ROOT/target/widget-recordings}"
NAME="${CASE}-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
FEATURES=(color-profiles)
if [[ "$CASE" == "line-gauge" ]]; then FEATURES+=(line-gauge); fi
cargo build -q --release -p lunatui --features "${FEATURES[*]}" --example widget_preview
BIN="$ROOT/target/release/examples/widget_preview"
for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS"; do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "record-widget: sizes, frames and fps must be positive integers" >&2
    exit 2
  fi
done
case "$PROFILE" in truecolor | ansi256 | ansi16) ;; *) echo "record-widget: unknown PROFILE" >&2; exit 2 ;; esac
case "$THEME" in dark | light) ;; *) echo "record-widget: unknown THEME" >&2; exit 2 ;; esac
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
