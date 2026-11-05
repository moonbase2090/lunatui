#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify the effects recordings, then export gallery media and source hashes."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
args = parser.parse_args()
subprocess.run([sys.executable, str(ROOT / "scripts/check-effects-recordings.py"),
                str(args.directory)], check=True)
out = ROOT / "docs/media"
artifacts = []
for size in ["80x24", "200x60"]:
    for theme in ["dark", "light"]:
        for profile in ["truecolor", "ansi256", "ansi16"]:
            name = f"effects-{size}-{profile}-{theme}"
            video = f"{name}-side-by-side.mp4"
            shutil.copyfile(args.directory / video, out / video)
            artifacts.append(video)
            for side in ["lunatui", "ratatui"]:
                png = f"{name}-{side}.png"
                subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-ss", "4",
                                "-i", str(args.directory / f"{name}-{side}.gif"),
                                "-frames:v", "1", str(out / png)], check=True)
                artifacts.append(png)
gallery = "effects-80x24-truecolor-dark.gif"
shutil.copyfile(args.directory / "effects-80x24-truecolor-dark-lunatui.gif", out / gallery)
artifacts.append(gallery)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


sources = ["benches/src/widget_suite/effects.rs", "lunatui-runtime/src/effect.rs",
           "examples/widget_preview.rs"]
manifest = {
    "source_sha256": {source: digest(ROOT / source) for source in sources},
    "baseline": "ratatui 0.30.2 with a hand-written fade loop; tachyonfx not used",
    "renderer": "agg 1.9.0+lunatui.2",
    "scripted_frames": 240, "fps": 30, "screenshot_second": 4,
    "pixel_check": "12 size/theme/color cases passed from decoded recordings",
    "artifacts": [{"file": file, "bytes": (out / file).stat().st_size,
                   "sha256": digest(out / file)} for file in artifacts],
}
(out / "effects-proof.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Exported {len(artifacts)} verified recording artifacts")
