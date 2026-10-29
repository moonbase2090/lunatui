#!/usr/bin/env python3
# Checks that the GIF renderer joins every box-drawing glyph Lunatui borders use: each corner,
# tee and cross must meet the lines beside it, with no gap and no offset.
#
# Without --demo it renders chains of every Lunatui border glyph and walks the strokes pixel by
# pixel: along a vertical chain every pixel row must touch the next one (diagonal neighbours
# count), and along a horizontal chain every pixel column must.
#
# With --demo CASE it renders the last frame of a widget_preview case instead, finds every pair
# of neighbouring box-drawing cells whose strokes meet (a corner and the line beside it, a tee and
# its stem), and checks that the stroke crosses each cell edge without a gap or a step.
#
# A renderer that draws some of these glyphs from a font with other metrics fails here.
# Usage: check-box-joins.py [--agg AGG] [--line-height H ...] [--demo CASE ...] [--preview BIN]
#        [--width W --height H --frames N]
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import unicodedata

VERTICAL = ["╭│├│╰", "╮│┤│╯", "┌│┼│└", "┬│┴", "╔║╚", "╗║╝", "┏┃┗"]
HORIZONTAL = ["╭─┬─╮", "╰─┴─╯", "├─┼─┤", "╔═╗", "╚═╝", "┏━┓", "┗━┛"]
FONT_SIZE = 28
LIT = 110

# Which sides of the cell each glyph's strokes reach: up, right, down, left.
REACH = {
    "─": "rl", "━": "rl", "═": "rl", "│": "ud", "┃": "ud", "║": "ud",
    "┌": "rd", "┐": "dl", "└": "ur", "┘": "ul", "╭": "rd", "╮": "dl", "╰": "ur", "╯": "ul",
    "┏": "rd", "┓": "dl", "┗": "ur", "┛": "ul", "╔": "rd", "╗": "dl", "╚": "ur", "╝": "ul",
    "├": "urd", "┤": "udl", "┬": "rdl", "┴": "url", "┼": "urdl",
}


def run_agg(agg, line_height, cast, gif):
    subprocess.run(
        [agg, "--font-size", str(FONT_SIZE), "--line-height", line_height,
         "--last-frame-duration", "1", cast, gif],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )


def last_frame(gif, pix_fmt):
    size = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v", "-show_entries",
         "stream=width,height", "-of", "csv=p=0", gif],
        check=True, capture_output=True, text=True,
    ).stdout.split()[0]
    width, height = (int(n) for n in size.split(","))
    raw = subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-i", gif, "-vf", rf"select=eq(n\,1),format={pix_fmt}",
         "-frames:v", "1", "-f", "rawvideo", "-"],
        check=True, capture_output=True,
    ).stdout
    depth = 3 if pix_fmt == "rgb24" else 1
    if len(raw) != width * height * depth:
        raise SystemExit(f"check-box-joins: unexpected frame size from {gif}")
    return width, height, raw


def write_cast(path, cols, rows, text):
    with open(path, "w", encoding="utf-8") as out:
        out.write(json.dumps({"version": 2, "width": cols, "height": rows}) + "\n")
        out.write(json.dumps([0.1, "o", "\x1b[?25l" + text]) + "\n")
        out.write(json.dumps([0.5, "o", ""]) + "\n")


def broken(lines):
    lit = [i for i, line in enumerate(lines) if line]
    if not lit:
        return "nothing drawn"
    for a, b in zip(range(lit[0], lit[-1]), range(lit[0] + 1, lit[-1] + 1)):
        if not any(abs(u - v) <= 1 for u in lines[a] for v in lines[b]):
            return f"stroke breaks between pixel {a} and {b}"
    return None


def check_chains(agg, heights, work):
    failures = []
    cast, gif = os.path.join(work, "chain.cast"), os.path.join(work, "chain.gif")
    for line_height in heights:
        for vertical, chains in ((True, VERTICAL), (False, HORIZONTAL)):
            for chain in chains:
                cols, rows = (3, len(chain) + 1) if vertical else (len(chain) + 2, 2)
                write_cast(cast, cols, rows, "\r\n".join(chain) if vertical else chain)
                run_agg(agg, line_height, cast, gif)
                width, height, raw = last_frame(gif, "gray")
                mask = [[raw[y * width + x] > LIT for x in range(width)] for y in range(height)]
                if vertical:
                    lines = [{x for x, on in enumerate(row) if on} for row in mask]
                else:
                    lines = [{y for y in range(height) if mask[y][x]} for x in range(width)]
                if (why := broken(lines)) is not None:
                    kind = "vertical" if vertical else "horizontal"
                    failures.append(f"line height {line_height}, {kind} {chain}: {why}")
    return failures, sum(len(VERTICAL) + len(HORIZONTAL) for _ in heights)


def screen(output, cols, rows):
    grid = [[" "] * cols for _ in range(rows)]
    row = col = 0
    for token in re.split(r"(\x1b\[[0-9;?]*[A-Za-z@]|\x1b[^\[])", output):
        if not token:
            continue
        if token.startswith("\x1b"):
            if token.endswith("H") and token.startswith("\x1b["):
                parts = (token[2:-1].split(";") + ["1", "1"])[:2]
                row, col = (int(p or 1) - 1 for p in parts)
            continue
        for ch in token:
            if ch in "\r\n":
                continue
            if 0 <= row < rows and 0 <= col < cols:
                grid[row][col] = ch
            col += 2 if unicodedata.east_asian_width(ch) in "WF" else 1
    return grid


def check_demo(agg, heights, preview, case, size, frames, work):
    cols, rows = size
    output = subprocess.run(
        [preview, "--side", "lunatui", "--case", case, "--width", str(cols), "--height", str(rows),
         "--frames", str(frames), "--fps", "1000", "--profile", "truecolor", "--theme", "dark"],
        check=True, capture_output=True,
    ).stdout.decode("utf-8", "replace")
    output = re.sub(r"\x1b\[\?(1049|2026)[hl]", "", output)
    grid = screen(output, cols, rows)
    joins = []
    for y in range(rows):
        for x in range(cols):
            here = REACH.get(grid[y][x], "")
            if "r" in here and x + 1 < cols and "l" in REACH.get(grid[y][x + 1], ""):
                if set(here) != set("rl") or set(REACH[grid[y][x + 1]]) != set("rl"):
                    joins.append(("h", x, y))
            if "d" in here and y + 1 < rows and "u" in REACH.get(grid[y + 1][x], ""):
                if set(here) != set("ud") or set(REACH[grid[y + 1][x]]) != set("ud"):
                    joins.append(("v", x, y))
    if not joins:
        return [], 0
    failures = []
    cast, gif = os.path.join(work, "demo.cast"), os.path.join(work, "demo.gif")
    write_cast(cast, cols, rows, "\x1b[2J" + output + "\x1b[?25l")
    for line_height in heights:
        run_agg(agg, line_height, cast, gif)
        width, height, raw = last_frame(gif, "rgb24")
        col_w, row_h = width / (cols + 2), FONT_SIZE * float(line_height)
        top = round(row_h / 2)
        col_edge = lambda c: round(col_w + c * col_w)
        row_edge = lambda r: top + round(r * row_h)

        def px(x, y):
            at = (y * width + x) * 3
            return raw[at:at + 3]

        def far_from(band):
            back = max(set(band), key=band.count)
            gap = lambda p: sum(abs(a - b) for a, b in zip(p, back))
            limit = max(24, 0.4 * max(gap(p) for p in band))
            return lambda p: gap(p) > limit

        for kind, x, y in joins:
            if kind == "h":
                edge, ys = col_edge(x + 1), range(row_edge(y), row_edge(y + 1))
                band = [px(edge + dx, yy) for dx in (-3, -2, -1, 0, 1, 2) for yy in ys]
                far = far_from(band)
                a = {yy for yy in ys if far(px(edge - 1, yy))}
                b = {yy for yy in ys if far(px(edge, yy))}
            else:
                edge, xs = row_edge(y + 1), range(col_edge(x), col_edge(x + 1))
                band = [px(xx, edge + dy) for dy in (-3, -2, -1, 0, 1, 2) for xx in xs]
                far = far_from(band)
                a = {xx for xx in xs if far(px(xx, edge - 1))}
                b = {xx for xx in xs if far(px(xx, edge))}
            ok = a and b and any(abs(u - v) <= 1 for u in a for v in b)
            ok = ok and abs(sum(a) / len(a) - sum(b) / len(b)) <= 1.0
            if not ok:
                pair = grid[y][x] + (grid[y][x + 1] if kind == "h" else grid[y + 1][x])
                failures.append(
                    f"{case}, line height {line_height}: {pair} at column {x + 1}, row {y + 1} "
                    "does not meet across the cell edge"
                )
    return failures, len(joins) * len(heights)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("agg", nargs="?", default="agg")
    parser.add_argument("heights", nargs="*")
    parser.add_argument("--demo", action="append", default=[])
    parser.add_argument("--preview")
    parser.add_argument("--width", type=int, default=120)
    parser.add_argument("--height", type=int, default=36)
    parser.add_argument("--frames", type=int, default=120)
    args = parser.parse_args()
    heights = args.heights or ["1.0", "1.4"]
    failures, checked = [], 0
    with tempfile.TemporaryDirectory() as work:
        if args.demo:
            if not args.preview:
                raise SystemExit("check-box-joins: --demo needs --preview (the widget_preview binary)")
            for case in args.demo:
                found, count = check_demo(
                    args.agg, heights, args.preview, case, (args.width, args.height), args.frames, work
                )
                failures += found
                checked += count
        else:
            failures, checked = check_chains(args.agg, heights, work)
    for failure in failures[:40]:
        print(f"check-box-joins: {failure}", file=sys.stderr)
    if failures:
        print(f"check-box-joins: {len(failures)} of {checked} joins fail with {args.agg}", file=sys.stderr)
        return 1
    print(f"check-box-joins: ok ({checked} joins)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
