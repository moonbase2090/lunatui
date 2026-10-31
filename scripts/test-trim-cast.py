#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Exercises trailing-frame trimming through the recording script's CLI."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class TrimCast(unittest.TestCase):
    def trim(self, events):
        with tempfile.TemporaryDirectory() as work:
            cast = Path(work) / "sample.cast"
            cast.write_text("\n".join(json.dumps(event) for event in [
                {"version": 3, "term": {"cols": 120, "rows": 36}}, *events,
            ]) + "\n")
            subprocess.run([sys.executable, str(Path(__file__).with_name("trim-cast.py")),
                            str(cast), "30", "2"], check=True, capture_output=True)
            return [json.loads(line) for line in cast.read_text().splitlines()[1:]]

    def test_a_frame_sharing_the_exit_event_is_retained(self):
        result = self.trim([
            [0.4, "o", "\x1b[?2026hWAIT\x1b[?2026l"],
            [0.1, "o", "\x1b[?2026hLIVE\x1b[?2026l\x1b[0m\x1b[?25h\x1b[?1049l"],
            [0.2, "o", "must not appear"],
        ])
        self.assertEqual(result, [
            [0.0, "o", "\x1b[?2026hWAIT\x1b[?2026l"],
            [0.033333, "o", "\x1b[?2026hLIVE\x1b[?2026l"],
        ])

    def test_a_separate_cleanup_event_does_not_add_a_frame(self):
        result = self.trim([
            [0.4, "o", "\x1b[?2026hWAIT\x1b[?2026l"],
            [0.1, "o", "\x1b[?2026hLIVE\x1b[?2026l"],
            [0.2, "o", "\x1b[0m\x1b[?25h\x1b[?1049l"],
        ])
        self.assertEqual(result[-1], [0.033333, "o", "\x1b[?2026hLIVE\x1b[?2026l"])
        self.assertEqual(len(result), 2)


if __name__ == "__main__":
    unittest.main()
