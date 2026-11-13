#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Record actual Field widgets and verify their value-free named-control snapshots."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/media/named-controls.gif")
    parser.add_argument("--proof-dir", type=Path, default=ROOT / "target/named-controls-recording")
    args = parser.parse_args()
    proof = args.proof_dir.resolve()
    proof.mkdir(parents=True, exist_ok=True)
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "lunatui-field",
                    "--example", "ui_field"], cwd=ROOT, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    binary = target / "release/examples" / ("ui_field.exe" if os.name == "nt" else "ui_field")
    frames = []
    snapshots = []
    for index in range(60):
        tick = index * 4
        output = subprocess.check_output([str(binary), "--snapshot", "--ansi", "--tick", str(tick)], cwd=ROOT)
        snapshot = json.loads(subprocess.check_output([str(binary), "--accessibility", "--tick", str(tick)], cwd=ROOT))
        controls = snapshot["regions"]
        assert snapshot["version"] == 2
        assert [c["name"] for c in controls] == ["Full name", "Email", "Notifications", "Workspace ID", "Save changes", "Cancel"]
        assert [c["role"] for c in controls] == ["text_input", "text_input", "checkbox", "text_input", "button", "button"]
        assert [c["disabled"] for c in controls] == [False, False, False, True, False, False]
        if index in {0: 2, 23: 3, 46: 1}:
            assert [c["id"] for c in controls if c["focused"]] == [{0: 2, 23: 3, 46: 1}[index]]
        for value in ["Example user", "user@example.com", "ws-7f3a91", "Receipts and alerts go here"]:
            assert value not in json.dumps(snapshot)
        frames.append([index / 10, "o", "\x1b[2J\x1b[H" + output.decode()])
        snapshots.append({"tick": tick, "snapshot": snapshot})
    cast = proof / "named-controls.cast"
    header = {"version": 2, "width": 120, "height": 36, "timestamp": 0, "env": {"TERM": "xterm-256color"}}
    cast.write_text("\n".join(json.dumps(item, ensure_ascii=False) for item in [header, *frames]) + "\n")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["agg", "--font-size", "14", "--line-height", "1.4", "--fps-cap", "10",
                    "--last-frame-duration", "0.1", str(cast), str(args.output)], check=True)
    assert 1000 < args.output.stat().st_size < 2_000_000
    (proof / "snapshots.json").write_text(json.dumps(snapshots, indent=2) + "\n")
    (proof / "manifest.json").write_text(json.dumps({
        "source_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "source_sha256": hashlib.sha256((ROOT / "lunatui-field/examples/ui_field.rs").read_bytes()).hexdigest(),
        "gif_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
        "frames": 60, "virtual_ticks": [0, 236], "width": 120, "height": 36,
        "metadata_checks": "all six names/roles/disabled states; fixed focus checkpoints; no values or messages",
    }, indent=2) + "\n")
    print("PASS: 60 real widget frames; six named controls; disabled and focus states; no exported values")


if __name__ == "__main__":
    main()
