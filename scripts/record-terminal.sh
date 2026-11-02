#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
# Capture in an isolated Unix VM; render its unchanged cast on a workstation if needed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${OUT:-$ROOT/target/terminal-recording}"
PROFILE="${PROFILE:-truecolor}"
THEME="${THEME:-dark}"
WIDTH="${WIDTH:-120}"
HEIGHT="${HEIGHT:-36}"
NAME="terminal-${WIDTH}x${HEIGHT}-${PROFILE}-${THEME}"
mkdir -p "$OUT"
if [[ "${RENDER_ONLY:-0}" != 1 ]]; then
  python3 scripts/terminal-fixture.py target/terminal-demo
  # Prebuild only. Both captured test runs still execute real tests in the child PTY.
  env -u CARGO_TARGET_DIR cargo test --quiet --manifest-path target/terminal-demo/Cargo.toml
  cargo build --locked --release -p lunatui --features app,color-profiles --example embedded_terminal
  python3 scripts/capture-terminal.py "$OUT/$NAME.cast" "$WIDTH" "$HEIGHT" 300 30 "$PROFILE" "$THEME"
fi
python3 scripts/check-terminal-cast.py "$OUT/$NAME.cast"
if [[ "${CAPTURE_ONLY:-0}" == 1 ]]; then exit 0; fi
AGG="${AGG:-agg}"
if [[ "$("$AGG" --version)" != "agg 1.9.0+lunatui.2" ]]; then
  echo "record-terminal: install the pinned terminal rasterizer from record-showcase.sh" >&2
  exit 2
fi
python3 -I scripts/check-box-joins.py "$AGG" 1.4
"$AGG" --fps-cap 60 --font-size 28 --line-height 1.4 --last-frame-duration 0.033333 \
  "$OUT/$NAME.cast" "$OUT/$NAME.gif"
python3 scripts/gif-to-webp.py "$OUT/$NAME.gif" "$OUT/$NAME.webp" 30 0.033333 30
BYTES="$(wc -c < "$OUT/$NAME.webp" | awk '{print $1}')"
if (( BYTES > 2000000 )); then echo "record-terminal: WebP exceeds 2 MB" >&2; exit 3; fi
ffmpeg -loglevel error -y -i "$OUT/$NAME.gif" -frames:v 1 "$OUT/$NAME.png"
ffmpeg -loglevel error -y -i "$OUT/$NAME.gif" \
  -vf 'tpad=stop_mode=clone:stop_duration=1,fps=9/10,scale=600:-1,tile=3x3' -frames:v 1 "$OUT/$NAME-contact.png"
echo "Recorded actual child PTY workspace: $OUT/$NAME.webp ($BYTES bytes)"
