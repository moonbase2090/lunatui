#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CASE=command-palette WIDTH=120 HEIGHT=36 FRAMES=300 FPS=30 LAST_FRAME_DURATION=0.033333
export PROFILE=truecolor THEME=dark SCALE=2 MAX_BYTES=2000000
export OUT="${OUT:-$ROOT/target/widget-showcase}"
"$ROOT/scripts/record-showcase.sh"
python3 - "$OUT/command-palette-120x36-truecolor-dark.cast" "$ROOT" <<'PYCAST'
import json
from pathlib import Path
import runpy
import sys
cast,root=sys.argv[1:]
lines=Path(cast).read_text().splitlines()
header=json.loads(lines[0])
frames=[json.loads(line) for line in lines[1:] if line.strip()]
assert header["term"]=={"cols":120,"rows":36}
assert len(frames)==300,len(frames)
screen=runpy.run_path(root+"/scripts/check-box-joins.py")["screen"]
for tick,expected in [(0,"8 commands"),(40,"1 command"),(80,"Opened src/session.rs"),(145,"3 commands"),(205,"Split editor right"),(255,"No matching commands"),(299,"Cancelled palette")]:
    text="\n".join("".join(row) for row in screen("".join(frame[2] for frame in frames[:tick+1]),120,36))
    assert expected in text,(tick,expected)
    if tick==205: assert "tests/session_test.rs" in text
print("Command palette: 300 actual frames verify fuzzy queries, command execution, split geometry and cancellation")
PYCAST

"${AGG:-agg}" --fps-cap 15 --font-size 14 --line-height 1.4 --last-frame-duration 0.033333 \
  "$OUT/command-palette-120x36-truecolor-dark.cast" "$OUT/command-palette-review.gif"
ffmpeg -loglevel error -y -i "$OUT/command-palette-review.gif" \
  -vf 'tpad=stop_mode=clone:stop_duration=1,fps=9/10,scale=600:-1,tile=3x3' -frames:v 1 \
  "$OUT/command-palette-contact.png"
