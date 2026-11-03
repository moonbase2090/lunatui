#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Assert actual child output and focus routing in the recorded workspace."""
import json
from pathlib import Path
import runpy
import sys

root = Path(__file__).resolve().parent.parent
lines = Path(sys.argv[1]).read_text().splitlines()
header = json.loads(lines[0])
width, height = header["width"], header["height"]
frames = [json.loads(line) for line in lines[1:]]
assert len(frames) == 300, len(frames)
assert all("\x1b[?1049l" not in frame[2] for frame in frames)
screen = runpy.run_path(str(root / "scripts/check-box-joins.py"))["screen"]
for tick in [0, 110, 200, 219, 299]:
    cells = screen("".join(frame[2] for frame in frames[:tick + 1]), width, height)
    text = "\n".join("".join(row) for row in cells)
    assert "PTY error:" not in text, (tick, text)
    assert "ORBIT" in text and "src/lib.rs" in text, tick
    left = width // 3 + 3
    terminal = ["".join(row[left:width - 2]).strip() for row in cells[5:height - 4]]
    if tick == 0:
        assert "Controlling terminal attached" in "\n".join(terminal)
    if tick == 110:
        assert "2 passed; 0 failed" in "\n".join(terminal), terminal
    if tick in [200, 219]:
        assert "Editor focused" in text
        assert "orbit % x" not in "\n".join(terminal)
    if tick == 299:
        assert "PTY ready" in terminal, terminal
        assert "Terminal focused" in text
print("Live PTY cast verifies real test results, paused input, restored focus and final child output")
