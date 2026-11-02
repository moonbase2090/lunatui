#!/usr/bin/env python3 -I
"""Rewrite the quickstart page's template frames from the real template programs.

usage: python3 -I scripts/site/quickstart_frames.py site/quickstart.html [--check]

Builds the six template examples, runs each one in a pseudo terminal of its own size, reads the
first frame it draws, maps the default dark theme colors to the page's color classes and
rewrites that template's `<pre class="term">`. With --check the page is left alone and the
script fails when it would change.
"""
import fcntl
import html
import importlib.util
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
spec = importlib.util.spec_from_file_location("scenes", os.path.join(HERE, "scenes.py"))
scenes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scenes)

TEMPLATES = {
    "minimal": (48, 10),
    "list": (42, 14),
    "form": (56, 12),
    "dashboard": (72, 22),
    "popover": (56, 16),
    "layout": (72, 22),
}

# default dark theme -> page classes ("" is the text color, "bg" the page background)
FG = {
    (0xcd, 0xd6, 0xf4): "",
    (0x9e, 0xce, 0x6a): "a",
    (0xa6, 0xe3, 0xa1): "o",
    (0xfa, 0xbd, 0x2f): "w",
    (0xf3, 0x8b, 0xa8): "f",
    (0xb4, 0xbc, 0xfa): "fc",
    (0x1e, 0x1e, 0x2e): "bg",
}
BG = {(0x1e, 0x1e, 0x2e): "", (0x31, 0x32, 0x44): "sel"}


def capture(name, width, height):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
    proc = subprocess.Popen(
        [os.path.join(ROOT, "target", "debug", "examples", f"template_{name}")],
        stdin=slave, stdout=slave, stderr=slave, close_fds=True,
    )
    os.close(slave)
    data, end = b"", time.time() + 10
    while time.time() < end:
        ready, _, _ = select.select([master], [], [], 0.1)
        if ready:
            try:
                data += os.read(master, 65536)
            except OSError:
                break
        if data.count(b"\x1b[?2026l") >= 1:
            break
    proc.kill()
    proc.wait()
    os.close(master)
    return data.decode("utf-8", "replace").split("\x1b[?2026l")[0]


def parse(text, width, height):
    cells = [[(" ", None, None, False)] * width for _ in range(height)]
    fg = bg = None
    bold = False
    row = col = 0
    i = 0
    while i < len(text):
        ch = text[i]
        if ch == "\x1b" and text.startswith("[", i + 1):
            m = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z])").match(text, i)
            if not m:
                i += 1
                continue
            args, final = m.group(1), m.group(2)
            if final == "H":
                parts = [int(p) if p else 1 for p in args.split(";")] if args else [1, 1]
                row, col = parts[0] - 1, (parts[1] - 1 if len(parts) > 1 else 0)
            elif final == "m":
                nums = [int(p) if p else 0 for p in args.split(";")] if args else [0]
                k = 0
                while k < len(nums):
                    n = nums[k]
                    if n == 0:
                        fg = bg = None
                        bold = False
                    elif n == 1:
                        bold = True
                    elif n == 22:
                        bold = False
                    elif n == 39:
                        fg = None
                    elif n == 49:
                        bg = None
                    elif n in (38, 48) and nums[k + 1 : k + 2] == [2]:
                        rgb = tuple(nums[k + 2 : k + 5])
                        if n == 38:
                            fg = rgb
                        else:
                            bg = rgb
                        k += 4
                    k += 1
            i = m.end()
            continue
        i += 1
        if ch in "\r\n":
            continue
        wide = unicodedata.east_asian_width(ch) in ("W", "F")
        if 0 <= row < height and 0 <= col < width:
            cells[row][col] = (ch, fg, bg, bold)
            if wide and col + 1 < width:
                cells[row][col + 1] = ("", fg, bg, bold)
        col += 2 if wide else 1
    return cells


def classes(fg, bg, bold):
    unknown = []
    f = FG.get(fg, "") if fg else ""
    if fg and fg not in FG:
        unknown.append(("fg", fg))
    b = BG.get(bg, "") if bg else ""
    if bg and bg not in BG:
        unknown.append(("bg", bg))
    return " ".join(p for p in (f if f != "bg" else "", b, "b" if bold else "") if p), unknown


def rows_of(cells):
    rows, unknown = [], set()
    for line in cells:
        runs = []
        for ch, fg, bg, bold in line:
            if ch == "":
                continue
            cls, bad = classes(fg, bg, bold)
            unknown.update(bad)
            if runs and runs[-1][1] == cls:
                runs[-1] = (runs[-1][0] + ch, cls)
            else:
                runs.append((ch, cls))
        rows.append(runs)
    return rows, unknown


def main():
    page_path = sys.argv[1]
    check = "--check" in sys.argv
    subprocess.run(["cargo", "build", "-q", "-p", "lunatui", "--examples"], check=True, cwd=ROOT)
    with open(page_path, encoding="utf-8") as handle:
        page = handle.read()
    for name, (width, height) in TEMPLATES.items():
        cells = parse(capture(name, width, height), width, height)
        rows, unknown = rows_of(cells)
        if unknown:
            print(f"{name}: unmapped colors {sorted(unknown)}", file=sys.stderr)
        body = scenes.cells(rows)
        pattern = re.compile(
            r'(<article class="tpl" id="' + name + r'">.*?<pre class="term[^"]*" style="--cols:'
            + str(width) + ";--rows:" + str(height) + r'" role="img" aria-label="[^"]*">)(.*?)(</pre>)',
            re.S,
        )
        if not pattern.search(page):
            sys.exit(f"no frame for {name} in {page_path}")
        page = pattern.sub(lambda m: m.group(1) + body + m.group(3), page, count=1)
    with open(page_path, encoding="utf-8") as handle:
        current = handle.read()
    if check:
        if page != current:
            sys.exit(f"{page_path} does not match the template output")
    else:
        with open(page_path, "w", encoding="utf-8") as handle:
            handle.write(page)


if __name__ == "__main__":
    main()
