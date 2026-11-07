#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Prove Linux PTY allocation cannot leak descriptors into an unrelated exec.

Needs Rust, cc and strace. All builds and evidence go in --out. --repo selects
an existing checkout to test, so an old ref can be tested without editing it.
--expect-race requires the old openpty race to reproduce. --fallback injects
ENOTTY into TIOCGPTPEER and proves the name-based fallback is also atomic.
"""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--expect-race", action="store_true")
    parser.add_argument("--fallback", action="store_true")
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("this descriptor and syscall proof requires Linux")
    if args.expect_race and args.fallback:
        parser.error("--fallback cannot be combined with --expect-race")
    repo, out = args.repo.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    fixture = Path(__file__).resolve().parent / "fixtures/pty-cloexec"
    probe = out / "probe"
    (probe / "src").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(fixture / "main.rs", probe / "src/main.rs")
    shutil.copyfile(repo / "Cargo.lock", probe / "Cargo.lock")
    manifest = (
        '[package]\nname = "pty-cloexec-probe"\nversion = "0.0.0"\nedition = "2024"\n'
        '[workspace]\n[dependencies]\nlunatui-term = { path = '
        + json.dumps(str(repo / "lunatui-term")) + ', features = ["pty"] }\n'
    )
    (probe / "Cargo.toml").write_text(manifest)
    with (out / "build.jsonl").open("w") as log:
        run(["cargo", "build", "--manifest-path", str(probe / "Cargo.toml"),
             "--message-format=json"], stdout=log)
    artifacts = [json.loads(line) for line in (out / "build.jsonl").read_text().splitlines()]
    executable = next(item["executable"] for item in artifacts
                      if item.get("reason") == "compiler-artifact"
                      and item.get("executable") and item["target"]["name"] == "pty-cloexec-probe")
    library = out / "pause-openpty.so"
    run(["cc", "-Wall", "-Wextra", "-Werror", "-shared", "-fPIC",
         str(fixture / "pause-openpty.c"), "-o", str(library), "-ldl"])
    ready, release = out / "ready", out / "release"
    ready.unlink(missing_ok=True)
    release.unlink(missing_ok=True)
    env = dict(os.environ, LD_PRELOAD=str(library), PTY_PROBE_READY=str(ready),
               PTY_PROBE_RELEASE=str(release))
    command = ["strace", "-f", "-s", "256", "-o", str(out / "syscalls.log"),
               "-e", "trace=open,openat,ioctl,fcntl,execve"]
    if args.fallback:
        command += ["-e", "inject=ioctl:error=ENOTTY:when=2"]
    result = subprocess.run(command + [executable], env=env, text=True,
                            capture_output=True, timeout=45)
    (out / "probe.log").write_text(result.stdout + result.stderr)
    trace = (out / "syscalls.log").read_text().splitlines()
    masters = [line for line in trace if re.search(r'open(?:at)?\(.*"/dev/(?:pts/)?ptmx"', line)
               and re.search(r'= \d+(?: |$)', line)]
    slaves = [line for line in trace if
              (re.search(r'open(?:at)?\(.*"/dev/pts/\d+"', line) or "TIOCGPTPEER" in line)
              and re.search(r'= \d+(?: |$)', line)]
    def cloexec(line):
        if "TIOCGPTPEER" in line:
            numeric = re.search(r"TIOCGPTPEER, (0x[0-9a-f]+|[0-9]+)\)", line)
            if numeric:
                return bool(int(numeric.group(1), 0) & os.O_CLOEXEC)
        return "O_CLOEXEC" in line

    atomic = bool(masters and slaves) and all(cloexec(line) for line in masters + slaves)
    injected = [index for index, line in enumerate(trace)
                if "TIOCGPTPEER" in line and "ENOTTY" in line and "INJECTED" in line]
    unexpected_injection = any("INJECTED" in line and "TIOCGPTPEER" not in line for line in trace)
    fallback = bool(injected) and not unexpected_injection and any(
        index > injected[0] and re.search(r'open(?:at)?\(.*"/dev/pts/\d+"', line)
        and re.search(r'= \d+(?: |$)', line) and cloexec(line)
        for index, line in enumerate(trace))
    leaked = "inherited_pty=true" in result.stdout
    paused = "paused_openpty=true" in result.stdout
    passed = (result.returncode == 1 and paused and leaked and not atomic) if args.expect_race else (
        result.returncode == 0 and not leaked and atomic and (not args.fallback or fallback))
    summary = dict(head=run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True).stdout.strip(),
                   expect_race=args.expect_race, fallback_requested=args.fallback,
                   fallback_injected=bool(injected), fallback_exercised=fallback,
                   unexpected_injection=unexpected_injection, paused_openpty=paused, inherited_pty=leaked,
                   atomic_creation=atomic, master_syscalls=masters, slave_syscalls=slaves,
                   probe_exit=result.returncode, passed=passed)
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
