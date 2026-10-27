#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Drops the leading blank and the trailing alternate-screen exit from an asciicast.

The wait before the first paint is a blank cursor frame. The widget preview also
leaves the alternate screen after its last frame. The first event delay is cleared
and events from the first alternate-screen exit onward are removed.
"""
import json
import sys

EXIT = "\x1b[?1049l"


def main(path: str) -> None:
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
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join([header] + kept) + "\n")


if __name__ == "__main__":
    main(sys.argv[1])
