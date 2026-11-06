#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Run six interleaved, gated widget comparisons and retain raw results and spread."""
import argparse
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--case", default="checkbox", choices=["checkbox", "node-graph"])
CASE = parser.parse_args().case
ROOT = Path(__file__).resolve().parent.parent
OUT = Path(os.environ.get("WIDGET_BENCH_OUT", ROOT / "target" / f"{CASE}-bench"))
OUT.mkdir(parents=True, exist_ok=True)
subprocess.run(["cargo", "bench", "--locked", "-p", "lunatui-bench", "--bench", "widget_suite", "--no-run"], cwd=ROOT, check=True)
report = {"architecture": platform.machine(), "cpu_count": os.cpu_count(),
          "load_before": os.getloadavg(), "rounds": []}
for round_number in range(6):
    command = ["cargo", "bench", "--locked", "-p", "lunatui-bench", "--bench", "widget_suite", "--", "--case", CASE, "--gate"]
    baseline_first = round_number % 2 == 1
    if baseline_first:
        command.append("--baseline-first")
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    (OUT / f"round-{round_number + 1}.log").write_text(result.stdout + result.stderr)
    result.check_returncode()
    order = f"measurement order: baseline_first={str(baseline_first).lower()}"
    if order not in result.stdout:
        raise SystemExit("Benchmark did not honor the requested side order")
    rows = []
    for line in result.stdout.splitlines():
        fields = line.split()
        if not fields or fields[0] != CASE:
            continue
        if len(fields) != 15 or fields[2] != "gated":
            raise SystemExit(f"Unexpected benchmark row: {line}")
        rows.append({"size": fields[1], "render_native_us": float(fields[3][:-2]),
                     "render_baseline_us": float(fields[4][:-2]),
                     "frame_native_us": float(fields[6][:-2]), "frame_baseline_us": float(fields[7][:-2]),
                     "frame_ratio": float(fields[8]), "alloc_native": float(fields[9]),
                     "alloc_baseline": float(fields[10]), "bytes_native": int(fields[13]),
                     "bytes_baseline": int(fields[14])})
    if [row["size"] for row in rows] != ["80x24", "200x60"]:
        raise SystemExit("The benchmark did not exercise both required terminal sizes")
    if any(row["bytes_native"] <= 0 or row["bytes_baseline"] <= 0 for row in rows):
        raise SystemExit("A timed presentation path emitted no bytes")
    report["rounds"].append({"baseline_first": baseline_first, "rows": rows})
    print(f"PASS round {round_number + 1}: baseline_first={baseline_first}", flush=True)
report["load_after"] = os.getloadavg()
report["summary"] = {}
for size in ["80x24", "200x60"]:
    rows = [row for run in report["rounds"] for row in run["rows"] if row["size"] == size]
    summary = {}
    for metric in ["render_native_us", "render_baseline_us", "frame_native_us", "frame_baseline_us", "frame_ratio", "alloc_native", "alloc_baseline"]:
        values = [row[metric] for row in rows]
        summary[metric] = {"median": statistics.median(values), "min": min(values), "max": max(values)}
    report["summary"][size] = summary

(OUT / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report["summary"], indent=2))
