#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Record a Graphite example in Cell and Graphite mode and check the decoded frames.

Each example draws the same widgets in both modes. Its Graphite frames are the images lunatui
painted, composited over the cell backgrounds; the example refuses to write a frame if any
cell text is left outside an image. Its Cell frames are a cast, rendered here with the pinned
agg. Writes the Graphite GIF and a still that shows one frame in each mode.

Cases: chips (graphite_chips in lunatui-badge) and monitor (activity_monitor).
"""
import argparse
import hashlib
import os
from pathlib import Path
import subprocess

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
FRAME_MS = 900
CASES = {
    "chips": {
        "package": "lunatui-badge", "example": "graphite_chips", "frames": 8,
        "size": (1600, 160), "still": 3, "side_by_side": False, "media": "graphite-chips",
    },
    "monitor": {
        "package": "lunatui", "example": "activity_monitor", "frames": 10,
        "size": (1536, 832), "still": 5, "side_by_side": True, "media": "graphite-activity-monitor",
    },
}


def decoded(path):
    with Image.open(path) as image:
        frames = []
        for index in range(image.n_frames):
            image.seek(index)
            frames.append(image.convert("RGB"))
        return frames


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=sorted(CASES))
    parser.add_argument("--proof-dir", type=Path)
    args = parser.parse_args()
    case = CASES[args.case]
    frames, size, media = case["frames"], case["size"], case["media"]
    output = ROOT / "docs/media" / f"{media}.gif"
    still_path = ROOT / "docs/media" / f"{media}.png"
    proof_dir = args.proof_dir or ROOT / "target" / f"{media}-recording"
    agg = os.environ.get("AGG", "agg")
    version = subprocess.check_output([agg, "--version"], text=True).strip()
    if version != "agg 1.9.0+lunatui.2":
        raise SystemExit("Set AGG to the pinned agg 1.9.0+lunatui.2 used by scripts/record-showcase.sh")
    proof = proof_dir.resolve()
    proof.mkdir(parents=True, exist_ok=True)
    subprocess.run(["cargo", "run", "--locked", "--release", "-p", case["package"],
                    "--example", case["example"], "--", str(proof)], cwd=ROOT, check=True)

    graphite = []
    for index in range(frames):
        with Image.open(proof / f"graphite-{index:02}.png") as frame:
            assert frame.size == size, frame.size
            graphite.append(frame.convert("RGB"))
    assert len({frame.tobytes() for frame in graphite}) == frames, "every Graphite frame differs"

    cells_gif = proof / "cells.gif"
    subprocess.run([agg, "--font-size", "26", "--line-height", "1.23", "--fps-cap", "1",
                    "--last-frame-duration", "1", str(proof / "cells.cast"), str(cells_gif)],
                   check=True)
    cells = decoded(cells_gif)
    assert len(cells) == frames, len(cells)

    output.parent.mkdir(parents=True, exist_ok=True)
    graphite[0].save(output, save_all=True, append_images=graphite[1:],
                     duration=FRAME_MS, loop=0, optimize=False, disposal=1)
    shown = decoded(output)
    assert len(shown) == frames, len(shown)
    for index, (frame, source) in enumerate(zip(shown, graphite)):
        assert frame.size == size
        # A GIF holds 256 colors per frame, so compare the decoded frame to its source loosely.
        differing = sum(1 for a, b in zip(frame.getdata(), source.getdata())
                        if max(abs(x - y) for x, y in zip(a, b)) > 24)
        assert differing < size[0] * size[1] // 100, (index, differing)
    assert output.stat().st_size < 2_000_000

    cell, graphite_still = cells[case["still"]], graphite[case["still"]]
    gap = 24
    if case["side_by_side"]:
        canvas = (cell.width + gap + size[0], max(cell.height, size[1]))
        at = (cell.width + gap, 0)
    else:
        canvas = (max(cell.width, size[0]), cell.height + gap + size[1])
        at = (0, cell.height + gap)
    still = Image.new("RGB", canvas, (18, 18, 24))
    still.paste(cell, (0, 0))
    still.paste(graphite_still, at)
    still.save(still_path, optimize=True)

    for path in [output, still_path]:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"{path.relative_to(ROOT)} {path.stat().st_size} bytes sha256 {digest}")


if __name__ == "__main__":
    main()
