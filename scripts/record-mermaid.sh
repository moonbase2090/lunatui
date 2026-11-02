#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CASE=mermaid WIDTH=120 HEIGHT=36 FRAMES=300 FPS=30 LAST_FRAME_DURATION=0.033333
export PROFILE=truecolor THEME=dark SCALE=2 MAX_BYTES=2000000
export OUT="${OUT:-$ROOT/target/widget-showcase}"
"$ROOT/scripts/record-showcase.sh"
python3 - "$OUT/mermaid-120x36-truecolor-dark.cast" "$ROOT" <<'PYCAST'
import json
from pathlib import Path
import runpy
import sys
cast,root=sys.argv[1:]
lines=Path(cast).read_text().splitlines()
header=json.loads(lines[0]);frames=[json.loads(line) for line in lines[1:] if line.strip()]
assert header["term"]=={"cols":120,"rows":36}
assert len(frames)==300,len(frames)
screen=runpy.run_path(root+"/scripts/check-box-joins.py")["screen"]
for tick,values in [(0,["Flowchart / TD","Commit","Tests pass?","Ship"]),(100,["Flowchart / LR","Horizontal layout"]),(135,["scroll 20,0"]),(170,["Sequence preview","Publish release","Save artifact"]),(220,["Scrolled to the release result","Artifact and metadata saved"]),(299,["ASCII lines","Publish failed"] )]:
    text="\n".join("".join(row) for row in screen("".join(frame[2] for frame in frames[:tick+1]),120,36))
    for value in values: assert value in text,(tick,value)
    cells=screen("".join(frame[2] for frame in frames[:tick+1]),120,36)
    preview="\n".join("".join(row[40:118]) for row in cells[5:31])
    diagram_values={0:["Commit","Tests pass?","Ship"],135:["Ship","Inspect logs"],170:["Publish release","Save artifact"],220:["Artifact and metadata saved"],299:["Publish failed","+"]}.get(tick,[])
    for value in diagram_values: assert value in preview,(tick,"diagram",value)
    if tick==135: assert "Commit" not in preview
print("Mermaid: 300 actual frames verify flow directions, sequence messages, scrolling and ASCII fallback")
PYCAST
"${AGG:-agg}" --fps-cap 15 --font-size 14 --line-height 1.4 --last-frame-duration 0.033333 \
  "$OUT/mermaid-120x36-truecolor-dark.cast" "$OUT/mermaid-review.gif"
ffmpeg -loglevel error -y -i "$OUT/mermaid-review.gif" \
  -vf 'tpad=stop_mode=clone:stop_duration=1,fps=9/10,scale=600:-1,tile=3x3' -frames:v 1 "$OUT/mermaid-contact.png"
