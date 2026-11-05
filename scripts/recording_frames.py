# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Decode actual glyph pixels from pinned agg recordings."""
import hashlib
import json
import subprocess
TIMES = [0.5, 2.5, 4.5, 6.5]

def inspect(path):
    data = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-show_streams", "-of", "json", str(path),
    ], text=True))
    return data["streams"][0]


def glyph_frames(path, cols, rows, column, row, cells=3):
    meta = inspect(path)
    if not 7.5 <= float(meta["duration"]) <= 12:
        raise SystemExit(f"Recording missed its 30 fps timeline: {path.name}")
    # Pinned agg adds one column and half a row of padding on each side.
    cell_w = meta["width"] / (cols + 2)
    cell_h = meta["height"] / (rows + 1)
    left = round((1 + column) * cell_w)
    top = round((0.5 + row) * cell_h)
    width = round((1 + cells + column) * cell_w) - left
    height = round((1.5 + row) * cell_h) - top
    result = []
    for time in TIMES:
        raw = subprocess.check_output([
            "ffmpeg", "-loglevel", "error", "-i", str(path),
            "-vf", f"fps=30,crop={width}:{height}:{left}:{top}", "-ss", str(time), "-frames:v", "1",
            "-pix_fmt", "rgb24", "-f", "rawvideo", "-",
        ])
        if len(raw) != width * height * 3:
            raise SystemExit(f"Missing decoded glyph pixels: {path.name} at {time}")
        result.append(hashlib.sha256(raw).hexdigest())
    return result
