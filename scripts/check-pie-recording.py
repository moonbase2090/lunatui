#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

"""Verify live native chart motion and native/cells/hide/restore phases in a real recording."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=Path, help='Directory produced by record-pixels.sh')
    args = parser.parse_args()
    receipt = json.loads((args.capture / 'capture.json').read_text())
    assert receipt['example'] == 'pixel_pie_chart'
    assert receipt['args'] == ['--compare']
    assert receipt['keys'] == '7:b,7.7:space,8.3:space,9:b'
    assert receipt['frames'] == 300
    assert receipt['duplicated_frames'] == receipt['dropped_frames'] == 0
    assert receipt['tracked_worktree_clean']
    video = args.capture / 'terminal.mkv'
    assert hashlib.sha256(video.read_bytes()).hexdigest() == receipt['sha256']['terminal.mkv']
    phases = {}
    chart_frames = {}
    stream = receipt['streams'][0]
    for name, time in [('native_live_early', '0.5'), ('native_live_mid', '3.5'),
                       ('native_live_late', '6.5'), ('cells', '7.3'), ('hidden', '8.0'),
                       ('restored_cells', '8.6'), ('native_restored', '9.3')]:
        pixels = subprocess.check_output([
            'ffmpeg', '-nostdin', '-loglevel', 'error', '-ss', time, '-i', str(video),
            '-frames:v', '1', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-',
        ], timeout=30)
        assert len(pixels) == stream['width'] * stream['height'] * 3
        phases[name] = hashlib.sha256(pixels).hexdigest()
        if name.startswith('native_live_'):
            width, height = stream['width'], stream['height']
            x0, x1 = round(width * 131 / 248), round(width * 195 / 248)
            y0, y1 = round(height * 9 / 44), round(height * 37 / 44)
            crop = b''.join(
                pixels[(y * width + x0) * 3:(y * width + x1) * 3]
                for y in range(y0, y1)
            )
            chart_frames[name] = hashlib.sha256(crop).hexdigest()
    assert len(set(chart_frames.values())) == 3, \
        'the native pie and legend region must change during the seven-second live segment'
    assert len(set(phases.values())) == len(phases), 'native, cells, hidden, restore and live phases must differ'
    print(json.dumps({'head': receipt['head'], 'frames': 300, 'live_motion_seconds': 7,
                      'distinct_live_samples': 3, 'distinct_control_states': 4,
                      'native_chart_crop_sha256': chart_frames, 'phase_sha256': phases}, indent=2))


if __name__ == '__main__':
    main()
