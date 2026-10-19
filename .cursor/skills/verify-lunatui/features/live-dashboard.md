# Live dashboard example

Dogfood dashboard with scheduler-driven animation; headless `--bench` prints paired frame-time percentiles.

## Sub-features

- `live-snapshot` renders one frame as plain text for captures.
- `live-bench` runs fixed frames without pacing and prints p50/p99/max for lunatui and a same-scene Ratatui 0.30.2 baseline.

## How to get to it (user POV)

- Snapshot: `cargo run --example live_dashboard -- --snapshot --width 80 --height 24`
- Bench: `cargo run --release --example live_dashboard -- --bench --width 200 --height 60 --frames 240`

## Driving it with verify-lunatui

Preconditions:

- Doctor passed for this `RUN_ID`.

- **Snapshot.** `.cursor/skills/verify-lunatui/scripts/run-example.sh live_dashboard -- --snapshot --width 80 --height 24`. Exit code 0; log contains `lunatui live dashboard` header line.

- **Bench.** `.cursor/skills/verify-lunatui/scripts/run-example.sh live_dashboard -- --bench --width 80 --height 24 --frames 4`. This captures a short debug-mode smoke run. For reported frame times, also run the documented release command at the same size and frame count and state the build profile in the PR proof.

- **Proof.** Attach `artifacts/$RUN_ID/live_dashboard.log` or copy to `examples/proof/live_dashboard-verify.log` for the PR.

## Gotchas

- `run-example.sh` uses debug `cargo run` by default; release bench claims need an explicit `cargo run --release` line in Proof when citing frame times.
