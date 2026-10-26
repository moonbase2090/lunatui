#!/usr/bin/env bash
# Records one parity-suite widget on the lunatui and ratatui sides with identical
# size, frame count, fps, theme and color profile, then stacks them side by side.
# Requires asciinema 3, agg and ffmpeg. Output: OUT/<case>-<size>-<profile>-<theme>.{cast,gif,mp4}
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASE="${CASE:-block}"
WIDTH="${WIDTH:-80}"
HEIGHT="${HEIGHT:-24}"
FRAMES="${FRAMES:-120}"
FPS="${FPS:-30}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
OUT="${OUT:-$ROOT/target/widget-recordings}"
NAME="${CASE}-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
cargo build -q --release -p lunatui --features color-profiles --example widget_preview
BIN="$ROOT/target/release/examples/widget_preview"
ARGS="--case ${CASE} --width ${WIDTH} --height ${HEIGHT} --frames ${FRAMES} --fps ${FPS} --profile ${PROFILE} --theme ${THEME}"
for side in lunatui ratatui; do
  asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "${side} ${CASE}" \
    -c "${BIN} --side ${side} ${ARGS}" "${OUT}/${NAME}-${side}.cast"
  agg --font-size 14 "${OUT}/${NAME}-${side}.cast" "${OUT}/${NAME}-${side}.gif"
done
ffmpeg -loglevel error -y -i "${OUT}/${NAME}-lunatui.gif" -i "${OUT}/${NAME}-ratatui.gif" \
  -filter_complex "[0:v][1:v]hstack=inputs=2" "${OUT}/${NAME}-side-by-side.gif"
echo "Recordings: ${OUT}/${NAME}-side-by-side.gif"
