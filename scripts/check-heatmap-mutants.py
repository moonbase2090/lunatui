#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Proves the heatmap tests reject incorrect colors and selection in an isolated copy."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
        "--features", "heatmap,color-profiles", "--test", "heatmap",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "heatmap-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="heatmap-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, output = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/heatmap-mutants/baseline.log")
        print("PASS baseline: heatmap tests pass at HEAD", flush=True)
        source = Path(work) / "lunatui-widgets/src/heatmap.rs"
        original = source.read_text()
        for name, before, after in [
            ("colors", "BANDS[band(value, scale)]", "BANDS[band(value, scale) * 0]"),
            ("selection", "self.selected == Some(cell) && !disabled", "self.selected == Some(cell) && disabled"),
        ]:
            if original.count(before) != 1:
                raise SystemExit(f"Update the {name} mutation for the current source")
            source.write_text(original.replace(before, after))
            code, output = run_tests(work, name, logs)
            if code == 0 or "test result: FAILED" not in output:
                raise SystemExit(f"{name}: expected an assertion failure; see its log")
            print(f"KILLED {name}: behavioral assertions rejected the changed implementation", flush=True)
            source.write_text(original)


if __name__ == "__main__":
    main()
