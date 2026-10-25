# Dashboard parity bench results

Fair side-by-side E2E: identical `lunatui_bench::scenes::ratatui::dashboard::Model`,
frame count, terminal size, release builds, same machine.

Reproduce:

```sh
./scripts/bench_dashboard_parity.sh
# or
cargo run --release -p lunatui --example dashboard_parity_bench -- --width 80 --height 24 --frames 240 --runs 5
```

## Aggregate (this machine, 2026-10-25)

### 80×24, 240 frames, 5 runs

| | median-of-medians | median-of-p95 | avg bytes/frame | avg allocs/frame | max RSS KiB |
|---|---|---|---|---|---|
| lunatui widgets + encode + sink | **93.4 µs** | 105.0 µs | 671 | 2209.8 | 4480 |
| ratatui 0.30.2 Terminal + Crossterm sink | **74.9 µs** | 88.9 µs | 493 | 507.7 | 4480 |

### 200×60, 240 frames, 5 runs

| | median-of-medians | median-of-p95 | avg bytes/frame | avg allocs/frame | max RSS KiB |
|---|---|---|---|---|---|
| lunatui | **463.6 µs** | 500.0 µs | 1891 | 9537.3 | 5680 |
| ratatui | **286.8 µs** | 339.8 µs | 2636 | 875.8 | 5680 |

## Honesty

On this E2E dashboard path **lunatui is slower than ratatui** (~1.2× at 80×24,
~1.6× at 200×60). Hotspots: full-grid fill every frame, widget rebuild, per-line
log clipping `String`s, and higher allocation traffic versus ratatui’s reused
`Terminal` buffer. Diff/cell packing still wins on the packed-grid microbench
(ADR 0001 same-Mac: ratatui ~63 µs vs lunatui `[u128]` ~3.8 µs for identical
200×60).

Raw logs: `dashboard_parity_bench_80x24.txt`, `dashboard_parity_bench_200x60.txt`.
