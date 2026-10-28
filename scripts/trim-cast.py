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
then drop frames. The preview opens each frame with the synchronized-output mark
BEGIN, so frame k is everything from its mark up to the next one, however the
recorder split or merged the writes; frame k becomes one event at k / fps and is
never shown half drawn. In a recording without marks, a frame is the run of events
written without a pause of FRAME_GAP seconds or more. With a frame count as
the third argument, a recording that does not hold exactly that many frames (one drawn
late enough to run into the next) is an error.
"""
import json
import sys

EXIT = "\x1b[?1049l"
BEGIN = "\x1b[?2026h"
FRAME_GAP = 0.01


def frames_of(events: list) -> list:
    """The output of each frame: split at the marks when there are any, else at pauses."""
    text = "".join(event[2] for event in events)
    if BEGIN in text:
        head, *rest = text.split(BEGIN)
        frames = [BEGIN + part for part in rest]
        frames[0] = head + frames[0]
        return frames
    frames = []
    for event in events:
        if not frames or event[0] >= FRAME_GAP:
            frames.append("")
        frames[-1] += event[2]
    return frames


def snap(kept: list, fps: int) -> tuple:
    """Retimes events (asciicast v3 intervals) so all of frame k lands at k / fps, and
    counts the frames."""
    frames = frames_of([event for event in map(json.loads, kept) if event[1] == "o"])
    out = [
        json.dumps([0.0 if k == 0 else round(1 / fps, 6), "o", text], ensure_ascii=False)
        for k, text in enumerate(frames)
    ]
    return out, len(frames)


def main(path: str, fps: int = 0, frames: int = 0) -> None:
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
        kept, found = snap(kept, fps)
        if frames and found != frames:
            sys.exit(f"trim-cast: {path} holds {found} frames, expected {frames}")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join([header] + kept) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:2], *(int(arg) for arg in sys.argv[2:4]))
