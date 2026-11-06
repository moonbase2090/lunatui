#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check actual node labels, connections and publication transitions in all recordings."""
import argparse
from pathlib import Path
from recording_frames import cell_frames, glyph_frames

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory', type=Path)
args = parser.parse_args()
# Literal terminal-cell positions in the two fixed comparison layouts.
POSITIONS = {(80, 24): [(6, 10, 4), (6, 14, 9), (25, 10, 5), (56, 12, 9)],
             (200, 60): [(36, 28, 4), (36, 32, 9), (75, 28, 5), (126, 30, 9)]}
CAPTIONS = ['01  Repository synced', '02  Checks passed',
            '03  Package prepared', '04  Release published']
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
                for step, caption in enumerate(CAPTIONS):
                    first_blank = 3 + len(caption)
                    tail = cell_frames(path, cols, rows, first_blank, rows - 3,
                                       cols - 3 - first_blank, times=[0.5 + step * 2])[0]
                    if len({tail[i:i + 3] for i in range(0, len(tail), 3)}) != 1:
                        raise SystemExit(f'Stale caption glyphs after step {step + 1}: {path.name}')
                sides[side] = samples
            if sides['lunatui'] != sides['ratatui']:
                raise SystemExit(f'Actual glyph pixels differ between implementations: {name}')
            checked.append(name)
            print(f'PASS {name}: matching node labels, connection pixels, publication transition and blank caption tails', flush=True)
print(f'PASS: {len(checked)} size/theme/color cases verified from actual decoded recordings')
