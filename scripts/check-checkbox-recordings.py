#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check actual decoded Checkbox glyph pixels across the recording matrix."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
args = parser.parse_args()
TIMES = [0.5, 2.5, 4.5, 6.5]
# Literal cell positions in the two supported fixture layouts.
POSITIONS = {(80, 24): (12, 6), (200, 60): (72, 24)}


def inspect(path):
    data = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-show_streams", "-of", "json", str(path),
    ], text=True))
    return data["streams"][0]


def glyph_frames(path, cols, rows, column, row):
    meta = inspect(path)
    if not 7.5 <= float(meta["duration"]) <= 12:
        raise SystemExit(f"Recording missed its 30 fps timeline: {path.name}")
    # Pinned agg adds one column and half a row of padding on each side.
    cell_w = meta["width"] / (cols + 2)
    cell_h = meta["height"] / (rows + 1)
    left = round((1 + column) * cell_w)
    top = round((0.5 + row) * cell_h)
    width = round((4 + column) * cell_w) - left
    height = round((1.5 + row) * cell_h) - top
    result = []
    for time in TIMES:
        raw = subprocess.check_output([
            "ffmpeg", "-loglevel", "error", "-ss", str(time), "-i", str(path),
            "-vf", f"crop={width}:{height}:{left}:{top}", "-frames:v", "1",
            "-pix_fmt", "rgb24", "-f", "rawvideo", "-",
        ])
        if len(raw) != width * height * 3:
            raise SystemExit(f"Missing decoded glyph pixels: {path.name} at {time}")
        result.append(hashlib.sha256(raw).hexdigest())
    return result


checked = []
for (cols, rows), (column, first_row) in POSITIONS.items():
    for theme in ["dark", "light"]:
        for profile in ["truecolor", "ansi256", "ansi16"]:
            name = f"checkbox-{cols}x{rows}-{profile}-{theme}"
            sides = {}
            for side in ["lunatui", "ratatui"]:
                path = args.directory / f"{name}-{side}.gif"
                first = glyph_frames(path, cols, rows, column, first_row)
                disabled = glyph_frames(path, cols, rows, column, first_row + 8)
                group = glyph_frames(path, cols, rows, column, first_row + 10)
                if first[0] == first[1] or len(set(first[1:])) != 1:
                    raise SystemExit(f"Unchecked/checked transition is wrong: {path.name}")
                if len(set(disabled)) != 1:
                    raise SystemExit(f"Disabled choice changed: {path.name}")
                if group[1] != group[2] or len({group[0], group[1], group[3]}) != 3:
                    raise SystemExit(f"Unchecked/mixed/checked group is wrong: {path.name}")
                sides[side] = {"first": first, "disabled": disabled, "group": group}
            if sides["lunatui"] != sides["ratatui"]:
                raise SystemExit(f"Actual glyph pixels differ between implementations: {name}")
            checked.append(name)
            print(f"PASS {name}: matching glyph pixels, three states and fixed disabled choice", flush=True)
print(f"PASS: {len(checked)} size/theme/color cases verified from actual decoded recordings")
