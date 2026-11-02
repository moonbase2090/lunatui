#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Proves the timeline tests reject wrong geometry, colors, progress and selection in an isolated copy."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent

MUTANTS = [
    ("span end rounding", "(first, self.ceil_column(end).max(first + 1))",
     "(first, self.floor_column(end).max(first + 1))"),
    ("status colors", "Self::Done => Token::Ok,", "Self::Done => Token::Accent,"),
    ("progress split", "let split = first + (last - first) * percent / 100;",
     "let split = first + (last - first) * (100 - percent) / 100;"),
    ("unstarted planned", "(None, TimelineStatus::Planned) => 0,",
     "(None, TimelineStatus::Planned) => 100,"),
    ("selection", "self.state != WidgetState::Disabled && self.selected == Some(id)",
     "self.state == WidgetState::Disabled && self.selected == Some(id)"),
    ("topmost hit", "            .rev()\n            .find(", "            .find("),
    ("view pan", "Self::place(i128::from(self.start) + i128::from(delta), self.len_i128())",
     "Self::place(i128::from(self.start) - i128::from(delta), self.len_i128())"),
    ("milestone reveal", "let end = end.max(start + 1);", "let end = end.max(start);"),
]


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
        "--features", "timeline,color-profiles", "--test", "timeline",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name.replace(' ', '-')}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "timeline-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="timeline-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, _ = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/timeline-mutants/baseline.log")
        print("PASS baseline: timeline tests pass at HEAD", flush=True)
        source = Path(work) / "lunatui-widgets/src/timeline.rs"
        original = source.read_text()
        for name, before, after in MUTANTS:
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
