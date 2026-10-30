#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

"""Check every displayed frame for colored pixels inside a widget's rectangle."""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("recording", type=Path)
    parser.add_argument("--region", required=True, help="x,y,width,height in screenshot pixels")
    parser.add_argument("--min-colored-pixels", required=True, type=int)
    parser.add_argument("--contact-sheet", type=Path)
    args = parser.parse_args()
    x, y, width, height = map(int, args.region.split(","))
    recording = Image.open(args.recording)
    if not (0 <= x < x + width <= recording.width and 0 <= y < y + height <= recording.height):
        parser.error("region must be inside the recording")
    if args.min_colored_pixels <= 0:
        parser.error("minimum colored pixel count must be positive")
    counts = []
    thumbnails = []
    elapsed = 0
    for index in range(recording.n_frames):
        recording.seek(index)
        frame = recording.convert("RGB")
        colored = sum(
            max(rgb) >= 100 and max(rgb) - min(rgb) >= 40
            for rgb in frame.crop((x, y, x + width, y + height)).getdata()
        )
        duration = recording.info.get("duration", 0)
        counts.append({"frame": index, "start_ms": elapsed, "duration_ms": duration, "colored_pixels": colored})
        elapsed += duration
        if args.contact_sheet:
            frame.thumbnail((400, 256))
            thumbnails.append(frame)
    if args.contact_sheet:
        sheet = Image.new("RGB", (1600, 280 * ((len(thumbnails) + 3) // 4)), "#333333")
        draw = ImageDraw.Draw(sheet)
        for index, frame in enumerate(thumbnails):
            left, top = index % 4 * 400, index // 4 * 280
            sheet.paste(frame, (left, top + 24))
            draw.text((left + 5, top + 5), f"frame {index}: {counts[index]['duration_ms']}ms", fill="white")
        sheet.save(args.contact_sheet)
    print(json.dumps({"region": [x, y, width, height], "frames": counts}, indent=2))
    failures = [item["frame"] for item in counts if item["colored_pixels"] < args.min_colored_pixels]
    if failures:
        raise SystemExit(f"Widget blank or below threshold in frames: {failures}")


if __name__ == "__main__":
    main()
