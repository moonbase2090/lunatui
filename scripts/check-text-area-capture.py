#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify the TextArea gallery timing and replay its real terminal output.

Build widget_preview with text-area,color-profiles,lunatui-theme/graphite-tokens
first. Uses only the Python standard library. Run from the repository root.
"""

import hashlib
import json
from pathlib import Path
import struct
import subprocess


def chunks(data):
    offset = 12
    while offset < len(data):
        kind, length = struct.unpack_from('<4sI', data, offset)
        yield kind, data[offset + 8:offset + 8 + length]
        offset += 8 + length + length % 2
    assert offset == len(data), 'truncated WebP container'


def main():
    stem = Path('docs/media/text-area')
    data = stem.with_suffix('.webp').read_bytes()
    assert data[:4] == b'RIFF' and data[8:12] == b'WEBP'
    durations = [int.from_bytes(payload[12:15], 'little')
                 for kind, payload in chunks(data) if kind == b'ANMF']
    assert durations == [3000, 3000, 3000], durations
    assert len(data) <= 3_000_000, 'gallery size limit exceeded'
    png = stem.with_suffix('.png').read_bytes()
    assert png[:8] == b'\x89PNG\r\n\x1a\n'
    assert struct.unpack_from('>II', png, 16) == (2057, 1450)

    records = [json.loads(line) for line in stem.with_suffix('.cast').read_text().splitlines()]
    header, events = records[0], records[1:]
    assert len(events) == 270 and events[0][0] == 0
    assert all(event[0] == 0.033333 and event[1] == 'o' for event in events[1:])
    assert header['version'] == 3 and header['term'] == {'cols': 120, 'rows': 36}
    command = ['target/release/examples/widget_preview', '--side', 'lunatui',
               '--case', 'text-area', '--width', '120', '--height', '36',
               '--frames', '270', '--fps', '30', '--profile', 'truecolor', '--theme', 'dark']
    assert header['command'].split() == command
    recorded = ''.join(event[2] for event in events if event[1] == 'o').encode()
    current = subprocess.run(command, capture_output=True, check=True).stdout
    cleanup = b'\x1b[0m\x1b[?25h\x1b[?1049l'
    assert current == recorded + cleanup, 'capture differs from current widget output'
    print('PASS: real TextArea stream matches current source; selection/search/edit each hold 3000 ms')
    print(f'WebP: {len(data)} bytes; 2057x1450; 9000 ms; stream SHA-256 {hashlib.sha256(recorded).hexdigest()}')


if __name__ == '__main__':
    main()
