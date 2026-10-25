# Performance vs ratatui (v0.1.x preview)

This page compares lunatui with stock **ratatui 0.30.2** on the same machine, with the same work on both sides. Numbers measured on different machines, or with different scopes (a full draw on one side and a render step on the other), are not compared here.

## Same-machine dashboard comparison

The dashboard scene is drawn by two separate release binaries, `dashboard_fair_lunatui` (lunatui widgets plus the ANSI encoder) and `dashboard_fair_ratatui` (ratatui `Terminal` with the Crossterm backend on a line-buffered writer, which mirrors stdout). Both start from the same model and follow the same scripted tick and scroll schedule, and a test (`dashboard_scene_parity`) keeps the two scenes identical cell for cell. A frame covers paint, diff and encode into a counting sink; model advancement is timed outside the frame. Each side runs as its own process, five runs of 240 frames, and the table shows the median of the run medians.

Measured on an Apple M5 Max (macOS, arm64), release profile, October 2026:

| | 80×24 lunatui | 80×24 ratatui | 200×60 lunatui | 200×60 ratatui |
|---|---:|---:|---:|---:|
| Frame time, median | **67 µs** | 84 µs | **251 µs** | 291 µs |
| Frame time, p95 | **76 µs** | 101 µs | **303 µs** | 377 µs |
| Bytes written per frame | **603** | 747 | **2868** | 3199 |
| `write()` calls per frame | 1 | 1 | **1** | 3 |
| Allocations per frame | **81** | 513 | **163** | 881 |
| Peak RSS | **2.9 MiB** | 3.6 MiB | **3.3 MiB** | 4.5 MiB |

Reproduce:

```sh
./scripts/run-dashboard-fair.sh
```

The script writes the raw logs to `target/dashboard-fair` (override with `DASHBOARD_FAIR_OUT`); `FRAMES` and `RUNS` change the sample.

## Limits

- Frames are timed in a headless run into a counting sink. Terminal-emulator rendering and real terminal latency are not measured.
- The lead is smaller at 200×60 than at 80×24, and five runs of 240 frames is a small sample. Treat gaps under about 10% as within noise.
- The lunatui scene draws the tab row, the table cells and the gauge with clipped `put_str` calls, because `Table` has no per-cell styles or column spacing and `Gauge` has no label. A version built only from the widgets would be a separate comparison.
- The two layout solvers squeeze over-constrained table columns differently, so scenes at very small sizes (for example 40×12) differ inside the table.
- Peak RSS is the process high-water mark from `getrusage`.

## Structural facts

| Metric | ratatui 0.30.2 | lunatui |
|---|---|---|
| Bytes per cell | 48 B | **16 B** (ADR 0001 packed cell) |
| Two grids at 200×60, steady state | ~1.1 MiB | **~375 KiB** (from the cell size ratio) |
| Frame budget (worst-case benches) | not gated | **≤16 ms** CI hard gate, **8 ms** target on dedicated hardware (PRD §1.3) |

## Microbenchmarks

The `criterion` benches (`cargo bench -p lunatui-bench`) track lunatui regressions, for example the unchanged-grid diff at about 4.6 µs for 200×60 and the dashboard render step at about 215 µs on the development hardware. These are lunatui-only numbers. Earlier versions of this page set them beside ratatui figures measured on another machine and over a different scope, so they are no longer compared.

## How to read this

- **Preview, not parity:** v0.1.x ships the comparison harness and the core wins, not every PRD target row.
- **Next steps:** wall-clock gates on instruction counts (`gungraun`) and published terminal latency (PRD §6.3) land in later roadmap PRs.
