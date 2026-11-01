#!/usr/bin/env python3 -I
"""Render the site's terminal scenes from the release console example.

usage: python3 -I scripts/site/scenes.py <out.json>

Runs `cargo run --example release_console_app -- --snapshot-styled` for the console, the narrow
console and each widget scene, and turns each styled frame into exact-width HTML cells.
"""
import html
import json
import subprocess
import sys

LABELS = {
    "console": ("A release console built from Lunatui widgets: tabs, a services table, capacity gauges, release progress, a requests sparkline and a log.", []),
    "console-narrow": ("The same release console reflowed to 46 columns for a narrow terminal.", ["--width", "46"]),
    "form": ("Form: Name and Email marked required, a hint under Name, an error under Email, a radio group and a toggle.", ["--scene", "form"]),
    "progress": ("ProgressBar: four pipeline stages, done, running, failed and queued.", ["--scene", "progress"]),
    "logview": ("LogView: timestamped entries with level colors, newest at the bottom.", ["--scene", "logview"]),
    "calendar": ("Monthly calendar for November 2026, Sunday first, with three event days marked.", ["--scene", "calendar"]),
    "sparkline": ("Two sparklines: a three-row requests line and a one-row errors line.", ["--scene", "sparkline"]),
    "barchart": ("BarChart: seven vertical bars of builds per day with labels.", ["--scene", "barchart"]),
    "accordion": ("Accordion: Build and Publish open with their steps, Package and Notify closed.", ["--scene", "accordion"]),
    "bigtext": ("BigText: the time 14:02 drawn in large pixels.", ["--scene", "bigtext"]),
}

COLORS = ("a", "o", "w", "f", "fc", "d", "ln")
EIGHTHS = {"█": "bg", "▀": "ht", "▄": "hb"}
EIGHTHS.update({ch: f"eb e{i}" for i, ch in enumerate("▁▂▃▄▅▆▇", 1)})
EIGHTHS.update({ch: f"el e{i}" for i, ch in enumerate("▏▎▍▌▋▊▉", 1)})


def frame(args):
    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "lunatui", "--example", "release_console_app", "--",
         "--snapshot-styled", *args],
        check=True, capture_output=True, text=True,
    ).stdout
    rows, row = [], []
    for line in out.split("\n"):
        if line == "--":
            rows.append(row)
            row = []
        elif line:
            cls, _, text = line.partition("\t")
            row.append((text, " ".join(dict.fromkeys(cls.split()))))
    return rows


def cells(rows):
    lines = []
    for row in rows:
        split = []
        for text, cls in row:
            color = next((k for k in cls.split() if k in COLORS), None)
            if color is None or not any(ch in EIGHTHS for ch in text):
                split.append((text, cls))
                continue
            rest = " ".join(k for k in cls.split() if k != color)
            for ch in text:
                if ch in EIGHTHS:
                    split.append((" ", f"{color} {EIGHTHS[ch]}" + (f" {rest}" if rest else "")))
                else:
                    split.append((ch, cls))
        merged = []
        for text, cls in split:
            if merged and merged[-1][1] == cls:
                merged[-1] = (merged[-1][0] + text, cls)
            else:
                merged.append((text, cls))
        lines.append("".join(
            f'<span class="{c}">{html.escape(t)}</span>' if c else html.escape(t) for t, c in merged
        ))
    return "\n".join(lines)


def main():
    out = {}
    for name, (label, args) in LABELS.items():
        rows = frame(args)
        cols = sum(len(t) for t, _ in rows[0])
        out[name] = {"cols": cols, "rows": len(rows), "label": label, "html": cells(rows)}
    with open(sys.argv[1], "w") as handle:
        json.dump(out, handle, indent=1, ensure_ascii=False)


if __name__ == "__main__":
    main()
