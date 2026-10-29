#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Re-encodes an agg GIF as a lossless animated WebP with millisecond frame timing.

GIF delays count in hundredths of a second and agg never writes one under 20 ms, so a
recording above 30 fps is rendered slowed down to a TIMELINE_FPS of at most 30. This
takes each GIF frame's presentation time, puts it back on its slot of that timeline,
and gives every frame the milliseconds of the real FPS up to the next slot (17, 17,
16 at 60 fps). A frame agg merged with identical neighbours keeps their slots. The
last frame is held for the given number of seconds.

Usage: gif-to-webp.py GIF WEBP FPS HOLD_SECONDS TIMELINE_FPS
"""
import os
import subprocess
import sys
import tempfile


def durations(times: list, fps: int, hold_ms: int, timeline: int) -> list:
    """Milliseconds each frame is shown, given its presentation time in seconds on a
    timeline of `timeline` frames per second."""
    slots = [round(time * timeline) for time in times]
    edges = [round(slot * 1000 / fps) for slot in slots]
    shown = [max(1, after - before) for before, after in zip(edges, edges[1:])]
    return shown + [hold_ms] if times else []


def frame_times(gif: str) -> list:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "frame=best_effort_timestamp_time", "-of", "csv=p=0", gif],
        check=True, capture_output=True, text=True,
    ).stdout
    return [float(line.strip().rstrip(",")) for line in out.splitlines() if line.strip()]


def main(gif: str, webp: str, fps: int, hold: float, timeline: int) -> None:
    times = frame_times(gif)
    with tempfile.TemporaryDirectory() as work:
        subprocess.run(
            ["ffmpeg", "-loglevel", "error", "-i", gif, "-fps_mode", "passthrough",
             os.path.join(work, "%05d.png")],
            check=True,
        )
        frames = sorted(name for name in os.listdir(work) if name.endswith(".png"))
        if len(frames) != len(times):
            sys.exit(f"gif-to-webp: {len(frames)} frames decoded but {len(times)} timestamps")
        args = ["img2webp", "-loop", "0", "-lossless", "-m", "4"]
        for name, shown in zip(frames, durations(times, fps, round(hold * 1000), timeline)):
            args += ["-d", str(shown), os.path.join(work, name)]
        subprocess.run(args + ["-o", webp], check=True, capture_output=True)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4]), int(sys.argv[5]))
