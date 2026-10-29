#!/usr/bin/env python3
# Checks that the GIF renderer joins every box-drawing glyph Lunatui borders use: each corner,
# tee and cross must meet the lines beside it, with no gap and no offset. Renders chains of glyphs
# with agg and walks the strokes pixel by pixel: along a vertical chain every pixel row must touch
# the next one, and along a horizontal chain every pixel column must. A renderer that draws some
# of these glyphs from a font with other metrics fails here.
# Usage: check-box-joins.py [AGG] [LINE_HEIGHT...]  (defaults: agg, then 1.0 and 1.4)
import json
import os
import subprocess
import sys
import tempfile

VERTICAL = ["╭│├│╰", "╮│┤│╯", "┌│┼│└", "┬│┴", "╔║╚", "╗║╝", "┏┃┗"]
HORIZONTAL = ["╭─┬─╮", "╰─┴─╯", "├─┼─┤", "╔═╗", "╚═╝", "┏━┓", "┗━┛"]
FONT_SIZE = 28
LIT = 110


def render(agg, line_height, rows, cols, text, work):
    cast = os.path.join(work, "chain.cast")
    gif = os.path.join(work, "chain.gif")
    with open(cast, "w", encoding="utf-8") as out:
        out.write(json.dumps({"version": 2, "width": cols, "height": rows}) + "\n")
        out.write(json.dumps([0.1, "o", "\x1b[?25l" + text]) + "\n")
        out.write(json.dumps([0.5, "o", ""]) + "\n")
    subprocess.run(
        [agg, "--font-size", str(FONT_SIZE), "--line-height", line_height,
         "--last-frame-duration", "1", cast, gif],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    size = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v", "-show_entries",
         "stream=width,height", "-of", "csv=p=0", gif],
        check=True, capture_output=True, text=True,
    ).stdout.split()[0]
    width, height = (int(n) for n in size.split(","))
    raw = subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-i", gif, "-vf", r"select=eq(n\,1),format=gray",
         "-frames:v", "1", "-f", "rawvideo", "-"],
        check=True, capture_output=True,
    ).stdout
    if len(raw) != width * height:
        raise SystemExit(f"check-box-joins: unexpected frame size from {gif}")
    return [[raw[y * width + x] > LIT for x in range(width)] for y in range(height)]


def broken(lines):
    lit = [i for i, line in enumerate(lines) if line]
    if not lit:
        return "nothing drawn"
    for a, b in zip(range(lit[0], lit[-1]), range(lit[0] + 1, lit[-1] + 1)):
        if not lines[a] & lines[b]:
            return f"stroke breaks between pixel {a} and {b}"
    return None


def main():
    agg = sys.argv[1] if len(sys.argv) > 1 else "agg"
    heights = sys.argv[2:] or ["1.0", "1.4"]
    failures = []
    with tempfile.TemporaryDirectory() as work:
        for line_height in heights:
            for chain in VERTICAL:
                mask = render(agg, line_height, len(chain) + 1, 3, "\r\n".join(chain), work)
                rows = [{x for x, on in enumerate(row) if on} for row in mask]
                if (why := broken(rows)) is not None:
                    failures.append(f"line height {line_height}, vertical {chain}: {why}")
            for chain in HORIZONTAL:
                mask = render(agg, line_height, 2, len(chain) + 2, chain, work)
                columns = [{y for y in range(len(mask)) if mask[y][x]} for x in range(len(mask[0]))]
                if (why := broken(columns)) is not None:
                    failures.append(f"line height {line_height}, horizontal {chain}: {why}")
    for failure in failures:
        print(f"check-box-joins: {failure}", file=sys.stderr)
    if failures:
        print(f"check-box-joins: {agg} does not join Lunatui border glyphs", file=sys.stderr)
        return 1
    print(f"check-box-joins: ok ({len(heights) * (len(VERTICAL) + len(HORIZONTAL))} chains)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
