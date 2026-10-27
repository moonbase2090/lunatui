#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Extract an 8x8 bitmap face from the pinned upstream font file.

The file is not shipped. Pass its path. Requires fontTools.
"""

import hashlib
import sys
from pathlib import Path

TTF_SHA256 = "034c77f1f05ec89421e4a63f0e3a4ca1ecf852cc6d2bf611f126f275728e017d"
PIXEL = 125
DRAWN = set("%-.:0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ")
BOX = [0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF]


class FillPen:
    def __init__(self):
        self.contours = []
        self.current = None

    def moveTo(self, point):
        self.current = [point]
        self.contours.append(self.current)

    def lineTo(self, point):
        self.current.append(point)

    def curveTo(self, *_points):
        raise SystemExit("curved outlines are not part of this face")

    def qCurveTo(self, *_points):
        raise SystemExit("curved outlines are not part of this face")

    def closePath(self):
        return None

    def endPath(self):
        return None

    def addComponent(self, *_args):
        raise SystemExit("components are not part of this face")


def inside(contours, x, y):
    wind = 0
    for contour in contours:
        count = len(contour)
        for index in range(count):
            x1, y1 = contour[index]
            x2, y2 = contour[(index + 1) % count]
            if (y1 > y) != (y2 > y):
                cross = x1 + (y - y1) * (x2 - x1) / (y2 - y1)
                if x < cross:
                    wind += 1 if y2 > y1 else -1
    return wind != 0


def raster(pen):
    rows = []
    for image_y in range(8):
        bits = 0
        sample_y = ((7 - image_y) + 0.5) * PIXEL
        for image_x in range(8):
            if inside(pen.contours, (image_x + 0.5) * PIXEL, sample_y):
                bits |= 1 << (7 - image_x)
        rows.append(bits)
    return rows


def load_rows(ttf_path):
    raw = Path(ttf_path).read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != TTF_SHA256:
        raise SystemExit("font SHA-256 mismatch")
    from fontTools.ttLib import TTFont

    font = TTFont(ttf_path)
    glyph_set = font.getGlyphSet()
    cmap = font.getBestCmap()
    drawn = {}
    for char in sorted(DRAWN):
        pen = FillPen()
        glyph_set[cmap[ord(char)]].draw(pen)
        drawn[char] = raster(pen)
    rows = []
    for code in range(32, 127):
        char = chr(code)
        if char == " ":
            rows.append([0] * 8)
        elif char in drawn:
            rows.append(drawn[char])
        else:
            rows.append(BOX[:])
    return rows


def rust_file(rows):
    lines = [
        "// Copyright (c) 2026 Moonbase 2090 LLC",
        "// SPDX-License-Identifier: MIT",
        "",
        "pub(super) static ROWS: [[u8; 8]; 95] = [",
    ]
    for row in rows:
        cells = ", ".join(f"0x{value:02X}" for value in row)
        lines.append(f"    [{cells}],")
    lines.extend(
        [
            "];",
            "",
            "pub(super) fn rows(ch: char) -> &'static [u8; 8] {",
            "    let ch = match ch {",
            "        'a'..='z' => char::from(u8::try_from(u32::from(ch).wrapping_sub(32)).unwrap_or(b' ')),",
            "        other => other,",
            "    };",
            "    let code = u32::from(ch);",
            "    if (32..127).contains(&code) {",
            "        let index = usize::try_from(code - 32).unwrap_or(0);",
            "        &ROWS[index]",
            "    } else {",
            "        &ROWS[0]",
            "    }",
            "}",
            "",
        ]
    )
    return "\n".join(lines)


def main():
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {sys.argv[0]} FONT")
    sys.stdout.write(rust_file(load_rows(sys.argv[1])))


if __name__ == "__main__":
    main()
