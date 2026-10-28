#!/usr/bin/env bash
# Record a Lunatui-only animated app screen for the README gallery.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
WIDTH="${WIDTH:-120}"
HEIGHT="${HEIGHT:-36}"
FRAMES="${FRAMES:-200}"
FPS="${FPS:-30}"
MAX_BYTES="${MAX_BYTES:-2000000}"
OUT="${OUT:-$ROOT/docs/media}"

for value in "$WIDTH" "$HEIGHT" "$FRAMES" "$FPS" "$MAX_BYTES"; do
  if [[ ! "$value" =~ ^[1-9][0-9]{0,9}$ ]]; then
    echo "record-app-screen: sizes and counts must be positive integers" >&2
    exit 2
  fi
done
if (( WIDTH < 120 || HEIGHT < 36 )); then
  echo "record-app-screen: gallery recordings must be at least 120x36" >&2
  exit 2
fi
if (( FRAMES < 6 * FPS || FRAMES > 12 * FPS )); then
  echo "record-app-screen: motion must run for 6 to 12 seconds" >&2
  exit 2
fi
if (( MAX_BYTES > 2000000 )); then
  echo "record-app-screen: MAX_BYTES may not exceed 2000000" >&2
  exit 2
fi

NAME="nightshift-board-${WIDTH}x${HEIGHT}-dark"
mkdir -p "$OUT"
mkdir -p "$ROOT/target/app-screen-recordings"
cargo build -q --release -p lunatui --features "color-profiles line-gauge" --example nightshift_board
BIN="$ROOT/target/release/examples/nightshift_board"
CAST="$ROOT/target/app-screen-recordings/${NAME}.cast"
GIF="$OUT/${NAME}.gif"
PNG="$OUT/${NAME}.png"

asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "lunatui nightshift board" \
  -c "$(printf '%q ' "$BIN" --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES" --fps "$FPS")" \
  "$CAST"
python3 "$ROOT/scripts/trim-cast.py" "$CAST"
agg --fps-cap 15 --font-size 14 "$CAST" "$GIF"
BYTES="$(wc -c < "$GIF" | awk '{print $1}')"
if [[ ! "$BYTES" =~ ^[0-9]+$ ]] || (( BYTES > MAX_BYTES )); then
  echo "record-app-screen: ${GIF} exceeds the ${MAX_BYTES}-byte limit" >&2
  exit 3
fi
DURATION="$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$GIF")"
if ! awk -v duration="$DURATION" 'BEGIN { exit !(duration >= 6 && duration <= 12) }'; then
  echo "record-app-screen: GIF duration ${DURATION}s is outside the 6 to 12 second range" >&2
  exit 3
fi
ffmpeg -loglevel error -y -i "$GIF" -frames:v 1 "$PNG"
echo "App screen: ${GIF} (${BYTES} bytes, ${DURATION}s), screenshot: ${PNG}"
