#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify command palette behavior assertions against mutations in an isolated copy."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
MUTANTS = [
    ("query whitespace", ".filter(|ch| !ch.is_whitespace())", ".filter(|_| true)"),
    ("detail column", "1 + title_width + 2", "1 + title_width + 1"),
    ("shortcut border", ".saturating_sub(1 + hint_cells)", ".saturating_sub(hint_cells)"),
    ("click right edge", "column - list.x >= list.width", "column - list.x > list.width"),
    ("query count gap", "PROMPT_WIDTH + right + u16::from(right > 0)", "PROMPT_WIDTH + right"),
    ("ordered matching", "query.iter().copied().peekable()", "query.iter().rev().copied().peekable()"),
    ("complete query", "wanted.next().is_none()", "wanted.next().is_some()"),
    ("smart case", "self.query.chars().any(|ch| ch.is_ascii_uppercase())", "false"),
    ("scroll window", ".saturating_sub(usize::from(rows))", ".saturating_sub(0)"),
    ("click position", "(at < found).then_some(at)", "(at < found).then_some(0)"),
    ("selection", "self.selected == Some(first + usize::from(row))", "self.selected != Some(first + usize::from(row))"),
    ("whole match grapheme", "ranges.push(at..at + grapheme.len())", "ranges.push(at..at + 1)"),
    ("caret grapheme boundary", ".grapheme_indices(true)\n            .map(|(at, _)| at)", ".char_indices()\n            .map(|(at, _)| at)"),
]


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
        "--features", "command-palette", "--test", "command_palette",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name.replace(' ', '-')}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "command-palette-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="command-palette-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, _ = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/command-palette-mutants/baseline.log")
        print("PASS baseline: command palette tests pass at HEAD", flush=True)
        for name, before, after in MUTANTS:
            source = Path(work) / "lunatui-widgets/src/command_palette.rs"
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
