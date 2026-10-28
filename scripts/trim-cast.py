#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Drops the leading blank and the trailing alternate-screen exit from an asciicast.

The wait before the first paint is a blank cursor frame. The widget preview also
leaves the alternate screen after its last frame. The first event delay is cleared
and events from the first alternate-screen exit onward are removed.

With a frame rate as the second argument, each frame is also put back on its slot of
that fixed timestep. The preview draws frame k for time k / fps, but the recorder
stamps output with a few milliseconds of jitter, and a sampler at the same rate would
then drop frames. A frame is the run of events written without a pause of
FRAME_GAP seconds or more; its first event moves to k / fps and the rest follow it.
"""
import json
import sys

EXIT = "\x1b[?1049l"
FRAME_GAP = 0.01


def snap(kept: list, fps: int) -> list:
    """Retimes events (asciicast v3 intervals) so frame k starts at k / fps."""
    events = [json.loads(line) for line in kept]
    frame, clock, out = -1, 0.0, []
    for event in events:
        if frame < 0 or event[0] >= FRAME_GAP:
            frame += 1
            at = frame / fps
            event[0] = round(at - clock, 6)
            clock = at
        else:
            clock += event[0]
        out.append(json.dumps(event, ensure_ascii=False))
    return out


def main(path: str, fps: int = 0) -> None:
    with open(path, encoding="utf-8") as handle:
        lines = handle.read().split("\n")
    header, events = lines[0], [line for line in lines[1:] if line.strip()]
    kept = []
    for line in events:
        event = json.loads(line)
        if event[1] == "o" and EXIT in event[2]:
            break
        kept.append(line)
    if kept:
        first = json.loads(kept[0])
        first[0] = 0
        kept[0] = json.dumps(first, ensure_ascii=False)
    if kept and fps > 0:
        kept = snap(kept, fps)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join([header] + kept) + "\n")


if __name__ == "__main__":
    main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 0)
