#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Reject broken named-control behavior in an isolated archive of committed HEAD."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
COMMANDS = {
    "field": ["cargo", "test", "--locked", "-p", "lunatui-field", "--test", "controls"],
    "runtime": ["cargo", "test", "--locked", "-p", "lunatui-runtime", "--no-default-features",
                "--features", "app", "--test", "controls", "--test", "accessibility"],
}
MUTANTS = [
    ("missing-registry", "lunatui-runtime/src/hit.rs", "annotation: Annotation::Control(metadata),",
     "annotation: Annotation::Role(metadata.record(id, area, z).role),", "field"),
    ("ambiguous-first-match", "lunatui-runtime/src/hit.rs", "if extra == 0", "if extra <= 1", "runtime"),
    ("disabled-button-activation", "lunatui-widgets/src/named_control.rs",
     ".disabled(self.disabled || self.body.control_disabled())", ".disabled(self.disabled)", "field"),
    ("lost-scroll-translation", "lunatui-widgets/src/render_cx.rs",
     "target.append_viewport(source, from, x, y);", "target.append_viewport(source, from, 0, 0);", "field"),
    ("unnamed-form-field", "lunatui-field/src/lib.rs", "!name.trim().is_empty()", "!name.is_empty()", "field"),
    ("unnamed-direct-control", "lunatui-runtime/src/hit/control.rs", "!name.trim().is_empty()", "!name.is_empty()", "runtime"),
    ("exported-input-value", "lunatui-runtime/src/hit/accessibility.rs",
     r'b"{\"version\":2,\"regions\":["', r'b"{\"version\":2,\"value\":\"private-token-123\",\"regions\":["', "field"),
    ("input-outside-form", "lunatui-field/src/form.rs",
     ".min(area.x.saturating_add(area.width.saturating_sub(1)))", ".max(area.x)", "field"),
]


def run(checkout, output, name, kind):
    result = subprocess.run(COMMANDS[kind], cwd=checkout, capture_output=True, text=True,
                            timeout=900, env={**os.environ, "CARGO_TARGET_DIR": str(output / "build")})
    text = result.stdout + result.stderr
    (output / f"{name}.log").write_text(text)
    return result.returncode, text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/named-controls-check")
    options = parser.parse_args()
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutations at HEAD")
    output = options.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output / "head.txt").write_text(subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True))
    with tempfile.TemporaryDirectory(prefix="named-controls-check-") as checkout:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", checkout], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        for kind in COMMANDS:
            code, _ = run(checkout, output, f"baseline-{kind}", kind)
            if code:
                raise SystemExit(f"{kind} baseline failed; inspect its log")
            print(f"PASS {kind} baseline", flush=True)
        for name, filename, before, after, kind in MUTANTS:
            path = Path(checkout) / filename
            source = path.read_text()
            if source.count(before) != 1:
                raise SystemExit(f"Update mutation point in {filename}: expected one occurrence")
            try:
                path.write_text(source.replace(before, after))
                code, text = run(checkout, output, name, kind)
            finally:
                path.write_text(source)
            if code == 0 or "test result: FAILED" not in text:
                raise SystemExit(f"{name}: expected a behavioral test failure; inspect its log")
            print(f"KILLED {name}: behavioral assertions fail", flush=True)


if __name__ == "__main__":
    main()
