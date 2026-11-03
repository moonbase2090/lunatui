#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
# Records the real release-plan screen and verifies its follow, overview and handoff states.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CASE=timeline WIDTH=120 HEIGHT=36 FRAMES=300 FPS=30 LAST_FRAME_DURATION=0.033333
export PROFILE=truecolor THEME=dark SCALE=2 MAX_BYTES=2000000
export OUT="${OUT:-$ROOT/target/widget-showcase}"
AGG="${AGG:-agg}"
"$ROOT/scripts/record-showcase.sh"
NAME="$OUT/timeline-120x36-truecolor-dark"
"$AGG" --fps-cap 15 --font-size 14 --line-height 1.4 --last-frame-duration 0.033333 \
  "$NAME.cast" "$NAME-review.gif"
ffmpeg -loglevel error -y -i "$NAME-review.gif" \
  -vf 'fps=9/10,scale=600:-1,tile=3x3' -frames:v 1 "$NAME-contact.png"
python3 - "$NAME" "$ROOT" <<'PY'
import json
from pathlib import Path
import runpy
import subprocess
import sys

name, root = sys.argv[1:]
with open(name + ".cast", encoding="utf-8") as source:
    header = json.loads(next(source))
    frames = [json.loads(line) for line in source if line.strip()]
assert header["term"]["cols"] == 120 and header["term"]["rows"] == 36
assert len(frames) == 300, len(frames)
assert all("\x1b[?1049l" not in frame[2] for frame in frames)
screen = runpy.run_path(str(Path(root) / "scripts/check-box-joins.py"))["screen"]
for tick, expected in [
    (0, ["release plan / lunatui 0.2", "NOW  Thu 05 02:00", "8 days, following now",
         "PLANNED / on track"]),
    (100, ["focus trap", "BLOCKED / waiting on fix", "focus trap leaks"]),
    (180, ["16 days, weekly ticks", "Nov 09", "0.2 GA"]),
    (299, ["wasm sandbox", "AT RISK / review owed", "widgets/timeline done"]),
]:
    output = "".join(frame[2] for frame in frames[:tick + 1])
    text = "\n".join("".join(row) for row in screen(output, 120, 36))
    for value in expected:
        assert value in text, f"Frame {tick} is missing {value!r}"
gif = Path(name + "-review.gif")
assert gif.stat().st_size < 2_000_000
duration = float(subprocess.check_output([
    "ffprobe", "-v", "error", "-show_entries", "format=duration",
    "-of", "default=noprint_wrappers=1:nokey=1", str(gif),
], text=True))
assert 6 <= duration <= 12, duration
print(f"Timeline: 120x36, 300 terminal frames, {duration:.2f}s, "
      f"review GIF {gif.stat().st_size} bytes, follow/overview/handoff assertions pass")
PY
