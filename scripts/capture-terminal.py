#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Capture every actual synchronized frame from the live child-PTY example."""
import json
from pathlib import Path
import subprocess
import sys
import time

root = Path(__file__).resolve().parent.parent
out, width, height, frames, fps, profile, theme = sys.argv[1:]
width, height, frames, fps = map(int, (width, height, frames, fps))
command = [
    "target/release/examples/embedded_terminal", "--scripted",
    "--workspace", "target/terminal-demo", "--width", str(width),
    "--height", str(height), "--frames", str(frames), "--fps", str(fps),
    "--profile", profile, "--theme", theme,
]
started = time.time()
result = subprocess.run(command, cwd=root, capture_output=True, timeout=frames / fps + 30)
if result.returncode:
    raise SystemExit(result.stderr.decode("utf-8", errors="replace"))
text = result.stdout.decode("utf-8")
prefix, *parts = text.split("\x1b[?2026h")
assert len(parts) == frames, len(parts)
header = {
    "version": 2, "width": width, "height": height,
    "timestamp": int(started), "duration": frames / fps,
    "command": " ".join(command),
}
with Path(out).open("w") as target:
    target.write(json.dumps(header) + "\n")
    for tick, part in enumerate(parts):
        payload, marker, _ = part.partition("\x1b[?2026l")
        assert marker, tick
        data = (prefix if tick == 0 else "") + "\x1b[?2026h" + payload + marker
        target.write(json.dumps([tick / fps, "o", data]) + "\n")
print(f"Captured {frames} actual PTY app frames at {width}x{height}, {profile}, {theme}")
