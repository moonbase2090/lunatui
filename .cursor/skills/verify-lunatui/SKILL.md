---
name: verify-lunatui
description: Drive lunatui the way users and reviewers do — workspace gates, runnable examples, and captured stdout. Use before claiming a PR Proof section is complete, after changing build/test/example wiring, or when /maintain-verification-skill is requested.
---

# Verify lunatui

lunatui is a Rust workspace library. Users and reviewers prove behavior with `cargo` commands and the documented examples in `examples/README.md`, not a long-lived server.

## Launch

From the repository root:

```sh
cargo build --workspace --locked
```

There is no daemon to keep running. Each example drive is a fresh `cargo run --example <name>` process. Set `RUN_ID` to a unique value when capturing artifacts (default: `date +%s`).

## Doctor

Run the helper (read-only):

```sh
.cursor/skills/verify-lunatui/scripts/doctor.sh
```

Requires: `cargo` on `PATH`, repository root as cwd, and `cargo xtask check` exiting 0. If doctor fails, fix the tree before driving examples.

## Drive

Use the feature map under `features/`. Prefer the exact commands listed in `examples/README.md` and each feature file.

Helpers:

```sh
.cursor/skills/verify-lunatui/scripts/run-example.sh <example-name> [-- extra args...]
```

`run-example.sh` writes combined stdout/stderr to `artifacts/$RUN_ID/<example>.log` and prints the exit code.

For non-tty examples, pipe documented stdin when the feature file says so.

## Evidence

Proof for a PR must show real command output or a capture file, not “tested locally” without logs.

Store verification artifacts under:

`.cursor/skills/verify-lunatui/artifacts/<RUN_ID>/`

Include at minimum for a feature PR:

- `doctor.log` from doctor
- Per touched example: `<example>.log` from `run-example.sh`
- For performance claims: the benchmark line copied from the example stdout (p50/p99/max vs 8 ms target when applicable)

Exercise the user-facing path from the feature map. Do not substitute internal unit tests alone when the PR changes an example or demo command.

## Cleanup

Remove only process state you started; do not `pkill` by name.

After a drive, no `cargo` example process should remain. **Do not delete** `artifacts/<RUN_ID>/` — proof survives cleanup.

## Helpers

| Script | Purpose |
|--------|---------|
| `scripts/doctor.sh` | Workspace gate via `cargo xtask check` |
| `scripts/run-example.sh` | Run one `lunatui` example and capture logs |

Both scripts must be invoked from the repository root.
