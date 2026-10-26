# Performance

These are lunatui's own measurements of its dashboard scene on one machine, with one scope. They are not compared with other libraries here.

## Dashboard frame

Measured on an Apple M5 Max (macOS, arm64) with the release profile, October 2026. The scene is the same scripted dashboard at each size. Each run draws 240 frames; the table shows the median of five runs.

| Metric | 80×24 | 200×60 |
|---|---|---|
| Frame time, median | 60 µs | 249 µs |
| Frame time, p95 | 65 µs | 285 µs |
| Allocations per frame | ≈39 | ≈69 |
| Bytes written per frame | 603 | 2868 |
| `write()` calls per frame | 1 | 1 |
| Peak RSS | 2.8 MiB | 3.1 MiB |

A frame covers the widget paint, the diff against the previous frame, the ANSI encoding and the write into a counting sink. Model updates are timed outside the frame. Frames are headless: terminal-emulator rendering and real terminal latency are not measured.

Reproduce:

```sh
./scripts/run-dashboard-fair.sh
```

The script writes the raw logs and a summary to `target/dashboard-fair`. `FRAMES` and `RUNS` change the sample size.

## Structure

| Metric | Value |
|---|---|
| Bytes per cell | 16 B (packed cell, ADR 0001) |
| Frame budget | ≤16 ms CI gate; 8 ms target on dedicated hardware (PRD §1.3) |

## Microbenchmarks

The `criterion` benches (`cargo bench -p lunatui-bench`) track regressions in individual operations. The unchanged-grid diff of the packed-cell prototype (ADR 0001) takes about 4.6 µs at 200×60 on the development hardware; the dashboard render figures are the same scene measured on the baseline, so read them from the bench output rather than this page.

## Limits

- Absolute times move with machine state. Three full runs of the script on one day gave 80×24 medians of 60, 67 and 60 µs, and 200×60 medians of 248, 251 and 249 µs.
- Five runs of 240 frames is a small sample. Treat differences under about 10% as noise.
- Widget benchmarks are gated per widget in `cargo xtask bench` and are not summarised here.
