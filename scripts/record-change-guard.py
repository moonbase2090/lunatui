#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Record real guarded editor events and check their state and decoded frames."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
DRAFT = "Release notes\n\nKeep the draft until its author approves a replacement.\nEvery change has a source. The application decides what to accept."
INCOMING = "Release notes\n\nThe reviewed replacement is ready.\nAccepted with permission. Continue editing the reviewed draft."


def run(*command, **kwargs):
    return subprocess.check_output(command, cwd=ROOT, text=True, **kwargs)


def hashes(path, *options):
    output = run("ffmpeg", "-loglevel", "error", "-i", str(path), *options,
                 "-pix_fmt", "rgb24", "-f", "framemd5", "-")
    return [line.rsplit(",", 1)[1].strip() for line in output.splitlines()
            if line and not line.startswith("#")]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/media/change-guard.gif")
    parser.add_argument("--proof-dir", type=Path, default=ROOT / "target/change-guard-recording")
    args = parser.parse_args()
    agg = os.environ.get("AGG", "agg")
    version = run(agg, "--version").strip()
    if version != "agg 1.9.0+lunatui.2":
        raise SystemExit("Set AGG to agg 1.9.0+lunatui.2, as used by scripts/record-showcase.sh")
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "lunatui",
                    "--example", "change_guard"], cwd=ROOT, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    binary = target / "release/examples" / ("change_guard.exe" if os.name == "nt" else "change_guard")
    proof = args.proof_dir.resolve()
    proof.mkdir(parents=True, exist_ok=True)

    def snapshot(tick, *flags):
        return run(str(binary), "--snapshot", "--tick", str(tick), *flags)

    checkpoints = {}
    for tick in [0, 30, 90, 140, 185, 248, 280, 299]:
        text = snapshot(tick)
        state = snapshot(tick, "--state")
        document = snapshot(tick, "--document")
        assert ("protected=true" in state) == (tick < 140), (tick, state)
        assert ("dialog_open=true" in state) == (tick < 185), (tick, state)
        assert ("focus=Some(HitId(2))" in state) == (tick < 185), (tick, state)
        if tick >= 185:
            assert "focus=Some(HitId(1))" in state
        if tick < 210:
            assert document == DRAFT, (tick, document)
        elif tick < 280:
            assert document == DRAFT.replace("Release notes", "Release notes - ready for review", 1)
        else:
            assert document == INCOMING
        if tick == 30:
            assert "reason: Escape(" in state and "source: Keyboard, cancel: true" in state
            assert "Close refused." in text
        if tick == 90:
            assert "reason: Edit, source: Agent, cancel: true" in state
            assert "External edit refused." in text
        if tick >= 280:
            assert "reason: Edit, source: Agent, cancel: false" in state
            assert "External edit accepted." in text
        checkpoints[tick] = {"state": state, "document": document, "text": text}
    frames = [[round(tick / 30, 6), "o", "\x1b[0m\x1b[2J\x1b[H" + snapshot(tick, "--ansi")]
              for tick in range(300)]
    cast = proof / "change-guard.cast"
    header = {"version": 2, "width": 100, "height": 28, "timestamp": 0,
              "env": {"TERM": "xterm-256color"}}
    cast.write_text("\n".join(json.dumps(item, ensure_ascii=False) for item in [header, *frames]) + "\n")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([agg, "--font-size", "14", "--line-height", "1.4", "--fps-cap", "60",
                    "--last-frame-duration", "0.8", str(cast), str(args.output)], check=True)
    metadata = json.loads(run("ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
                              "-show_entries", "stream=width,height,duration,nb_read_frames", "-of", "json",
                              str(args.output)))["streams"][0]
    decoded = hashes(args.output)
    assert 20 <= len(set(decoded)), len(set(decoded))
    assert 9 <= float(metadata["duration"]) <= 12, metadata
    assert 1000 < args.output.stat().st_size < 2_000_000
    # Confirm the refusal, permission, dismissal and replacement frames differ in the actual GIF.
    sampled = [hashes(args.output, "-ss", str(time), "-vf", "fps=30", "-frames:v", "1")[0]
               for time in [0.2, 1.2, 3.2, 4.9, 6.4, 9.6]]
    assert len(set(sampled)) == len(sampled), sampled
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(args.output), "-ss", "3.2", "-vf", "fps=30",
                    "-frames:v", "1", "-update", "1", str(args.output.with_suffix(".png"))], check=True)
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(args.output),
                    "-vf", "fps=9/10,scale=600:-1,tile=3x3", "-frames:v", "1", "-update", "1",
                    str(proof / "contact.png")], check=True)
    (proof / "checkpoints.json").write_text(json.dumps(checkpoints, indent=2) + "\n")
    manifest = {"source_head": run("git", "rev-parse", "HEAD").strip(),
                "source_sha256": hashlib.sha256((ROOT / "examples/change_guard.rs").read_bytes()).hexdigest(),
                "gif_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
                "source_frames": 300, "decoded_frames": len(decoded), "unique_decoded_frames": len(set(decoded)),
                "bytes": args.output.stat().st_size, "renderer": version, **metadata}
    (proof / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
