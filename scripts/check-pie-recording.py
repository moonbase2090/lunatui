#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

"""Verify held native/cells/hide/restore/selection/tick phases in a real recording."""

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
    assert receipt['args'] == ['--compare', '--hold', '96']
    assert receipt['keys'] == '1:b,2:b,3:space,4:space,5:Right,6:u'
    assert receipt['frames'] == 300
    assert receipt['duplicated_frames'] == receipt['dropped_frames'] == 0
    assert receipt['tracked_worktree_clean']
    video = args.capture / 'terminal.mkv'
    assert hashlib.sha256(video.read_bytes()).hexdigest() == receipt['sha256']['terminal.mkv']
    phases = {}
    for name, time in [('native', '0.5'), ('cells', '1.5'), ('native_again', '2.5'),
                       ('hidden', '3.5'), ('restored', '4.5'), ('selected', '5.5'), ('tick', '6.5')]:
        pixels = subprocess.check_output([
            'ffmpeg', '-nostdin', '-loglevel', 'error', '-ss', time, '-i', str(video),
            '-frames:v', '1', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-',
        ], timeout=30)
        stream = receipt['streams'][0]
        assert len(pixels) == stream['width'] * stream['height'] * 3
        phases[name] = hashlib.sha256(pixels).hexdigest()
    assert phases['native'] == phases['native_again'] == phases['restored'], 'restored native frame differs'
    assert len(set(phases.values())) == 5, 'native, cells, hidden, selection and tick must differ'
    print(json.dumps({'head': receipt['head'], 'frames': 300, 'distinct_control_states': 5,
                      'native_restored_exactly': True, 'phase_sha256': phases}, indent=2))


if __name__ == '__main__':
    main()
