# Performance vs ratatui (v0.1.x preview)

Numbers below are **same-machine, same-scope** where possible. Do not mix
ratatui-perf Xeon baselines with Mac lunatui prototype times.

Reproduce:

```sh
# Packed-cell identical-grid diff (ADR 0001 microbench)
cargo bench -p lunatui-bench --bench criterion_cell_prototype -- cell_diff

# Fair E2E dashboard (render + encode/write sink, identical Model)
./scripts/bench_dashboard_parity.sh
# or:
cargo run --release -p lunatui --example dashboard_parity_bench -- --width 80 --height 24 --frames 240 --runs 5
cargo run --release -p lunatui --example dashboard_parity_bench -- --width 200 --height 60 --frames 240 --runs 5

cargo run --release --example hello_box -- --metrics
```

## Summary table (this Mac, October 2026)

| Metric | ratatui v0.30.2 | lunatui | Scope |
|---|---|---|---|
| Bytes per cell | 48 B | **16 B** | ADR 0001 packed cell |
| Diff, unchanged 200×60 grid | **~63 µs** | **~3.8 µs** (`[u128]` rows) | ADR 0001 same-Mac table (not the older Xeon 122 µs figure) |
| `write()` calls, ~68 KiB frame | **67** | **1** | Encoder batches one buffer (`hello_box --metrics`) |
| Dashboard E2E 80×24 (median frame) | **~75 µs** | **~93 µs** | `dashboard_parity_bench` (render + encode + sink) |
| Dashboard E2E 200×60 (median frame) | **~287 µs** | **~464 µs** | same bench |
| Allocations per dashboard frame 80×24 | **~508** | **~2210** | CountingAllocator over the E2E loop |
| Allocations per dashboard frame 200×60 | **~876** | **~9537** | same |
| Frame budget (worst-case benches) | not gated | **≤16 ms** CI hard gate | **8 ms** target on dedicated hardware (PRD §1.3) |

Raw E2E logs: [`examples/captures/parity/`](../examples/captures/parity/).

## How to read this

- **Diff microbench ≠ E2E dashboard.** The packed `[u128]` diff is much faster
  than ratatui’s cell compare. The full dashboard path (widgets, fill, encode)
  is currently slower than ratatui on this machine; see the parity README for
  hotspots.
- **Preview, not parity:** v0.1.x ships the comparison harness and core wins
  (cell size, single `write()`), not every PRD target row.
- **Fair scenes:** Dashboard model data is shared via
  `lunatui_bench::scenes::ratatui::dashboard::Model`.
