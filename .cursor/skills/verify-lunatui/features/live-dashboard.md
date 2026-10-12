# Live dashboard example

Dogfood dashboard with scheduler-driven animation; non-tty run prints frame-time percentiles.

## Sub-features

- `live-snapshot` renders one frame as plain text for captures.
- `live-bench` runs a short release bench and prints p50/p99/max vs the 8 ms target.

## How to get to it (user POV)

- Snapshot: `cargo run --example live_dashboard -- --snapshot --width 80 --height 24`
- Bench: `cargo run --release --example live_dashboard -- --width 80 --height 24 --frames 4`

## Driving it with verify-lunatui

Preconditions:

- Doctor passed for this `RUN_ID`.

- **Snapshot.** `.cursor/skills/verify-lunatui/scripts/run-example.sh live_dashboard -- --snapshot --width 80 --height 24`. Exit code 0; log contains `lunatui live dashboard` header line.

- **Bench.** `.cursor/skills/verify-lunatui/scripts/run-example.sh live_dashboard -- --width 80 --height 24 --frames 4` with `CARGO_PROFILE_RELEASE=1` or document release in PR proof separately (`cargo run --release ...`).

- **Proof.** Attach `artifacts/$RUN_ID/live_dashboard.log` or copy to `examples/proof/live_dashboard-verify.log` for the PR.

## Gotchas

- `run-example.sh` uses debug `cargo run` by default; release bench claims need an explicit `cargo run --release` line in Proof when citing frame times.
