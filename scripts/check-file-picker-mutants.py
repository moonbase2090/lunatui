#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Proves the file picker tests reject incorrect scrolling, marks, sizes, stepping and path handling in an isolated copy."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
        "--features", "file-picker,color-profiles", "--test", "file_picker",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "file-picker-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="file-picker-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, output = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/file-picker-mutants/baseline.log")
        print("PASS baseline: file picker tests pass at HEAD", flush=True)
        source = Path(work) / "lunatui-widgets/src/file_picker.rs"
        original = source.read_text()
        for name, before, after in [
            ("scroll", "(active + 1).saturating_sub(usize::from(rows))", "active.saturating_sub(usize::from(rows))"),
            ("active-row", "self.active_index() == Some(index)", "self.active_index() != Some(index)"),
            ("marks", "self.marked.binary_search(&index).is_ok()", "self.marked.binary_search(&index).is_err()"),
            ("step-clamp", "index.saturating_add(delta.unsigned_abs()).min(last)", "index.saturating_add(delta.unsigned_abs())"),
            ("step-back", "index.saturating_sub(delta.unsigned_abs())", "index.saturating_add(delta.unsigned_abs()).min(last)"),
            ("type-ahead-wrap", "(start + offset) % entries.len()", "start + offset"),
            ("size-rounding", "tenths = (tenths + 512) / 1024;", "tenths /= 1024;"),
            ("parent", "Some(at) => &trimmed[..at],", "Some(at) => &trimmed[..=at],"),
            ("path-clip", "let budget = usize::from(max_cols.saturating_sub(1));", "let budget = usize::from(max_cols);"),
            ("columns", "if area_width >= SHOW_MODIFIED_AT {", "if area_width >= SHOW_SIZE_AT {"),
            ("toggle", "Ok(at) => {\n                marked.remove(at);", "Ok(at) => {\n                marked.insert(at, index);"),
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
