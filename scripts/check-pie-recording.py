#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

"""Verify live chart motion and continuously visible native/cells output in a real recording."""

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
    assert receipt['keys'] == '7:b,9:b'
    assert receipt['frames'] == 300
    assert receipt['duplicated_frames'] == receipt['dropped_frames'] == 0
    assert receipt['tracked_worktree_clean']
    video = args.capture / 'terminal.mkv'
    assert hashlib.sha256(video.read_bytes()).hexdigest() == receipt['sha256']['terminal.mkv']
    phases = {}
    chart_frames = {}
    stream = receipt['streams'][0]
    for name, time in [('native_live_early', '0.5'), ('native_live_mid', '3.5'),
                       ('native_live_late', '6.5'), ('cells', '7.3'),
                       ('native_restored', '9.3')]:
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
    assert len(set(phases.values())) == len(phases), 'native, cells and live phases must differ'

    # Inspect the chart and legend regions in every decoded frame. These crops
    # contain only the chart area, so a deliberate hide or a rendering dropout
    # produces a uniform surface and fails the same check.
    width, height = stream['width'], stream['height']
    chart_regions = [
        (round(width * 7 / 248), round(height * 9 / 44),
         round(width * 71 / 248), round(height * 35 / 44)),
        (round(width * 131 / 248), round(height * 9 / 44),
         round(width * 195 / 248), round(height * 37 / 44)),
    ]
    for index, (x0, y0, x1, y1) in enumerate(chart_regions):
        crop_width, crop_height = x1 - x0, y1 - y0
        process = subprocess.Popen([
            'ffmpeg', '-nostdin', '-loglevel', 'error', '-i', str(video),
            '-vf', f'crop={crop_width}:{crop_height}:{x0}:{y0},format=gray',
            '-f', 'rawvideo', '-pix_fmt', 'gray', '-',
        ], stdout=subprocess.PIPE)
        assert process.stdout is not None
        frame_bytes = crop_width * crop_height
        checked = 0
        while frame := process.stdout.read(frame_bytes):
            assert len(frame) == frame_bytes
            assert len(set(frame)) >= 8, \
                f'chart panel {index} is blank in decoded frame {checked}'
            checked += 1
        assert process.wait() == 0
        assert checked == receipt['frames']

    print(json.dumps({'head': receipt['head'], 'frames': 300, 'live_motion_seconds': 7,
                      'distinct_live_samples': 3, 'distinct_control_states': 2,
                      'continuously_visible_chart_panels': 2,
                      'native_chart_crop_sha256': chart_frames, 'phase_sha256': phases}, indent=2))


if __name__ == '__main__':
    main()
