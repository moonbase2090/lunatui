# Performance vs ratatui (v0.1.0 preview)

This page publishes comparison numbers for the **v0.1.0 preview**. Ratatui baselines in the first column come from the measured **ratatui v0.30.2** runs in [PRD §1.3](PRD.md). Lunatui numbers in the second column were reproduced on the bench harness in this repository (release profile, October 2026). Targets in the PRD are goals, not promises for every row.

Reproduce locally:

```sh
cargo bench -p lunatui-bench --bench criterion_cell_prototype -- 'cell_diff/200x60/words_identical'
cargo bench -p lunatui-bench --bench criterion_ratatui -- 'dashboard/200x60/no_color/render'
cargo bench -p lunatui-bench --bench metrics
cargo run --release --example hello_box -- --metrics
```

## Summary table

| Metric | ratatui v0.30.2 (baseline) | lunatui v0.1.0 (measured) | Notes |
|---|---|---|---|
| Bytes per cell | 48 B | **16 B** | ADR 0001 packed cell |
| Two grids 200×60 (steady state) | ~1.1 MiB | **~375 KiB** | From cell size ratio |
| Diff, unchanged 200×60 grid | **122 µs** | **~4.6 µs** | `criterion_cell_prototype`, `words_identical`; production uses `diff_runs` on dirty rows |
| `write()` calls, ~68 KiB frame | **67** | **1** | Encoder batches one buffer (`hello_box --metrics`, PRD §1.3) |
| Dashboard render 200×60 (CPU only) | **~0.48 ms** E2E baseline | **~215 µs** render | `criterion_ratatui` `dashboard/200x60/no_color/render` on dev hardware; not full encode |
| Allocations per dashboard frame 200×60 | **~980** | *not yet parity-benched* | Ratatui: `metrics` bench ≈883 allocs/frame over 100 frames |
| Frame budget (worst-case benches) | not gated | **≤16 ms** CI hard gate | **8 ms** target on dedicated hardware (PRD §1.3) |

## How to read this

- **Preview, not parity:** v0.1.0 ships the comparison harness and core wins (cell size, diff prototype, single `write()`), not every PRD target row.
- **Fair scenes:** Dashboard, log, table, and gradient scenes are ported 1:1 in `lunatui-bench` (PR 03) with fidelity tests against ratatui output.
- **Next steps:** Wall-clock gates on instruction counts (`gungraun`) and published terminal latency (PRD §6.3) land in later roadmap PRs.
