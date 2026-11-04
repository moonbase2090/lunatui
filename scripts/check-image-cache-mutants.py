#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Check image cache regressions against mutations in an isolated HEAD archive."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
MATCH = "cached.image == current && cached.size == size"
MUTANTS = [
    ("cache reuse", "pixel/immediate.rs", MATCH, "false && " + MATCH),
    ("generation key", "pixel/immediate.rs", MATCH,
     "cached.image == Uploaded { generation: cached.image.generation, ..current } && cached.size == size"),
    ("placement size key", "pixel/immediate.rs", MATCH, "cached.image == current"),
    ("opacity key", "pixel/immediate.rs", MATCH,
     "cached.image == Uploaded { opacity: cached.image.opacity, ..current } && cached.size == size"),
    ("crop key", "pixel/immediate.rs", MATCH,
     "cached.image == Uploaded { crop_x: cached.image.crop_x, crop_y: cached.image.crop_y, width: cached.image.width, height: cached.image.height, ..current } && cached.size == size"),
    ("opacity pixels", "pixel/immediate.rs", "u32::from(crop.opacity)", "255"),
    ("omitted image", "pixel.rs",
     "self.immediate\n                .retain(|cached| self.frame.iter().any(|placed| placed.id == cached.image.id));", ""),
    ("cleared image", "pixel.rs", "self.immediate.clear();", ""),
]


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-term", "--no-default-features",
        "--features", "pixel-layer", "--test", "pixel_layer", "immediate_",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name.replace(' ', '-')}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "image-cache-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="image-cache-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, _ = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/image-cache-mutants/baseline.log")
        print("PASS baseline: image cache tests pass at HEAD", flush=True)
        for name, filename, before, after in MUTANTS:
            source = Path(work) / "lunatui-term/src" / filename
            original = source.read_text()
            if original.count(before) != 1:
                raise SystemExit(f"Update the {name} mutation for the current source")
            source.write_text(original.replace(before, after))
            code, output = run_tests(work, name, logs)
            source.write_text(original)
            if code == 0 or "test result: FAILED" not in output:
                raise SystemExit(f"{name}: expected an assertion failure; see its log")
            print(f"KILLED {name}: image assertions rejected the changed implementation", flush=True)


if __name__ == "__main__":
    main()
