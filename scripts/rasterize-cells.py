#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Rasterize a captured terminal frame into a PNG, one flat color per cell.

Usage: rasterize-cells.py --fonts DIR [--cols 64] [--rows 24] IN.ansi OUT.png [IN OUT ...]

The input is the escape-sequence stream a lunatui example writes for one
frame. Each cell becomes an 18x36 pixel block (9x18 at 1x). Box drawing,
block, shade, and braille glyphs are drawn as shapes that fill the cell, the
way GPU terminals draw them, so lines join across cells. Other glyphs come
from JetBrains Mono, with DejaVu Sans Mono and DejaVu Sans as fallbacks.

The fonts are pinned by SHA-256 and the output is only reproducible with the
pinned Pillow and FreeType; scripts/capture-site-widgets.sh runs this inside
a pinned container.
"""

import argparse
import hashlib
import re
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont

CW, CH = 18, 36
LW = 2
FONTS = {
    "JetBrainsMono-wght.ttf": "48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda",
    "DejaVuSansMono.ttf": "54bf827eb99404e8f430c330ad30f063334f637eba0109b6a18d4f566a8e9dd8",
    "DejaVuSans.ttf": "57f73e11f51999432bf7ab22ce55b6f945d5eca1bf824404cfa9ec2e3718c84e",
}
DEFAULT_FG = "#ECE7DA"
DEFAULT_BG = "#0A0D1C"
CSI = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z])")
OSC = re.compile(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")


class Fonts:
    def __init__(self, directory):
        paths = {}
        for name, digest in FONTS.items():
            path = Path(directory) / name
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != digest:
                sys.exit(f"{path}: sha256 {actual}, expected {digest}")
            paths[name] = str(path)
        mono = paths["JetBrainsMono-wght.ttf"]
        self.regular = ImageFont.truetype(mono, 28)
        self.regular.set_variation_by_name("Regular")
        self.bold = ImageFont.truetype(mono, 28)
        self.bold.set_variation_by_name("Bold")
        self.fallbacks = [
            (set(TTFont(paths["DejaVuSansMono.ttf"]).getBestCmap()),
             ImageFont.truetype(paths["DejaVuSansMono.ttf"], 26)),
            (set(TTFont(paths["DejaVuSans.ttf"]).getBestCmap()),
             ImageFont.truetype(paths["DejaVuSans.ttf"], 24)),
        ]
        self.mono_cmap = set(TTFont(mono).getBestCmap())

    def pick(self, ch, bold):
        if ord(ch) in self.mono_cmap:
            return self.bold if bold else self.regular
        for cmap, font in self.fallbacks:
            if ord(ch) in cmap:
                return font
        sys.exit(f"no pinned font has U+{ord(ch):04X}")


def parse(stream, cols, rows):
    grid = [[[" ", DEFAULT_FG, DEFAULT_BG, False] for _ in range(cols)] for _ in range(rows)]
    fg, bg, bold = DEFAULT_FG, DEFAULT_BG, False
    x = y = 0
    i = 0
    while i < len(stream):
        ch = stream[i]
        if ch == "\x1b":
            osc = OSC.match(stream, i)
            if osc:
                i = osc.end()
                continue
            csi = CSI.match(stream, i)
            if not csi:
                i += 1
                continue
            params, command = csi.group(1), csi.group(2)
            i = csi.end()
            if command == "H":
                parts = [int(p) if p else 1 for p in params.split(";")] if params else [1, 1]
                y = parts[0] - 1
                x = (parts[1] if len(parts) > 1 else 1) - 1
            elif command == "m":
                codes = [int(p) if p else 0 for p in params.split(";")] if params else [0]
                k = 0
                while k < len(codes):
                    code = codes[k]
                    if code == 0:
                        fg, bg, bold = DEFAULT_FG, DEFAULT_BG, False
                    elif code == 1:
                        bold = True
                    elif code == 22:
                        bold = False
                    elif code == 39:
                        fg = DEFAULT_FG
                    elif code == 49:
                        bg = DEFAULT_BG
                    elif code in (38, 48) and k + 4 < len(codes) and codes[k + 1] == 2:
                        color = "#%02X%02X%02X" % tuple(codes[k + 2:k + 5])
                        if code == 38:
                            fg = color
                        else:
                            bg = color
                        k += 4
                    k += 1
            continue
        if ch == "\r":
            x = 0
        elif ch == "\n":
            y += 1
        else:
            if 0 <= y < rows and 0 <= x < cols:
                grid[y][x] = [ch, fg, bg, bold]
            x += 1
        i += 1
    return grid


def shape(draw, ch, left, top, fg):
    cx, cy = left + CW // 2, top + CH // 2
    eighths_up = {"▁": 1, "▂": 2, "▃": 3, "▄": 4, "▅": 5, "▆": 6, "▇": 7, "█": 8}
    eighths_left = {"▏": 1, "▎": 2, "▍": 3, "▌": 4, "▋": 5, "▊": 6, "▉": 7}
    if ch in eighths_up:
        h = round(CH * eighths_up[ch] / 8)
        draw.rectangle([left, top + CH - h, left + CW - 1, top + CH - 1], fill=fg)
        return True
    if ch in eighths_left:
        w = round(CW * eighths_left[ch] / 8)
        draw.rectangle([left, top, left + w - 1, top + CH - 1], fill=fg)
        return True
    if ch == "▀":
        draw.rectangle([left, top, left + CW - 1, top + CH // 2 - 1], fill=fg)
        return True
    if ch == "▐":
        draw.rectangle([left + CW // 2, top, left + CW - 1, top + CH - 1], fill=fg)
        return True
    if ch in "░▒":
        step = 4 if ch == "░" else 2
        for yy in range(top, top + CH, step):
            for xx in range(left + (yy // step % 2) * (step // 2), left + CW, step):
                draw.point((xx, yy), fill=fg)
        return True
    if 0x2800 <= ord(ch) <= 0x28FF:
        bits = ord(ch) - 0x2800
        dots = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3)]
        for index, (px, py) in enumerate(dots):
            if bits >> index & 1:
                ox, oy = left + 4 + px * 8, top + 3 + py * 9
                draw.ellipse([ox, oy, ox + 5, oy + 5], fill=fg)
        return True
    arms = {"─": "lr", "━": "LR", "│": "ud", "├": "udr", "┤": "udl", "┬": "lrd",
            "┴": "lru", "┼": "udlr", "╶": "r", "╴": "l"}
    if ch in arms:
        spec = arms[ch]
        w = LW * 2 if spec.isupper() else LW
        spec = spec.lower()
        if "l" in spec:
            draw.rectangle([left, cy - w // 2, cx, cy - w // 2 + w - 1], fill=fg)
        if "r" in spec:
            draw.rectangle([cx, cy - w // 2, left + CW - 1, cy - w // 2 + w - 1], fill=fg)
        if "u" in spec:
            draw.rectangle([cx - w // 2, top, cx - w // 2 + w - 1, cy], fill=fg)
        if "d" in spec:
            draw.rectangle([cx - w // 2, cy, cx - w // 2 + w - 1, top + CH - 1], fill=fg)
        return True
    corners = {"╭": ("r", "d"), "╮": ("l", "d"), "╰": ("r", "u"), "╯": ("l", "u")}
    if ch in corners:
        h, v = corners[ch]
        r = CW // 2
        ex = left + CW - 1 if h == "r" else left
        ey = top + CH - 1 if v == "d" else top
        acx = cx + (r if h == "r" else -r)
        acy = cy + (r if v == "d" else -r)
        if h == "r":
            draw.line([(acx, cy), (ex, cy)], fill=fg, width=LW)
        else:
            draw.line([(ex, cy), (acx, cy)], fill=fg, width=LW)
        draw.line([(cx, acy), (cx, ey)], fill=fg, width=LW)
        start = {("r", "d"): 180, ("l", "d"): 270, ("r", "u"): 90, ("l", "u"): 0}[(h, v)]
        draw.arc([acx - r, acy - r, acx + r, acy + r], start, start + 90, fill=fg, width=LW)
        return True
    return False


def render(grid, fonts, out):
    rows, cols = len(grid), len(grid[0])
    image = Image.new("RGB", (cols * CW, rows * CH))
    draw = ImageDraw.Draw(image)
    for y in range(rows):
        for x in range(cols):
            ch, fg, bg, bold = grid[y][x]
            left, top = x * CW, y * CH
            draw.rectangle([left, top, left + CW - 1, top + CH - 1], fill=bg)
            if ch == " " or shape(draw, ch, left, top, fg):
                continue
            draw.text((left + CW / 2, top + CH / 2 + 1), ch, font=fonts.pick(ch, bold),
                      fill=fg, anchor="mm")
    image.save(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--fonts", required=True)
    parser.add_argument("--cols", type=int, default=64)
    parser.add_argument("--rows", type=int, default=24)
    parser.add_argument("pairs", nargs="+")
    args = parser.parse_args()
    if len(args.pairs) % 2:
        parser.error("pass IN.ansi OUT.png pairs")
    fonts = Fonts(args.fonts)
    for source, out in zip(args.pairs[::2], args.pairs[1::2]):
        stream = Path(source).read_text(encoding="utf-8")
        render(parse(stream, args.cols, args.rows), fonts, out)
        print(f"rasterized {out}")


if __name__ == "__main__":
    main()
