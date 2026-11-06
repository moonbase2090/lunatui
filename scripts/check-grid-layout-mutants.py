#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Prove grid layout tests reject mutations in an isolated checkout of HEAD."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', type=Path, default=ROOT / 'target/grid-layout-mutants')
args = parser.parse_args()
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
command = ['cargo', 'test', '--locked', '-p', 'lunatui-layout', '--test', 'grid']
mutants = [
    ('column-span',
     'self.span(area.rows.clone(), area.columns.clone())\n    }',
     'self.span(area.rows.clone(), area.columns.start..area.columns.start + 1)\n    }'),
    ('empty-range', 'range.start >= range.end', 'range.start > range.end'),
    ('duplicate-name',
     'if self.areas.iter().any(|area| area.name == name) {',
     'if self.areas.iter().any(|area| area.name == name && area.rows.is_empty()) {'),
    ('name-identity',
     'self.areas.iter().find(|area| area.name == name)?',
     'self.areas.iter().find(|area| area.name.eq_ignore_ascii_case(name))?'),
    ('interior-endpoint', 'end = end.max(position + size);', 'end = position + size;'),
    ('origin-saturation', 'self.origin.0.saturating_add(x)', 'self.origin.0.wrapping_add(x)'),
    ('axis-diagnostic',
     'rows.needs_solver || columns.needs_solver',
     'rows.needs_solver && columns.needs_solver'),
]
with tempfile.TemporaryDirectory(prefix='lunatui-grid-mutants-') as temporary:
    checkout = Path(temporary) / 'repo'
    subprocess.run(['git', 'worktree', 'add', '--detach', str(checkout), head],
                   cwd=ROOT, check=True, capture_output=True)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(out / 'target'))
    path = checkout / 'lunatui-layout/src/grid.rs'
    try:
        for name, original, changed in mutants:
            source = path.read_text()
            if source.count(original) != 1:
                raise SystemExit(f'{name}: mutation does not match source exactly once')
            path.write_text(source.replace(original, changed))
            try:
                result = subprocess.run(command, cwd=checkout, env=environment,
                                        capture_output=True, text=True)
                log = result.stdout + result.stderr
                (out / f'{name}.log').write_text(log)
                if result.returncode == 0 or 'test result: FAILED.' not in log or 'error[E' in log:
                    raise SystemExit(f'{name}: mutant survived or failed before a behavior test')
                print(f'KILLED {name}: behavior test failed', flush=True)
            finally:
                path.write_text(source)
        result = subprocess.run(command, cwd=checkout, env=environment,
                                capture_output=True, text=True)
        (out / 'restored.log').write_text(result.stdout + result.stderr)
        result.check_returncode()
        print(f'PASS: {len(mutants)} mutants killed; restored tests pass at {head}')
    finally:
        subprocess.run(['git', 'worktree', 'remove', '--force', str(checkout)],
                       cwd=ROOT, check=True)
