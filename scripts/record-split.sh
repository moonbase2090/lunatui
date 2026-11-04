#!/usr/bin/env bash
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
# Record real pointer/keyboard resizing and prove that restoring recovers the saved layout.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CASE=split WIDTH=120 HEIGHT=36 FRAMES=330 FPS=30 LAST_FRAME_DURATION=0.033333
export PROFILE=truecolor THEME=dark SCALE=2 MAX_BYTES=2000000
export OUT="${OUT:-$ROOT/target/widget-showcase}"
"$ROOT/scripts/record-showcase.sh"
python3 - "$OUT/split-120x36-truecolor-dark.cast" "$ROOT" <<'PY'
import json
from pathlib import Path
import runpy
import sys

cast, root = sys.argv[1:]
with open(cast, encoding="utf-8") as source:
    header = json.loads(next(source))
    frames = [json.loads(line) for line in source if line.strip()]
assert header["term"] == {"cols": 120, "rows": 36}
assert len(frames) == 330, len(frames)
screen = runpy.run_path(str(Path(root) / "scripts/check-box-joins.py"))["screen"]
states = {}
for tick, expected in [
    (195, "resized Editor +4"),
    (227, "resized Terminal -2"),
    (243, "saved the layout"),
    (255, "resized Files +8"),
    (287, "resized Terminal +3"),
    (303, "restored the saved layout"),
]:
    output = "".join(frame[2] for frame in frames[:tick + 1])
    cells = screen(output, 120, 36)
    text = "\n".join("".join(row) for row in cells)
    assert expected in text, (tick, expected)
    states[tick] = cells[:-1]
assert states[243] != states[287], "Resizing must change the saved layout"
assert states[243] == states[303], "Restoring must recover the saved pane geometry"
print("Split showcase: 330 frames, keyboard/save/restore labels and restored geometry verified")
PY
