# Dashboard parity bench results

Fair side-by-side E2E: identical `Model`, frame count, terminal size, release
builds, same machine. Lunatui times
`lunatui_bench::scenes::lunatui::dashboard::render` (the same painter
`examples/lunatui_dashboard` uses) plus encode+sink. Ratatui times
`scenes::ratatui::dashboard::render` plus Crossterm sink. Each side runs in its
**own child process** so peak RSS is isolated.

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
| lunatui dashboard render + encode + sink | **1337.9 µs** | 1514.0 µs | 815 | 20376.1 | 3824 |
| ratatui dashboard::render + Crossterm sink | **104.1 µs** | 126.5 µs | 493 | 512.7 | 3824 |

### 200×60, 240 frames, 5 runs

| | median-of-medians | median-of-p95 | avg bytes/frame | avg allocs/frame | max RSS KiB |
|---|---|---|---|---|---|
| lunatui | **1623.5 µs** | 1866.4 µs | 3363 | 27872.2 | 4784 |
| ratatui | **401.1 µs** | 491.5 µs | 2636 | 880.8 | 4896 |

## Honesty

On this E2E dashboard path **lunatui is slower than ratatui** (~13× at 80×24,
~4× at 200×60) and allocates far more per frame. Dominant cost: `Paragraph::new`
owns a fresh `String` of the full log body every frame; table header rebuild and
full-grid fill add more. Diff/cell packing still wins on the packed-grid
microbench (ADR 0001 same-Mac: ratatui ~63 µs vs lunatui `[u128]` ~3.8 µs for
identical 200×60).

Raw logs: `dashboard_parity_bench_80x24.txt`, `dashboard_parity_bench_200x60.txt`.
