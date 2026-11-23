#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Record the graphite_chips example in Cell and Graphite mode and check the decoded frames.

The example draws the same badges and key hints in both modes. Its Graphite frames are the
images lunatui painted, composited over the cell backgrounds; the example refuses to write a
frame if any cell text is left outside an image. Its Cell frames are a cast, rendered here
with the pinned agg. Writes the Graphite GIF and a still that shows one frame in each mode.
"""
import argparse
import hashlib
import os
from pathlib import Path
import subprocess

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
FRAMES = 8
FRAME_MS = 900
SIZE = (1600, 160)
STILL_FRAME = 3


def decoded(path):
    with Image.open(path) as image:
        frames = []
        for index in range(image.n_frames):
            image.seek(index)
            frames.append(image.convert("RGB"))
        return frames


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/media/graphite-chips.gif")
    parser.add_argument("--still", type=Path, default=ROOT / "docs/media/graphite-chips.png")
    parser.add_argument("--proof-dir", type=Path, default=ROOT / "target/graphite-chips-recording")
    args = parser.parse_args()
    agg = os.environ.get("AGG", "agg")
    version = subprocess.check_output([agg, "--version"], text=True).strip()
    if version != "agg 1.9.0+lunatui.2":
        raise SystemExit("Set AGG to the pinned agg 1.9.0+lunatui.2 used by scripts/record-showcase.sh")
    proof = args.proof_dir.resolve()
    proof.mkdir(parents=True, exist_ok=True)
    subprocess.run(["cargo", "run", "--locked", "--release", "-p", "lunatui-badge",
                    "--example", "graphite_chips", "--", str(proof)], cwd=ROOT, check=True)

    graphite = []
    for index in range(FRAMES):
        with Image.open(proof / f"graphite-{index:02}.png") as frame:
            assert frame.size == SIZE, frame.size
            graphite.append(frame.convert("RGB"))
    assert len({frame.tobytes() for frame in graphite}) == FRAMES, "every Graphite frame differs"

    cells_gif = proof / "cells.gif"
    subprocess.run([agg, "--font-size", "26", "--line-height", "1.23", "--fps-cap", "1",
                    "--last-frame-duration", "1", str(proof / "cells.cast"), str(cells_gif)],
                   check=True)
    cells = decoded(cells_gif)
    assert len(cells) == FRAMES, len(cells)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    graphite[0].save(args.output, save_all=True, append_images=graphite[1:],
                     duration=FRAME_MS, loop=0, optimize=False, disposal=1)
    shown = decoded(args.output)
    assert len(shown) == FRAMES, len(shown)
    for index, (frame, source) in enumerate(zip(shown, graphite)):
        assert frame.size == SIZE
        # A GIF holds 256 colors per frame, so compare the decoded frame to its source loosely.
        differing = sum(1 for a, b in zip(frame.getdata(), source.getdata())
                        if max(abs(x - y) for x, y in zip(a, b)) > 24)
        assert differing < SIZE[0] * SIZE[1] // 100, (index, differing)
    assert args.output.stat().st_size < 2_000_000

    cell = cells[STILL_FRAME]
    gap = 24
    width = max(cell.width, SIZE[0])
    still = Image.new("RGB", (width, cell.height + SIZE[1] + gap), (18, 18, 24))
    still.paste(cell, (0, 0))
    still.paste(graphite[STILL_FRAME], (0, cell.height + gap))
    args.still.parent.mkdir(parents=True, exist_ok=True)
    still.save(args.still, optimize=True)

    for path in [args.output, args.still]:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"{path.relative_to(ROOT)} {path.stat().st_size} bytes sha256 {digest}")


if __name__ == "__main__":
    main()
