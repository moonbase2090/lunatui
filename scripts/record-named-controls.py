#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Record real profile edits and verify the decoded motion and value-free controls."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
NAMES = ["Full name", "Email", "Notifications", "Workspace ID", "Save changes", "Cancel"]
ROLES = ["text_input", "text_input", "checkbox", "text_input", "button", "button"]
ERROR = "Enter a complete email address."
CHECKPOINTS = {
    0: ("", "", 1),
    48: ("Example u", "", 1),
    72: ("Example user", "", 1),
    120: ("Example user", "user@", 2),
    180: ("Example user", "user@example.com", 2),
    198: ("Example user", "user@example.com", 3),
    216: ("Example user", "user@example.com", 3),
    240: ("Example user", "user@example.com", 3),
    258: ("Example user", "user@example.com", 3),
    270: ("Example user", "user@example.com", 5),
    294: ("Example user", "user@example.com", 5),
}


def probe(path):
    return json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
        "-show_entries", "stream=width,height,duration,nb_read_frames", "-of", "json", str(path),
    ], text=True))["streams"][0]


def frame_hashes(path, *options):
    output = subprocess.check_output([
        "ffmpeg", "-loglevel", "error", "-i", str(path), *options, "-pix_fmt", "rgb24", "-f", "framemd5", "-",
    ], text=True)
    return [line.rsplit(",", 1)[1].strip() for line in output.splitlines()
            if line and not line.startswith("#")]


def value_at(lines, control):
    area = control["area"]
    line = lines[area["y"] + area["height"] // 2]
    return line[area["x"]:area["x"] + area["width"]].replace("│", "").strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/media/named-controls.gif")
    parser.add_argument("--still", type=Path)
    parser.add_argument("--proof-dir", type=Path, default=ROOT / "target/named-controls-recording")
    args = parser.parse_args()
    agg = os.environ.get("AGG", "agg")
    version = subprocess.check_output([agg, "--version"], text=True).strip()
    if version != "agg 1.9.0+lunatui.2":
        raise SystemExit("Set AGG to the pinned agg 1.9.0+lunatui.2 used by scripts/record-showcase.sh")
    proof = args.proof_dir.resolve()
    proof.mkdir(parents=True, exist_ok=True)
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "lunatui-field",
                    "--example", "ui_field"], cwd=ROOT, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    binary = target / "release/examples" / ("ui_field.exe" if os.name == "nt" else "ui_field")
    frames = []
    snapshots = []
    for tick in range(300):
        output = subprocess.check_output([str(binary), "--snapshot", "--ansi", "--tick", str(tick)], cwd=ROOT)
        text = subprocess.check_output([str(binary), "--snapshot", "--tick", str(tick)], cwd=ROOT, text=True)
        snapshot = json.loads(subprocess.check_output([str(binary), "--accessibility", "--tick", str(tick)], cwd=ROOT))
        controls = snapshot["regions"]
        assert snapshot["version"] == 2
        assert [c["name"] for c in controls] == NAMES
        assert [c["role"] for c in controls] == ROLES
        assert [c["disabled"] for c in controls] == [False, False, False, True, False, False]
        focused = [c["id"] for c in controls if c["focused"]]
        assert len(focused) == 1 and 4 not in focused, (tick, focused)
        lines = text.splitlines()
        values = [value_at(lines, controls[index]) for index in [0, 1, 3]]
        for value in [*values, "Receipts and alerts go here", ERROR]:
            if len(value) >= 4:
                assert value not in json.dumps(snapshot), (tick, value)
        assert "scripted" not in text.lower() and "timer" not in text.lower()
        assert "Email me about deploys" in text
        if tick in CHECKPOINTS:
            name, email, focus = CHECKPOINTS[tick]
            assert values == [name, email, "ws-7f3a91"], (tick, values)
            assert focused == [focus], (tick, focused)
        if tick in {198: "●───", 216: "━━━●", 240: "●───", 258: "━━━●"}:
            expected = {198: "●───", 216: "━━━●", 240: "●───", 258: "━━━●"}[tick]
            assert value_at(lines, controls[2]).startswith(expected), tick
        if tick == 120:
            assert ERROR in text
        if tick == 180:
            assert ERROR not in text
        if tick == 294:
            assert "Changes saved" in text and "Your profile is up to date." in text
        frames.append([round(tick * 0.033, 6), "o", "\x1b[0m\x1b[2J\x1b[H" + output.decode()])
        snapshots.append({"tick": tick, "snapshot": snapshot})
    cast = proof / "named-controls.cast"
    header = {"version": 2, "width": 120, "height": 36, "timestamp": 0, "env": {"TERM": "xterm-256color"}}
    cast.write_text("\n".join(json.dumps(item, ensure_ascii=False) for item in [header, *frames]) + "\n")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([agg, "--font-size", "14", "--line-height", "1.4", "--fps-cap", "60",
                    "--last-frame-duration", "0.8", str(cast), str(args.output)], check=True)
    assert 1000 < args.output.stat().st_size < 2_000_000
    meta = probe(args.output)
    duration = float(meta["duration"])
    hashes = frame_hashes(args.output)
    unique = len(set(hashes))
    assert 6 <= duration <= 12, duration
    assert len(hashes) >= 40 and unique >= 40, (len(hashes), unique)
    toggle = snapshots[198]["snapshot"]["regions"][2]["area"]
    cell_w, cell_h = int(meta["width"]) / 122, int(meta["height"]) / 37
    left, top = round((1 + toggle["x"]) * cell_w), round((0.5 + toggle["y"]) * cell_h)
    crop = f"fps=30,crop={round(27 * cell_w)}:{round(cell_h)}:{left}:{top}"
    toggles = [frame_hashes(args.output, "-ss", str(time), "-vf", crop, "-frames:v", "1")[0]
               for time in [6.5, 7.15, 7.9, 8.5]]
    assert all(left != right for left, right in zip(toggles, toggles[1:])), toggles
    still = args.still or args.output.with_suffix(".png")
    still.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(args.output), "-ss", "4.1", "-vf", "fps=30",
                    "-frames:v", "1", "-update", "1", str(still)], check=True)
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(args.output),
                    "-vf", "fps=9/10,scale=600:-1,tile=3x3", "-frames:v", "1", "-update", "1",
                    str(proof / "contact.png")], check=True)
    (proof / "snapshots.json").write_text(json.dumps(snapshots, indent=2) + "\n")
    manifest = {
        "source_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "source_sha256": hashlib.sha256((ROOT / "lunatui-field/examples/ui_field.rs").read_bytes()).hexdigest(),
        "gif_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
        "still_sha256": hashlib.sha256(still.read_bytes()).hexdigest(),
        "source_frames": 300, "decoded_frames": len(hashes), "unique_decoded_frames": unique,
        "decoded_duration_seconds": duration, "bytes": args.output.stat().st_size,
        "virtual_ticks": [0, 299], "width": 120, "height": 36, "theme": "dark", "renderer": version,
        "pixel_width": int(meta["width"]), "pixel_height": int(meta["height"]),
        "toggle_pixel_checks": "off/on/off/on in the actual decoded control area",
        "metadata_checks": "six names/roles/disabled states; focus skips disabled field; no values or messages",
        "scene_checks": "partial name; invalid email; completed email; notifications; save focus; saved confirmation",
    }
    (proof / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"PASS: {duration:.2f}s, {len(hashes)} decoded frames ({unique} unique), "
          f"{manifest['bytes']} bytes; real typing/validation/focus/toggle/save; still captured")


if __name__ == "__main__":
    main()
