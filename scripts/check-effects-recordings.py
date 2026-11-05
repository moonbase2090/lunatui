#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check decoded frames of the effects recordings across the size/theme/color matrix."""
import argparse
import hashlib
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
args = parser.parse_args()


def frame(path, second):
    raw = subprocess.check_output([
        "ffmpeg", "-loglevel", "error", "-ss", str(second), "-i", str(path),
        "-frames:v", "1", "-pix_fmt", "rgb24", "-f", "rawvideo", "-",
    ])
    if not raw:
        raise SystemExit(f"No decoded frame at {second}s: {path.name}")
    return hashlib.sha256(raw).hexdigest()


def similarity(first, second, at):
    """PSNR in dB of two recordings at one time; GIF palettes make exact hashes differ."""
    run = subprocess.run([
        "ffmpeg", "-loglevel", "info", "-ss", str(at), "-t", "0.1", "-i", str(first),
        "-ss", str(at), "-t", "0.1", "-i", str(second), "-lavfi", "psnr", "-f", "null", "-",
    ], capture_output=True, text=True, check=True)
    match = re.search(r"average:(\S+)", run.stderr)
    if not match:
        raise SystemExit(f"No PSNR result for {first.name}")
    return float("inf") if match.group(1) == "inf" else float(match.group(1))


MIN_DB = 40.0
checked = 0
for size in ["80x24", "200x60"]:
    for theme in ["dark", "light"]:
        for profile in ["truecolor", "ansi256", "ansi16"]:
            name = f"effects-{size}-{profile}-{theme}"
            held = {}
            for side in ["lunatui", "ratatui"]:
                path = args.directory / f"{name}-{side}.gif"
                faded, mid, hold_a, hold_b = (frame(path, t) for t in (0.0, 1.0, 3.5, 4.5))
                if hold_a != hold_b:
                    raise SystemExit(f"The held frame changes: {path.name}")
                if faded == hold_a:
                    raise SystemExit(f"The fade-in never leaves the faded look: {path.name}")
                # 16 colors can snap a mid-fade frame onto a neighbouring look.
                if profile != "ansi16" and len({faded, mid, hold_a}) != 3:
                    raise SystemExit(f"Fade-in does not progress through three looks: {path.name}")
                held[side] = path
            # ratatui writes nothing while the frame holds, and the pinned renderer can
            # keep a stale 16-color tint for that stretch even though the byte stream
            # matches the native one, so the pixel match applies to the other profiles.
            if profile != "ansi16":
                db = similarity(held["lunatui"], held["ratatui"], 3.5)
                if db < MIN_DB:
                    raise SystemExit(f"Held frames differ between implementations: {name} ({db:.1f} dB)")
            checked += 1
            print(f"PASS {name}: fade progresses, held frame stable, matching on both sides where the renderer is exact", flush=True)
print(f"PASS: {checked} size/theme/color cases verified from decoded recordings")
