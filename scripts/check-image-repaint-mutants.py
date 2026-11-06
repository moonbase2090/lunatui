#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Prove image repaint tests reject mutations in an isolated checkout of HEAD."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', type=Path, default=ROOT / 'target/image-repaint-mutants')
args = parser.parse_args()
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
commands = [
    ['cargo', 'test', '--locked', '-p', 'lunatui-runtime', '--lib'],
    ['cargo', 'test', '--locked', '-p', 'lunatui-term', '--test', 'encode_transcript'],
]
mutants = [
    ('image-invalidation', 'lunatui-runtime/src/run.rs',
     'let repaint = repaint || frames.images.needs_cell_repaint();',
     'let repaint = repaint;'),
    ('full-encoding', 'lunatui-runtime/src/run.rs',
     '.encode_full(&frames.next, &frames.clusters, ColorMode::TrueColor)',
     '.encode_diff(&frames.prev, &frames.next, &frames.clusters, ColorMode::TrueColor)'),
    ('repaint-presentation', 'lunatui-runtime/src/run.rs',
     'frames.next.width() != 0 && frames.next.height() != 0', 'false'),
    ('last-column', 'lunatui-term/src/encode/mod.rs',
     'width: next.width(),', 'width: next.width().saturating_sub(1),'),
    ('run-reuse', 'lunatui-term/src/encode/mod.rs',
     'self.runs.clear();\n        if next.width() != 0 {',
     'if next.width() != 0 {'),
]


def run_tests(checkout, name, environment):
    results = [subprocess.run(command, cwd=checkout, env=environment,
                              capture_output=True, text=True) for command in commands]
    log = ''.join(result.stdout + result.stderr for result in results)
    (out / f'{name}.log').write_text(log)
    return all(result.returncode == 0 for result in results), log


with tempfile.TemporaryDirectory(prefix='lunatui-image-repaint-') as temporary:
    checkout = Path(temporary) / 'repo'
    subprocess.run(['git', 'worktree', 'add', '--detach', str(checkout), head],
                   cwd=ROOT, check=True, capture_output=True)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(out / 'target'))
    try:
        passed, _ = run_tests(checkout, 'baseline', environment)
        if not passed:
            raise SystemExit('Baseline failed; see baseline.log')
        for name, filename, original, changed in mutants:
            path = checkout / filename
            source = path.read_text()
            if source.count(original) != 1:
                raise SystemExit(f'{name}: mutation does not match source exactly once')
            path.write_text(source.replace(original, changed))
            try:
                passed, log = run_tests(checkout, name, environment)
                if passed or 'test result: FAILED.' not in log or 'error[E' in log:
                    raise SystemExit(f'{name}: mutant survived or failed before a behavior test')
                print(f'KILLED {name}: behavior test failed', flush=True)
            finally:
                path.write_text(source)
        passed, _ = run_tests(checkout, 'restored', environment)
        if not passed:
            raise SystemExit('Restored tests failed; see restored.log')
        print(f'PASS: {len(mutants)} mutants killed; restored tests pass at {head}')
    finally:
        subprocess.run(['git', 'worktree', 'remove', '--force', str(checkout)],
                       cwd=ROOT, check=True)
