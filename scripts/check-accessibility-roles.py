#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Run the headless role-export example and verify both real JSON snapshots."""
from pathlib import Path
import json
import subprocess

ROOT = Path(__file__).resolve().parent.parent
result = subprocess.run([
    "cargo", "run", "--quiet", "--locked", "-p", "lunatui-runtime",
    "--no-default-features", "--features", "accessibility", "--example", "accessibility_roles",
], cwd=ROOT, capture_output=True, text=True, check=True)
snapshots = [json.loads(line) for line in result.stdout.splitlines()]
expected = [
    {"version": 1, "regions": [
        {"id": 1, "role": "button", "area": {"x": 1, "y": 1, "width": 8, "height": 1}, "z": 0, "focused": True},
        {"id": 2, "role": "text_input", "area": {"x": 1, "y": 3, "width": 20, "height": 1}, "z": 0, "focused": False},
    ]},
    {"version": 1, "regions": [
        {"id": 1, "role": "button", "area": {"x": 1, "y": 1, "width": 8, "height": 1}, "z": 0, "focused": False},
        {"id": 2, "role": "text_input", "area": {"x": 1, "y": 3, "width": 20, "height": 1}, "z": 0, "focused": True},
    ]},
]
if snapshots != expected:
    raise SystemExit(f"Unexpected accessibility snapshots: {snapshots!r}")
print(result.stdout, end="")
print("PASS: two valid version-1 snapshots preserve roles/geometry and follow Tab focus")
