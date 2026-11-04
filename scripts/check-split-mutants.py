#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify split resizing and persistence assertions against mutations in an isolated copy."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
MUTANTS = [
    ("resize direction", "split.rs", "i32::from(current) + i32::from(delta)",
     "i32::from(current) - i32::from(delta)"),
    ("resize minimums", "split.rs",
     "let first = clamp_first(wanted, inner, line.min_first, line.min_second);",
     "let first = wanted;"),
    ("nearest divider", "split.rs", "inside && split.axis == axis && nearest.is_none()",
     "inside && split.axis == axis"),
    ("persisted axis", "split/spec.rs", "SplitAxis::Horizontal => 'h',",
     "SplitAxis::Horizontal => 'v',"),
    ("depth cap", "split/spec.rs", "if depth >= MAX_DEPTH {", "if depth > MAX_DEPTH {"),
    ("duplicate pane", "split/spec.rs", "pair[0] == pair[1]", "pair[0] != pair[1]"),
    ("next pane id", "split/spec.rs", "largest >= next", "largest > next"),
    ("maximum ratio", "split/spec.rs", "if ratio > FULL {", "if ratio >= FULL {"),
]


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
        "--features", "split-keys,split-persist", "--test", "split", "--test", "split_keys",
        "--test", "split_persist",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name.replace(' ', '-')}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "split-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="split-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, _ = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/split-mutants/baseline.log")
        print("PASS baseline: split tests pass at HEAD", flush=True)
        for name, filename, before, after in MUTANTS:
            source = Path(work) / "lunatui-widgets/src" / filename
            original = source.read_text()
            if original.count(before) != 1:
                raise SystemExit(f"Update the {name} mutation for the current source")
            source.write_text(original.replace(before, after))
            code, output = run_tests(work, name, logs)
            source.write_text(original)
            if code == 0 or "test result: FAILED" not in output:
                raise SystemExit(f"{name}: expected an assertion failure; see its log")
            print(f"KILLED {name}: behavioral assertions rejected the changed implementation", flush=True)


if __name__ == "__main__":
    main()
