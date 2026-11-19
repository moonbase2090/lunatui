#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Compare native widget frame instruction counts against a frozen main revision."""
import argparse
from contextlib import contextmanager
from dataclasses import asdict, dataclass
from fractions import Fraction
import json
import os
import platform
from pathlib import Path
import shutil
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
WORKER = Path("benches/examples/widget_instruction_worker.rs")
SIZES = ((80, 24), (200, 60))


@dataclass(frozen=True)
class Sample:
    instructions: int
    elapsed_seconds: float


@dataclass(frozen=True)
class Frame:
    label: str
    current: Sample
    baseline: Sample | None


def regressions(frames, limit):
    return [f"{frame.label} instructions {frame.current.instructions} exceed main "
            f"{frame.baseline.instructions} by more than {limit}%"
            for frame in frames if frame.baseline is not None
            and frame.current.instructions * 100 > frame.baseline.instructions * (100 + limit)]


def parse_count(text):
    lines = text.splitlines()
    events = next((line[8:].split() for line in lines if line.startswith("events: ")), None)
    summary = next((line[9:].split() for line in lines if line.startswith("summary: ")), None)
    if events is None or "Ir" not in events:
        raise ValueError("Callgrind output has no Ir event")
    if summary is None or len(summary) != len(events):
        raise ValueError("Callgrind summary does not match its events")
    value = summary[events.index("Ir")]
    if not value.isascii() or not value.isdecimal() or int(value) <= 0:
        raise ValueError("Callgrind Ir must be a positive integer")
    return int(value)


def percentage(value):
    try:
        result = Fraction(value)
    except (ValueError, ZeroDivisionError) as error:
        raise argparse.ArgumentTypeError("regression limit must be a finite percentage") from error
    if result < 0:
        raise argparse.ArgumentTypeError("regression limit must be nonnegative")
    return result


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


@contextmanager
def frozen_base(explicit):
    ref = None
    try:
        if explicit:
            commit = git("rev-parse", "--verify", f"{explicit}^{{commit}}")
        else:
            ref = f"refs/widget-instruction-base/{uuid.uuid4().hex}"
            subprocess.run(["git", "fetch", "--no-tags", "--no-write-fetch-head", "origin",
                            f"refs/heads/main:{ref}"], cwd=ROOT, check=True)
            commit = git("rev-parse", ref)
        yield commit
    finally:
        if ref is not None:
            git("update-ref", "-d", ref)


@contextmanager
def baseline_tree(commit, out):
    tree = out / "baseline"
    git("worktree", "add", "--detach", str(tree), commit)
    injected = not (tree / WORKER).exists()
    try:
        if injected:
            (tree / WORKER).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / WORKER, tree / WORKER)
        yield tree
    finally:
        if injected:
            (tree / WORKER).unlink(missing_ok=True)
        git("worktree", "remove", str(tree))


def build_worker(tree, out, name, env):
    result = subprocess.run([
        "cargo", "build", "--profile", "bench", "--locked", "-p", "lunatui-bench", "--features", "text-area",
        "--example", "widget_instruction_worker", "--message-format=json",
        "--target-dir", str(TARGET),
    ], cwd=tree, capture_output=True, text=True, env=env)
    (out / f"{name}-build.log").write_text(result.stdout + result.stderr)
    result.check_returncode()
    for line in result.stdout.splitlines():
        item = json.loads(line)
        if (item.get("reason") == "compiler-artifact"
                and item["target"]["name"] == "widget_instruction_worker" and item.get("executable")):
            executable = out / f"{name}-worker"
            shutil.copy2(item["executable"], executable)
            return executable
    raise ValueError("cargo did not produce the instruction worker")


def cases(executable, flag):
    return subprocess.check_output([str(executable), flag], text=True).splitlines()


def collect(executable, case, width, height, out, side):
    profile = out / f"{case}-{width}x{height}-{side}.callgrind"
    start = time.perf_counter()
    result = subprocess.run([
        "valgrind", "--quiet", "--tool=callgrind", "--collect-atstart=no",
        "--toggle-collect=*instruction_frames*", f"--callgrind-out-file={profile}",
        str(executable), case, str(width), str(height),
    ], capture_output=True, text=True)
    elapsed = time.perf_counter() - start
    (profile.with_suffix(".log")).write_text(result.stdout + result.stderr)
    result.check_returncode()
    return Sample(parse_count(profile.read_text()), elapsed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="explicit baseline revision; default fetches and freezes main")
    parser.add_argument("--case", help="one registered gated case; default checks all gated cases")
    parser.add_argument("--max-regression", type=percentage, default=Fraction(3))
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    if sys.platform != "linux" or not shutil.which("valgrind"):
        parser.error("instruction checking needs Linux and Valgrind")
    head = git("rev-parse", "HEAD")
    out = (args.out or TARGET / "widget-instructions" / f"{head[:12]}-{uuid.uuid4().hex}").resolve()
    out.mkdir(parents=True, exist_ok=False)
    toolchain = subprocess.check_output(["rustup", "show", "active-toolchain"], cwd=ROOT, text=True).split()[0]
    env = dict(os.environ, RUSTUP_TOOLCHAIN=toolchain)
    report = {"head": head, "dirty": bool(git("status", "--porcelain")),
              "max_regression_percent": str(args.max_regression), "frames": [],
              "toolchain": toolchain, "architecture": platform.machine(),
              "cpu_count": os.cpu_count(), "load_before": os.getloadavg(),
              "valgrind": subprocess.check_output(["valgrind", "--version"], text=True).strip()}
    current = build_worker(ROOT, out, "current", env)
    selected = cases(current, "--list-gated")
    if not selected or (args.case is not None and args.case not in selected):
        parser.error("the selected case is not a registered gated case")
    if args.case:
        selected = [args.case]
    frames = []
    with frozen_base(args.base) as base, baseline_tree(base, out) as tree:
        report["base"] = base
        baseline = build_worker(tree, out, "baseline", env)
        available = set(cases(baseline, "--list"))
        if not available:
            raise ValueError("baseline worker listed no exact cases")
        for case in selected:
            for width, height in SIZES:
                prior = (collect(baseline, case, width, height, out, "main")
                         if case in available else None)
                current_sample = collect(current, case, width, height, out, "current")
                frame = Frame(f"{case} {width}x{height}", current_sample, prior)
                frames.append(frame)
                report["frames"].append(asdict(frame))
                (out / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
                print(f"{frame.label}: instructions {current_sample.instructions}, "
                      f"main {prior.instructions if prior else 'new case'}", flush=True)
    failures = regressions(frames, args.max_regression)
    report["load_after"] = os.getloadavg()
    report["failures"] = failures
    (out / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
    for failure in failures:
        print(failure, file=sys.stderr)
    print(f"Instruction proof: {out}")
    return int(bool(failures))


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"widget instruction check failed: {error}", file=sys.stderr)
        raise SystemExit(2) from error
