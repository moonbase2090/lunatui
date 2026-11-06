#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify and export the actual Kitty node graph capture without changing its pixels."""
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
checks = json.loads(subprocess.check_output([sys.executable, str(ROOT / 'scripts/check-node-graph-pixels.py'), str(args.directory)], text=True))
receipt = json.loads((args.directory / 'capture.json').read_text())
assert receipt['tracked_worktree_clean'] and receipt['frames'] == 240
out = ROOT / 'docs/media'
artifacts = []
for suffix in ['webp', 'png']:
    source = args.directory / f'terminal.{suffix}'
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    assert digest == receipt['sha256'][f'terminal.{suffix}']
    name = f'node-graph-kitty.{suffix}'
    shutil.copyfile(source, out / name)
    artifacts.append({'file': name, 'bytes': source.stat().st_size, 'sha256': digest})
manifest = {'source_commit': receipt['head'], 'source': receipt['source'],
            'terminal_configuration': receipt['terminal_configuration'],
            'kitty': receipt['kitty'], 'frames': receipt['frames'],
            'duration_seconds': float(receipt['format']['duration']),
            'duplicated_frames': receipt['duplicated_frames'], 'dropped_frames': receipt['dropped_frames'],
            'keys': receipt['keys'], 'checks': checks, 'artifacts': artifacts}
(out / 'node-graph-pixel-proof.json').write_text(json.dumps(manifest, indent=2) + '\n')
print('Exported actual Kitty recording and screenshot; pixel/cell/pixel restoration verified')
