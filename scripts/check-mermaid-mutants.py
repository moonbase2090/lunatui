#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify Mermaid parsing, layout and rendering assertions against mutations in an isolated copy."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
MUTANTS = [
    ("flow direction", "flow_parse.rs", '"LR" => Some(Self::LeftRight)', '"LR" => Some(Self::RightLeft)'),
    ("edge weight", "flow_parse.rs", "run('-', Weight::Solid)", "run('-', Weight::Thick)"),
    ("node cap", "flow_parse.rs", "self.flow.nodes.len() >= LIMIT", "self.flow.nodes.len() > LIMIT"),
    ("source byte limit", "../mermaid.rs", "source.len() > MAX_BYTES", "source.len() >= MAX_BYTES"),
    ("dotted reply", "seq_parse.rs", '("-->>", true, End::Head)', '("-->>", false, End::Head)'),
    ("message numbering", "seq_parse.rs", "self.count += 1", "self.count += 2"),
    ("frame depth", "seq_parse.rs", "self.open.len() >= MAX_DEPTH", "self.open.len() > MAX_DEPTH"),
    ("extra arrowhead rejection", "seq_parse.rs", "matches!(ch, '<' | '>')", "false"),
    ("combining mark cleanup", "label.rs", "!ch.is_control()", "(!ch.is_control() && lunatui_core::char_width(ch) > 0)"),
    ("grapheme measurement", "canvas.rs", "i32::try_from(width(text)).unwrap_or(i32::MAX)", "text.chars().map(|ch| i32::try_from(lunatui_core::char_width(ch)).unwrap_or(0)).sum()"),
    ("grapheme drawing", "../mermaid.rs", "Glyph::Cluster(index) => canvas.cluster(index)", 'Glyph::Cluster(_) => ""'),
    ("scroll clamp", "../mermaid.rs", "self.scroll.0.min(max_x)", "self.scroll.0.max(max_x)"),
]


def run_tests(checkout, name, logs):
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
        "--features", "mermaid", "--test", "mermaid",
    ], cwd=checkout, env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")},
        capture_output=True, text=True)
    output = result.stdout + result.stderr
    (logs / f"{name.replace(' ', '-')}.log").write_text(output)
    return result.returncode, output


def main():
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutants at HEAD")
    logs = ROOT / "target" / "mermaid-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mermaid-mutants-") as work:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", work], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        code, _ = run_tests(work, "baseline", logs)
        if code != 0:
            raise SystemExit("Baseline failed; see target/mermaid-mutants/baseline.log")
        print("PASS baseline: Mermaid tests pass at HEAD", flush=True)
        for name, filename, before, after in MUTANTS:
            source = Path(work) / "lunatui-widgets/src/mermaid" / filename
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
