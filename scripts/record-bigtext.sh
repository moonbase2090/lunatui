#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
# Records the live cutover screen and a 3x3 contact sheet from its terminal output.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CASE=bigtext WIDTH=120 HEIGHT=36 FRAMES=330 FPS=30 LAST_FRAME_DURATION=0.033333
export PROFILE=truecolor THEME=dark SCALE=2
export OUT="${OUT:-$ROOT/target/widget-showcase}"
AGG="${AGG:-agg}"
"$ROOT/scripts/record-showcase.sh"
NAME="$OUT/bigtext-120x36-truecolor-dark"
"$AGG" --fps-cap 15 --font-size 14 --line-height 1.4 --last-frame-duration 0.033333 \
  "$NAME.cast" "$NAME-review.gif"
ffmpeg -loglevel error -y -i "$NAME-review.gif" \
  -vf 'fps=9/11,scale=600:-1,tile=3x3' -frames:v 1 "$NAME-contact.png"
python3 - "$NAME-review.gif" "$NAME.cast" "$ROOT" <<'PY'
import json
import os
import runpy
import subprocess
import sys

gif, cast, root = sys.argv[1:]
with open(cast, encoding="utf-8") as source:
    header = json.loads(next(source))
    frames = [json.loads(line) for line in source if line.strip()]
assert header["term"]["cols"] == 120 and header["term"]["rows"] == 36, header
assert len(frames) == 330, len(frames)
assert all("\x1b[?1049l" not in frame[2] for frame in frames)
screen = runpy.run_path(os.path.join(root, "scripts/check-box-joins.py"))["screen"]
for tick, expected in [
    (0, ["REQUEST RATE", "CUTOVER EVENTS", "1480 req/s", "Standby pool ready"]),
    (180, ["1840 req/s", "Cutover complete; traffic live", "Retry queue: 12 errors"]),
    (210, ["1900 req/s", "Recovery: 5 errors"]),
    (329, ["2138 req/s", "Standby pool drained"]),
]:
    output = "".join(frame[2] for frame in frames[:tick + 1])
    text = "\n".join("".join(row) for row in screen(output, 120, 36))
    for value in expected:
        assert value in text, f"Frame {tick} is missing {value!r}"
assert os.path.getsize(gif) < 2_000_000, "Review GIF exceeds 2 MB"
duration = float(subprocess.check_output([
    "ffprobe", "-v", "error", "-show_entries", "format=duration",
    "-of", "default=noprint_wrappers=1:nokey=1", gif,
], text=True))
assert 6 <= duration <= 12, duration
print(f"BigText: 120x36, {len(frames)} terminal frames, {duration:.2f}s, "
      f"review GIF {os.path.getsize(gif)} bytes, no alternate-screen blank tail")
PY
echo "Contact sheet: $NAME-contact.png"
