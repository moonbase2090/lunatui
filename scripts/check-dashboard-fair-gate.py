#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Prove dashboard timing enforcement against real frames in an isolated HEAD archive."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
TESTS = ["cargo", "test", "--locked", "-p", "lunatui", "--example", "dashboard_fair_report"]
METRICS = ["cargo", "bench", "--locked", "-p", "lunatui-bench", "--bench", "metrics"]


def run(checkout, output, name, command):
    result = subprocess.run(
        command, cwd=checkout, capture_output=True, text=True, timeout=900,
        env={**os.environ, "CARGO_TARGET_DIR": str(output / "build")},
    )
    text = result.stdout + result.stderr
    (output / f"{name}.log").write_text(text)
    return result.returncode, text


def change(checkout, filename, before, after):
    path = Path(checkout) / filename
    source = path.read_text()
    if source.count(before) != 1:
        raise SystemExit(f"Update mutation point in {filename}: expected one occurrence")
    path.write_text(source.replace(before, after))
    return path, source


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/dashboard-fair-gate-check")
    options = parser.parse_args()
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking enforcement at HEAD")
    output = options.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output / "head.txt").write_text(subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True))
    with tempfile.TemporaryDirectory(prefix="dashboard-fair-gate-") as checkout:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", checkout], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, text = run(checkout, output, "baseline-tests", TESTS)
        if code or "6 passed" not in text:
            raise SystemExit("Report test baseline failed; inspect baseline-tests.log")
        code, text = run(checkout, output, "baseline-metrics", METRICS)
        if code or text.count("dashboard timing gate: PASS") != 2:
            raise SystemExit("Metrics baseline did not pass both sizes; inspect baseline-metrics.log")
        print("PASS baseline: report tests and metrics gates at both sizes", flush=True)

        path, source = change(checkout, "examples/dashboard_fair_report.rs",
                              "if ratio > max_ratio {", "if false {")
        try:
            code, text = run(checkout, output, "disabled-comparison", TESTS)
        finally:
            path.write_text(source)
        if code == 0 or "timing_gate_rejects_one_nanosecond_over_the_boundary ... FAILED" not in text:
            raise SystemExit("Tests accepted disabled timing enforcement; inspect disabled-comparison.log")
        print("KILLED disabled comparison: boundary regression fails", flush=True)

        path, source = change(checkout, "benches/src/dashboard_fair.rs",
                              "let start = Instant::now();\n            luna_dash::render(",
                              "let start = Instant::now();\n            std::thread::sleep(Duration::from_millis(5));\n            luna_dash::render(")
        try:
            code, text = run(checkout, output, "slow-frames", METRICS)
            if code == 0 or "dashboard timing gate: FAIL" not in text:
                raise SystemExit("Metrics accepted slow frames; inspect slow-frames.log")
            print("REJECTED slow frames: metrics exits nonzero", flush=True)
            wire, original = change(checkout, "benches/benches/metrics.rs",
                                    "    dashboard_timing_gate()?;",
                                    "    if std::hint::black_box(false) { dashboard_timing_gate()?; }")
            try:
                code, text = run(checkout, output, "disconnected-gate", METRICS)
            finally:
                wire.write_text(original)
            if code or "dashboard timing gate: FAIL" in text:
                raise SystemExit("Disconnected control failed unexpectedly; inspect disconnected-gate.log")
            print("CONTROL disconnected gate accepts the same slow implementation", flush=True)
        finally:
            path.write_text(source)


if __name__ == "__main__":
    main()
