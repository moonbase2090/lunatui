#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Prove graph behavior tests reject real mutations in an isolated checkout."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', type=Path, default=ROOT / 'target/node-graph-mutants')
args = parser.parse_args()
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
command = ['cargo', 'test', '--locked', '-p', 'lunatui-widgets', '--no-default-features',
           '--features', 'node-graph', '--test', 'node_graph']
mutants = [
    ('output-port', 'canvas.rs', ".glyph = Glyph::Char('├');", ".glyph = Glyph::Char('?');"),
    ('obstacle-routing', 'canvas.rs', 'if next != goal', 'if next == usize::MAX'),
    ('disabled-style', 'view.rs', 'style.attrs = style.attrs.set_dim(true);', 'style.attrs = style.attrs.set_dim(false);'),
    ('label-budget', 'layout.rs', '65_536_usize', '262_144_usize'),
    ('input-port-order', 'layout.rs', 'entries.sort_by_key(|&(port, node)| (port, node));', 'entries.sort_by_key(|&(_, node)| node);'),
    ('wide-grapheme-clip', 'canvas.rs', 'columns > usize::from(width - column)', 'columns > usize::from(width - column) + 1'),
]
with tempfile.TemporaryDirectory(prefix='lunatui-node-mutants-') as temporary:
    checkout = Path(temporary) / 'repo'
    subprocess.run(['git', 'worktree', 'add', '--detach', str(checkout), head], cwd=ROOT, check=True, capture_output=True)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(out / 'target'))
    try:
        for name, file, original, changed in mutants:
            path = checkout / 'lunatui-widgets/src/node_graph' / file
            source = path.read_text()
            if source.count(original) != 1:
                raise SystemExit(f'{name}: mutation does not match the source exactly once')
            path.write_text(source.replace(original, changed))
            try:
                result = subprocess.run(command, cwd=checkout, env=environment, capture_output=True, text=True)
                log = result.stdout + result.stderr
                (out / f'{name}.log').write_text(log)
                if result.returncode == 0 or 'test result: FAILED.' not in log or 'error[E' in log:
                    raise SystemExit(f'{name}: mutant survived or failed before a behavior assertion')
                print(f'KILLED {name}: behavior assertion failed', flush=True)
            finally:
                path.write_text(source)
        result = subprocess.run(command, cwd=checkout, env=environment, capture_output=True, text=True)
        (out / 'restored.log').write_text(result.stdout + result.stderr)
        result.check_returncode()
        print(f'PASS: {len(mutants)} mutants killed; restored tests pass at {head}')
    finally:
        subprocess.run(['git', 'worktree', 'remove', '--force', str(checkout)], cwd=ROOT, check=True)
