#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Compose the Graphite mode design mockups.

These are hand-composed pictures of what Graphite mode could look like. They are
not output of lunatui. The data matches the real Cell mode capture next to them.
Run: python3 scripts/graphite-mode-mockups.py OUTPUT_DIRECTORY
Needs Pillow and the macOS system fonts. The Cell mode restart dialog is drawn from
restart-dialog-cell-grid.json in the output directory when it exists: the cells the
lunatui Block and Accordion widgets produced for the restart dialog prototype.
"""
import json
import math
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SCALE = 3
W, H = 1480, 818

BG = "#111214"
PANEL = "#15171b"
TEXT = "#e6e9ee"
MUTED = "#a3abb8"
HAIR = "#262a32"
SELECT = "#1c2330"
FOCUS = "#5aa2ff"
WORKING = "#4cc98a"
WAITING = "#f2b84b"
FAILED = "#ff7a6b"
STOPPED = "#7d8594"

SANS = "/System/Library/Fonts/SFNS.ttf"
MONO = "/System/Library/Fonts/SFNSMono.ttf"


def font(path, size, weight=None):
    f = ImageFont.truetype(path, int(size * SCALE))
    if weight:
        try:
            f.set_variation_by_name(weight)
        except OSError:
            pass
    return f


class Canvas:
    def __init__(self, w, h, background):
        self.w, self.h = w, h
        self.image = Image.new("RGB", (w * SCALE, h * SCALE), background)
        self.draw = ImageDraw.Draw(self.image, "RGBA")

    def s(self, v):
        return v * SCALE

    def rect(self, x, y, w, h, fill, radius=0, outline=None, width=1):
        box = [self.s(x), self.s(y), self.s(x + w) - 1, self.s(y + h) - 1]
        self.draw.rounded_rectangle(
            box, radius=self.s(radius), fill=fill, outline=outline, width=self.s(width) if outline else 0
        )

    def line(self, x1, y1, x2, y2, fill, width=1):
        self.draw.line([self.s(x1), self.s(y1), self.s(x2), self.s(y2)], fill=fill, width=int(self.s(width)))

    def text(self, x, y, value, f, fill, anchor="ls"):
        self.draw.text((self.s(x), self.s(y)), value, font=f, fill=fill, anchor=anchor)

    def width_of(self, value, f):
        return self.draw.textlength(value, font=f) / SCALE

    def circle(self, cx, cy, r, fill):
        self.draw.ellipse([self.s(cx - r), self.s(cy - r), self.s(cx + r), self.s(cy + r)], fill=fill)

    def polygon(self, points, fill):
        self.draw.polygon([(self.s(x), self.s(y)) for x, y in points], fill=fill)

    def save(self, path):
        self.image.resize((self.w, self.h), Image.LANCZOS).save(path)


def tint(color, alpha):
    r, g, b = (int(color[i : i + 2], 16) for i in (1, 3, 5))
    return (r, g, b, alpha)


def glyph(c, kind, x, y, size=7):
    color = {"failed": FAILED, "waiting": WAITING, "working": WORKING, "idle": STOPPED, "stuck": FAILED, "down": STOPPED}[kind]
    if kind == "working":
        c.circle(x, y, size * 0.55, color)
    elif kind == "waiting":
        c.polygon([(x, y - size * 0.7), (x + size * 0.7, y), (x, y + size * 0.7), (x - size * 0.7, y)], color)
    elif kind == "failed":
        d = size * 0.5
        c.line(x - d, y - d, x + d, y + d, color, 1.8)
        c.line(x - d, y + d, x + d, y - d, color, 1.8)
    elif kind == "idle":
        c.draw.ellipse(
            [c.s(x - size * 0.5), c.s(y - size * 0.5), c.s(x + size * 0.5), c.s(y + size * 0.5)],
            outline=color,
            width=int(c.s(1.5)),
        )
    elif kind == "stuck":
        c.rect(x - 1, y - size * 0.7, 2, size * 0.9, color, 1)
        c.circle(x, y + size * 0.65, 1.1, color)
    elif kind == "down":
        c.rect(x - size * 0.5, y - size * 0.5, size, size, color, 1.5)


def chip(c, x, y, kind, label, f):
    color = {"working": WORKING, "waiting": WAITING, "failed": FAILED, "idle": STOPPED, "stuck": FAILED, "down": STOPPED}[kind]
    w = c.width_of(label, f) + 34
    c.rect(x, y, w, 26, tint(color, 30 if kind in ("idle", "down", "stuck") else 44), 13)
    glyph(c, kind, x + 14, y + 13, 7)
    c.text(x + 25, y + 18, label, f, TEXT if kind in ("idle", "down", "stuck") else color)
    return w


def lane(c, x, y, w, h, bins, base=None):
    gap = 1.5
    bw = (w - gap * (len(bins) - 1)) / len(bins)
    c.rect(x, y + h - 1, w, 1, HAIR)
    for i, (state, level) in enumerate(bins):
        color = {"working": WORKING, "waiting": WAITING, "failed": FAILED, "idle": STOPPED}[state]
        bh = max(2.0, level * (h - 2))
        c.rect(x + i * (bw + gap), y + h - 1 - bh, bw, bh, color, min(bw / 2, 2.2))


def make_bins(seed, count, mix):
    rng = random.Random(seed)
    bins, level, state = [], 0.5, "working"
    for _ in range(count):
        if rng.random() < 0.12:
            state = rng.choices(["working", "waiting", "failed"], mix)[0]
        level = min(1.0, max(0.1, level + rng.uniform(-0.28, 0.28)))
        bins.append((state, level))
    return bins


EVENTS = [
    ("54s", "failed", "test_failed", FAILED),
    ("58s", "failed", "test_failed", FAILED),
    ("1m", "start", "test_started", TEXT),
    ("1m", "dot", "edit_finished", TEXT),
    ("1m", "start", "command_started", TEXT),
    ("1m", "waiting", "waiting", TEXT),
    ("1m", "failed", "test_failed", FAILED),
    ("1m", "start", "test_started", TEXT),
    ("1m", "dot", "edit_finished", TEXT),
    ("1m", "start", "command_started", TEXT),
    ("1m", "waiting", "waiting", TEXT),
    ("1m", "failed", "test_failed", FAILED),
    ("1m", "start", "test_started", TEXT),
    ("1m", "dot", "edit_finished", TEXT),
    ("2m", "start", "command_started", TEXT),
]


def keycap(c, x, y, key, label, f, fk):
    kw = max(22, c.width_of(key, fk) + 14)
    c.rect(x, y - 15, kw, 22, "#1d2027", 6, outline="#2c313b", width=1)
    c.text(x + kw / 2, y + 1, key, fk, TEXT, anchor="ms")
    lw = c.width_of(label, f)
    c.text(x + kw + 8, y + 1, label, f, MUTED)
    return kw + 8 + lw + 22


def activity_monitor(path):
    c = Canvas(W, H, BG)
    body = font(SANS, 15, "Regular")
    body_s = font(SANS, 14, "Regular")
    strong = font(SANS, 17, "Semibold")
    title = font(SANS, 19, "Bold")
    small = font(SANS, 12.5, "Regular")
    mono = font(MONO, 13, "Regular")

    c.text(28, 36, "Activity monitor", title, TEXT)
    x = 262
    for kind, label in (
        ("working", "1 working"),
        ("waiting", "1 waiting"),
        ("idle", "0 idle"),
        ("stuck", "0 stuck"),
        ("failed", "1 failed"),
        ("down", "0 down"),
    ):
        x += chip(c, x, 17, kind, label, body_s) + 8

    c.text(28, 76, "sample", strong, TEXT)
    sx = 28 + c.width_of("sample", strong) + 12
    c.text(sx, 76, "connected  ·  3 seats  ·", body, MUTED)
    sx += c.width_of("connected  ·  3 seats  ·", body) + 8
    c.text(sx, 76, "2 need you", body, WAITING)

    c.rect(18, 98, 742, 1, HAIR)
    rows = [
        ("failed", "builder", "test_failed", "54s", True),
        ("waiting", "reviewer", "waiting", "53s", False),
        ("working", "worker", "command_started", "52s", False),
    ]
    for i, (kind, name, state, age, selected) in enumerate(rows):
        y = 108 + i * 54
        if selected:
            c.rect(12, y, 754, 46, SELECT, 10)
            c.rect(12, y + 7, 3, 32, FOCUS, 1.5)
        glyph(c, kind, 40, y + 23, 8)
        c.text(64, y + 29, name, strong, TEXT)
        c.text(196, y + 29, state, body, MUTED)
        c.text(496, y + 29, age, body, TEXT, anchor="rs")
        c.rect(508, y + 13, 50, 21, tint(STOPPED, 40), 10.5)
        c.text(533, y + 28, "stale", small, MUTED, anchor="ms")
        lane(c, 580, y + 10, 166, 28, make_bins(11 + i, 38, [6, 2, 2 if kind != "failed" else 4]))

    c.line(776, 62, 776, H - 56, HAIR, 1)

    x0 = 804
    c.text(x0, 76, "builder", font(SANS, 21, "Bold"), TEXT)
    bx = x0 + c.width_of("builder", font(SANS, 21, "Bold")) + 10
    c.text(bx, 76, "on sample", body, MUTED)
    cx = bx + c.width_of("on sample", body) + 22
    cx += chip(c, cx, 56, "failed", "failed for 58s", body_s) + 8
    c.rect(cx, 56, 56, 26, tint(STOPPED, 40), 13)
    c.text(cx + 28, 74, "stale", small, MUTED, anchor="ms")
    c.text(x0, 112, "80 events  ·  1 task  ·  17 failures  ·  last 54s ago", body, MUTED)

    big = make_bins(5, 96, [5, 2, 3])
    lane(c, x0, 138, 640, 60, big)
    for label, frac in (("-10m", 0.0), ("-5m", 0.5), ("now", 1.0)):
        px = x0 + frac * 640
        anchor = "ls" if frac == 0 else ("rs" if frac == 1 else "ms")
        c.line(px, 200, px, 206, HAIR, 1)
        c.text(px, 224, label, small, MUTED, anchor=anchor)

    ey = 262
    for i, (age, kind, name, color) in enumerate(EVENTS):
        y = ey + i * 32
        if i % 2 == 0:
            c.rect(x0 - 10, y - 4, 664, 30, (255, 255, 255, 6), 8)
        c.text(x0 + 62, y + 17, age, body, MUTED, anchor="rs")
        c.text(x0 + 76, y + 17, "+3.6s", font(SANS, 13, "Regular"), (163, 171, 184, 150))
        icon_x, icon_y = x0 + 156, y + 12
        if kind == "failed":
            glyph(c, "failed", icon_x, icon_y, 6)
        elif kind == "waiting":
            glyph(c, "waiting", icon_x, icon_y, 6)
        elif kind == "start":
            c.polygon([(icon_x - 3, icon_y - 4.5), (icon_x + 4.5, icon_y), (icon_x - 3, icon_y + 4.5)], WORKING)
        else:
            c.circle(icon_x, icon_y, 1.6, MUTED)
        c.text(x0 + 178, y + 17, name, body, color)
        c.text(x0 + 376, y + 17, "hooks", body_s, MUTED)
    c.rect(1452, 262, 3, 300, "#2c313b", 1.5)
    c.rect(1452, 262, 3, 120, "#4a5160", 1.5)

    c.rect(0, H - 52, W, 1, HAIR)
    kx = 24
    kf = font(SANS, 14, "Regular")
    kk = font(SANS, 12.5, "Medium")
    for key, label in (("↑↓", "move"), ("⏎", "timeline"), ("d", "details"), ("tab", "host"), ("f", "filter"), ("s", "sort"), ("i", "IDs"), ("?", "keys"), ("q", "quit")):
        kx += keycap(c, kx, H - 20, key, label, kf, kk)
    c.save(path)


def restart_dialog(path):
    c = Canvas(760, 560, "#0e0f12")
    for i in range(14):
        c.rect(20, 20 + i * 36, 720, 18, (255, 255, 255, 5), 4)
    c.rect(0, 0, 760, 560, (0, 0, 0, 150))
    px, py, pw, ph = 70, 62, 620, 438
    for i in range(10, 0, -1):
        c.rect(px - i, py - i + 6, pw + 2 * i, ph + 2 * i, (0, 0, 0, 10), 14 + i)
    c.rect(px, py, pw, ph, "#1b1e24", 14, outline="#2c313b", width=1)
    title = font(SANS, 19, "Bold")
    body = font(SANS, 14.5, "Regular")
    label = font(SANS, 16, "Medium")
    hint = font(SANS, 13.5, "Regular")
    mono = font(MONO, 13, "Regular")
    c.text(px + 28, py + 46, "Restart to update", title, TEXT)
    c.text(px + 28 + c.width_of("Restart to update", title) + 12, py + 46, "v0.3.70", mono, MUTED)
    lines = ["Save the session configuration to recreate your sessions after restart.", "Unsaved work inside programs is not saved."]
    for i, line in enumerate(lines):
        c.text(px + 28, py + 78 + i * 22, line, body, MUTED)
    c.rect(px + 1, py + 130, pw - 2, 1, HAIR)
    options = [
        ("Save and restart", "Recreate sessions afterwards", True),
        ("Restart without saving", "Sessions are discarded", False),
        ("Cancel", "Nothing changes now", False),
    ]
    for i, (name, note, selected) in enumerate(options):
        y = py + 142 + i * 54
        if selected:
            c.rect(px + 10, y, pw - 20, 46, "#252a33", 9)
            c.rect(px + 10, y + 8, 3.5, 30, FOCUS, 2)
        c.text(px + 32, y + 29, name, label, "#ffffff" if selected else TEXT)
        c.text(px + pw - 28, y + 29, note, hint, MUTED, anchor="rs")
    c.rect(px + 1, py + 312, pw - 2, 1, HAIR)
    c.text(px + 28, py + 344, "44 sessions · names, directories, launch commands, pane layouts", hint, MUTED)
    c.text(px + pw - 28, py + 344, "Show", hint, FOCUS, anchor="rs")
    kx = px + 26
    kf = font(SANS, 13, "Regular")
    kk = font(SANS, 12, "Medium")
    for key, label_ in (("⏎", "select"), ("esc", "close"), ("↑↓", "move")):
        kx += keycap(c, kx, py + ph - 24, key, label_, kf, kk)
    c.save(path)


def cell_dialog(grid_path, path):
    rows = json.loads(grid_path.read_text())
    cw, ch = 11, 22
    margin = 40
    w = len(rows[0]) * cw + 2 * margin
    h = len(rows) * ch + 2 * margin
    c = Canvas(w, h, "#0e0f12")
    c.rect(0, 0, w, h, (0, 0, 0, 150))
    mono = font(MONO, 15, "Regular")
    bold = font(MONO, 15, "Bold")
    for y, row in enumerate(rows):
        for x, (glyph_, fg, bg, is_bold) in enumerate(row):
            px, py = margin + x * cw, margin + y * ch
            if bg != "-":
                c.rect(px, py, cw, ch, bg)
            if glyph_ and glyph_ != " ":
                c.text(px + cw / 2, py + 16, glyph_, bold if is_bold else mono, fg if fg != "-" else TEXT, anchor="ms")
    c.save(path)


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    activity_monitor(out / "activity-monitor-graphite-mockup.png")
    restart_dialog(out / "restart-dialog-graphite-mockup.png")
    grid = out / "restart-dialog-cell-grid.json"
    if grid.exists():
        cell_dialog(grid, out / "restart-dialog-cell-mode.png")


if __name__ == "__main__":
    main()
