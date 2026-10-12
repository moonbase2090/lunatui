# Workspace gates

Reviewers expect the tree to pass the local gate job before merge.

## Sub-features

- `xtask-check` runs comment, README, and example wiring checks.

## How to get to it (user POV)

- A contributor runs the documented gate from the repo root before opening a PR.

## Driving it with verify-lunatui

Preconditions:

- `scripts/doctor.sh` is executable.

- **Gate.** Run `.cursor/skills/verify-lunatui/scripts/doctor.sh 2>&1 | tee .cursor/skills/verify-lunatui/artifacts/$RUN_ID/doctor.log`. Exit code 0 and output contains `xtask check: ok`.

- **Proof.** Keep `doctor.log` for the PR Proof section or paste the final line.

## Gotchas

- Doctor does not run Clippy or the full CI matrix; add `cargo clippy --workspace --all-targets --all-features -- -D warnings` when the PR touches Rust APIs.
