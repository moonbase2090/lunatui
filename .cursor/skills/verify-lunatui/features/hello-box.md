# Hello box example

The hello box draws mixed-width text in a bordered grid and can print draw metrics.

## Sub-features

- `hello-metrics` prints timing/allocation style metrics for the demo frame.

## How to get to it (user POV)

- Run `cargo run --example hello_box -- --metrics` from the repository root (see `examples/README.md`).

## Driving it with verify-lunatui

Preconditions:

- Doctor passed for this `RUN_ID`.

- **Metrics run.** `.cursor/skills/verify-lunatui/scripts/run-example.sh hello_box -- --metrics`. Exit code 0; log contains `hello_box` and metric output (write/alloc lines per README).

- **Proof.** Attach `artifacts/$RUN_ID/hello_box.log` or paste a representative tail into the PR Proof section.

## Gotchas

- Flags like `--encode`, `--session`, and `--oracle` need extra setup; use the paths documented in `examples/README.md` when those subsystems change.
