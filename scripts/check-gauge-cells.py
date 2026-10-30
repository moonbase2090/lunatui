#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

"""Compare the deployment scene's complete cell streams at two commits."""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile


def run(command, repo, **kwargs):
    return subprocess.run(command, cwd=repo, check=True, **kwargs)


def revision(repo, ref):
    return subprocess.check_output(
        ["git", "rev-parse", "--verify", "--end-of-options", ref + "^{commit}"],
        cwd=repo,
        text=True,
    ).strip()


def streams(executable, source):
    result = {}
    for width, height, theme, profile in [
        (120, 36, "dark", "truecolor"),
        (80, 24, "dark", "truecolor"),
        (120, 36, "light", "truecolor"),
        (120, 36, "dark", "ansi16"),
    ]:
        name = f"{width}x{height}-{theme}-{profile}"
        result[name] = subprocess.check_output(
            [str(executable), "--case", "deploy", "--side", "lunatui",
             "--width", str(width), "--height", str(height), "--frames", "240",
             "--fps", "100000", "--theme", theme, "--profile", profile],
            cwd=source,
            timeout=120,
        )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="Reference commit before the Gauge change")
    parser.add_argument("--head", default="HEAD", help="Commit to verify, default HEAD")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    base = revision(repo, args.base)
    head = revision(repo, args.head)
    print(f"Base {base}\nHead {head}", flush=True)
    captures = []
    with tempfile.TemporaryDirectory(prefix="gauge-cells-") as directory:
        scratch = Path(directory)
        for name, commit in [("base", base), ("head", head)]:
            source = scratch / name
            target = scratch / (name + "-target")
            log = scratch / (name + ".log")
            with log.open("wb") as output:
                run(["git", "worktree", "add", "--detach", str(source), commit], repo,
                    stdout=output, stderr=subprocess.STDOUT)
            try:
                with log.open("ab") as output:
                    run(["cargo", "build", "--locked", "-p", "lunatui", "--features",
                         "color-profiles", "--example", "widget_preview", "--target-dir", str(target)],
                        source, stdout=output, stderr=subprocess.STDOUT, timeout=900)
                executable = target / "debug" / "examples" / ("widget_preview.exe" if os.name == "nt" else "widget_preview")
                captures.append(streams(executable, source))
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                print(log.read_text(errors="replace").replace(str(scratch), "<scratch>").replace(str(Path.home()), "<home>"))
                raise
            finally:
                run(["git", "worktree", "remove", str(source)], repo,
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for name, expected in captures[0].items():
        actual = captures[1][name]
        if actual != expected:
            raise SystemExit(f"FAIL {name}: deployment cell stream differs")
        print(f"PASS {name}: {hashlib.sha256(actual).hexdigest()}")
    print("4 deployment streams, 240 frames each, are byte-identical")


if __name__ == "__main__":
    main()
