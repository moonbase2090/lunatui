#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify terminal behavior and fail-closed benchmark writes in an isolated HEAD archive."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
MUTANTS = [
    ("focus gate", "lunatui-widgets/src/terminal.rs", "(self.state == WidgetState::Focused)", "(self.state != WidgetState::Focused)", "screen"),
    ("control enter", "lunatui-widgets/src/terminal/input.rs", r"KeyCode::Enter => vec![b'\r']", r"KeyCode::Enter => vec![b'\n']", "screen"),
    ("application cursor", "lunatui-widgets/src/terminal/input.rs", "else if screen.application_cursor()", "else if !screen.application_cursor()", "screen"),
    ("wide clipping", "lunatui-widgets/src/terminal.rs", "column + 1 >= columns", "column + 1 > columns", "screen"),
    ("control string cap", "lunatui-widgets/src/terminal/output.rs", "self.pending.len() >= MAX_STRING", "self.pending.len() > MAX_STRING", "screen"),
    ("device reply cap", "lunatui-widgets/src/terminal/screen.rs", "self.replies.len() + bytes.len() <= MAX_REPLIES", "self.replies.len() + bytes.len() >= MAX_REPLIES", "screen"),
    ("default foreground", "lunatui-widgets/src/terminal.rs", "let foreground = cx.token(Token::TextPrimary);", "let foreground = cx.token(Token::Warn);", "screen"),
    ("disabled dim", "lunatui-widgets/src/terminal.rs", "style.attrs.set_dim(style.attrs.dim() || disabled)", "style.attrs.set_dim(style.attrs.dim())", "screen"),
    ("child color profile", "lunatui-widgets/src/terminal.rs", "style.fg = profile.downsample(style.fg);", "style.fg = Color::Default;", "screen"),
    ("input queue boundary", "lunatui-term/src/pty/unix.rs", "self.input.len().saturating_add(bytes.len()) > MAX_INPUT", "self.input.len().saturating_add(bytes.len()) >= MAX_INPUT", "process"),
    ("screen area boundary", "lunatui-term/src/pty.rs", "u32::from(cols) * u32::from(rows) > 1_048_576", "u32::from(cols) * u32::from(rows) >= 1_048_576", "process"),
    ("descriptor inheritance", "lunatui-term/src/pty/unix.rs", "libc::F_SETFD, libc::FD_CLOEXEC", "libc::F_SETFD, 0", "process"),
    ("process group cleanup", "lunatui-term/src/pty/unix.rs", "libc::kill(-pid, libc::SIGKILL)", "libc::kill(pid, libc::SIGKILL)", "process"),
    ("kernel resize", "lunatui-term/src/pty/unix.rs", "let winsize = window_size(size);", "let winsize = window_size(self.size);", "process"),
]


def run(checkout, name, logs, kind):
    if kind == "screen":
        command = ["cargo", "test", "--locked", "-p", "lunatui-widgets", "--no-default-features",
                   "--features", "terminal,color-profiles", "--test", "pseudo_terminal"]
    elif kind == "process":
        command = ["cargo", "test", "--locked", "-p", "lunatui-term", "--no-default-features",
                   "--features", "pty", "--test", "pty", "--", "--test-threads=1"]
    else:
        command = ["cargo", "bench", "--locked", "-p", "lunatui-bench", "--bench",
                   "criterion_widgets", "--", "--test", "widget_terminal_"]
    result = subprocess.run(command, cwd=checkout, capture_output=True, text=True, timeout=240,
                            env={**os.environ, "CARGO_TARGET_DIR": str(logs / "build")})
    output = result.stdout + result.stderr
    (logs / f"{name.replace(' ', '-')}.log").write_text(output)
    return result.returncode, output


def change(checkout, filename, before, after):
    path = Path(checkout) / filename
    source = path.read_text()
    if source.count(before) != 1:
        raise SystemExit(f"Update mutation point in {filename}: expected exactly one occurrence")
    path.write_text(source.replace(before, after))
    return path, source


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--process", action="store_true", help="also run child-process checks, in a clean Linux VM")
    parser.add_argument("--bench", action="store_true", help="also inject a failed benchmark sink write")
    options = parser.parse_args()
    if options.process and sys.platform != "linux":
        parser.error("full process mutation checks require a clean Linux VM")
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise SystemExit("Commit the source before checking mutations at HEAD")
    logs = ROOT / "target/terminal-mutants"
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="terminal-mutants-") as checkout:
        archive = subprocess.Popen(["git", "archive", "HEAD"], cwd=ROOT, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", checkout], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise SystemExit("Could not archive HEAD")
        kinds = ["screen"] + (["process"] if options.process else []) + (["bench"] if options.bench else [])
        for kind in kinds:
            code, _ = run(checkout, f"baseline-{kind}", logs, kind)
            if code:
                raise SystemExit(f"{kind} baseline failed; inspect target/terminal-mutants")
            print(f"PASS {kind} baseline at HEAD", flush=True)
        for name, filename, before, after, kind in MUTANTS:
            if kind not in kinds:
                continue
            path, source = change(checkout, filename, before, after)
            try:
                code, output = run(checkout, name, logs, kind)
            finally:
                path.write_text(source)
            if code == 0 or "test result: FAILED" not in output:
                raise SystemExit(f"{name}: expected a behavioral test failure; inspect its log")
            print(f"KILLED {name}: behavioral assertions reject the mutation", flush=True)
        if options.bench:
            path, source = change(checkout, "benches/src/widget_suite.rs", "Ok(buf.len())",
                                  'Err(io::Error::other("injected write failure"))')
            try:
                code, output = run(checkout, "benchmark-write-error", logs, "bench")
            finally:
                path.write_text(source)
            if code == 0 or "widget benchmark frame failed: injected write failure" not in output:
                raise SystemExit("Criterion accepted a failed frame write; inspect its log")
            print("REJECTED benchmark write error: Criterion exits nonzero", flush=True)


if __name__ == "__main__":
    main()
