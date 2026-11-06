#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check decoded WidgetList frames against tui-widget-list across the recording matrix."""
import argparse
import hashlib
from pathlib import Path
import subprocess
from recording_frames import TIMES, glyph_frames, inspect

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory', type=Path)
args = parser.parse_args()
# Heading cell of the centered 60-column panel in the two fixed comparison layouts.
HEADINGS = {(80, 24): (12, 2), (200, 60): (72, 16)}


def frames(path):
    meta = inspect(path)
    if not 7.5 <= float(meta['duration']) <= 12:
        raise SystemExit(f'Recording missed its 30 fps timeline: {path.name}')
    result = []
    for time in TIMES:
        raw = subprocess.check_output([
            'ffmpeg', '-loglevel', 'error', '-i', str(path), '-vf', 'fps=30',
            '-ss', str(time), '-frames:v', '1', '-pix_fmt', 'rgb24', '-f', 'rawvideo', '-',
        ])
        if len(raw) != meta['width'] * meta['height'] * 3:
            raise SystemExit(f'Missing decoded frame: {path.name} at {time}')
        result.append(hashlib.sha256(raw).hexdigest())
    return result


checked = []
for (cols, rows), (column, row) in HEADINGS.items():
    for theme in ['dark', 'light']:
        for profile in ['truecolor', 'ansi256', 'ansi16']:
            name = f'widget-list-{cols}x{rows}-{profile}-{theme}'
            sides = {}
            for side in ['lunatui', 'ratatui']:
                path = args.directory / f'{name}-{side}.gif'
                whole = frames(path)
                heading = glyph_frames(path, cols, rows, column, row, 13)
                if len(set(heading)) != 1:
                    raise SystemExit(f'Heading changed while the list moved: {path.name}')
                if len(set(whole)) != len(TIMES):
                    raise SystemExit(f'Selection or scroll did not change between samples: {path.name}')
                sides[side] = whole
            if sides['lunatui'] != sides['ratatui']:
                raise SystemExit(f'Decoded frames differ between implementations: {name}')
            checked.append(name)
            print(f'PASS {name}: identical decoded frames, moving selection, fixed heading', flush=True)
print(f'PASS: {len(checked)} size/theme/color cases verified from actual decoded recordings')
