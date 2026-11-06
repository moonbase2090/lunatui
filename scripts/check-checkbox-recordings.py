#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check actual decoded Checkbox glyph pixels across the recording matrix."""
import argparse
from pathlib import Path
from recording_frames import glyph_frames

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
args = parser.parse_args()
# Literal cell positions in the two supported fixture layouts.
POSITIONS = {(80, 24): (12, 6), (200, 60): (72, 24)}



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
