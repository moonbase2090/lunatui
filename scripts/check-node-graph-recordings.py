#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check actual node labels, connections and publication transitions in all recordings."""
import argparse
from pathlib import Path
from recording_frames import glyph_frames

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory', type=Path)
args = parser.parse_args()
# Literal terminal-cell positions in the two fixed comparison layouts.
POSITIONS = {(80, 24): [(6, 10, 4), (6, 14, 9), (25, 10, 5), (56, 12, 9)],
             (200, 60): [(36, 28, 4), (36, 32, 9), (75, 28, 5), (126, 30, 9)]}
checked = []
for (cols, rows), positions in POSITIONS.items():
    for theme in ['dark', 'light']:
        for profile in ['truecolor', 'ansi256', 'ansi16']:
            name = f'node-graph-{cols}x{rows}-{profile}-{theme}'
            sides = {}
            for side in ['lunatui', 'ratatui']:
                path = args.directory / f'{name}-{side}.gif'
                samples = [glyph_frames(path, cols, rows, x, y, cells) for x, y, cells in positions]
                if any(len(set(hashes)) != 1 for hashes in samples[:3]):
                    raise SystemExit(f'Stable node contents or connection changed: {path.name}')
                release = samples[3]
                if len(set(release[:3])) != 1 or release[3] == release[0]:
                    raise SystemExit(f'Queued/published transition is wrong: {path.name}')
                sides[side] = samples
            if sides['lunatui'] != sides['ratatui']:
                raise SystemExit(f'Actual glyph pixels differ between implementations: {name}')
            checked.append(name)
            print(f'PASS {name}: matching node labels, connection pixels and publication transition', flush=True)
print(f'PASS: {len(checked)} size/theme/color cases verified from actual decoded recordings')
