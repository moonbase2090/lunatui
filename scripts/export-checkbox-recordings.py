#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify real Checkbox recordings, then export gallery media and source hashes."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory', type=Path)
args = parser.parse_args()
subprocess.run([sys.executable, str(ROOT / 'scripts/check-checkbox-recordings.py'),
                str(args.directory)], check=True)
out = ROOT / 'docs/media'
artifacts = []
for size in ['80x24', '200x60']:
    for theme in ['dark', 'light']:
        for profile in ['truecolor', 'ansi256', 'ansi16']:
            name = f'checkbox-{size}-{profile}-{theme}'
            video = f'{name}-side-by-side.mp4'
            shutil.copyfile(args.directory / video, out / video)
            artifacts.append(video)
            for side in ['lunatui', 'ratatui']:
                png = f'{name}-{side}.png'
                subprocess.run(['ffmpeg', '-loglevel', 'error', '-y', '-ss', '6.5',
                                '-i', str(args.directory / f'{name}-{side}.gif'),
                                '-frames:v', '1', str(out / png)], check=True)
                artifacts.append(png)
gallery = 'checkbox-80x24-truecolor-dark.gif'
shutil.copyfile(args.directory / 'checkbox-80x24-truecolor-dark-lunatui.gif', out / gallery)
artifacts.append(gallery)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


sources = ['benches/src/widget_suite/checkbox.rs', 'examples/widget_preview.rs',
           'lunatui-choice/src/lib.rs', 'lunatui-ui-tokens/src/lib.rs']
manifest = {
    'source_sha256': {source: digest(ROOT / source) for source in sources},
    'baseline': 'tui-checkbox 0.5.3, default features disabled',
    'renderer': 'agg 1.9.0+lunatui.2',
    'scripted_frames': 240, 'fps': 30, 'screenshot_second': 6.5,
    'pixel_check': '12 size/theme/color cases passed from actual decoded recordings',
    'artifacts': [{'file': file, 'bytes': (out / file).stat().st_size,
                   'sha256': digest(out / file)} for file in artifacts],
}
(out / 'checkbox-proof.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(f'Exported {len(artifacts)} verified recording artifacts')
