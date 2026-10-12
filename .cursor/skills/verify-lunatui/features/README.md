# lunatui verification map

Read this index before driving the repo. Each feature file is a user- or reviewer-facing proof path.

## Baseline preconditions

- Repository root is the lunatui workspace (contains `lunatui/Cargo.toml` and `xtask/`).
- `cargo build --workspace --locked` succeeds.
- `RUN_ID` is set for artifact names (e.g. `export RUN_ID=$(date +%s)`).

## Driving conventions

- Run `scripts/doctor.sh` before the first example drive and after any failed drive.
- Use `scripts/run-example.sh` so stdout/stderr land in `artifacts/$RUN_ID/`.
- Match flags and feature gates in `examples/README.md` (e.g. `color-profiles` for `gradient`).

## Proof and skip reporting

- Gate proof: include `doctor.log` showing `xtask check: ok`.
- Example proof: include the `.log` file and cite the command line in the PR Proof section.
- Committed reference bundle for this skill: `artifacts/verify-seed/` (`doctor.log`, `keys.log`, `hello_box.log`).
- If an example needs a real tty and cannot run headless, record `verified-unreachable` with the attempted command and missing prerequisite.

## Features

- [Workspace gates](./workspace-gates.md) — fmt, comments, and crate README checks via `cargo xtask check`.
- [Hello box example](./hello-box.md) — mixed-width boxed demo and metrics line.
- [Keys example](./keys.md) — decoder demo from captured bytes / stdin.
- [Live dashboard example](./live-dashboard.md) — scheduler-driven demo snapshot and bench summary.
