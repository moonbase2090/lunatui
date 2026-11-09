# Latency

Measured library frame time for the scripted dashboard. This page does not
report terminal parse time or end-to-end input-to-photon time. Those were
not measured in this run.

## Host

| Item | Value |
|---|---|
| Date | 2026-11-09 |
| CPU | Apple M5 Max |
| OS | macOS 27.0, arm64 |
| Compiler | rustc 1.99.0 |
| Profile | release |
| Source | `main` at `a37dd155`, plus the p99 field on the same frame samples |

## Method

`./scripts/run-dashboard-fair.sh` with `FRAMES=240` and `RUNS=5`. Each frame
is widget paint, the diff against the previous frame, ANSI encoding, and one
write into a counting sink. The clock starts at the frame and stops when
that write returns. No key is read from a terminal, and no emulator paints
the bytes.

Each run reports the median, the 95th percentile, and the 99th percentile of
its 240 frame times. The table uses the middle value of the five run
summaries after sorting.

## Library frame time

| Size | Median | p95 | p99 | Bytes per frame | Writes per frame | Allocations per frame | Peak RSS |
|---|---|---|---|---|---|---|---|
| 80×24 | 29.4 µs | 33.8 µs | 35.7 µs | 603 | 1 | 39.1 | 2.7 MiB |
| 200×60 | 115.1 µs | 136.4 µs | 150.5 µs | 2868 | 1 | 68.8 | 3.1 MiB |

The five 80×24 medians were 29.1, 29.1, 29.4, 29.6, and 36.0 µs. The five
80×24 p99 values were 34.1, 34.8, 35.7, 39.9, and 46.2 µs. The five 200×60
medians were 113.4, 114.8, 115.1, 115.7, and 116.6 µs. The five 200×60 p99
values were 143.8, 146.2, 150.5, 150.8, and 153.1 µs.

At 200×60 the p99 is 150.5 µs, under the 1 ms library-side budget in the
product plan. That budget is for a key read through to `write()`. This run
does not include the key read, so it is a lower bound on that path, not the
full measurement.

## Not measured

Terminal parse throughput (a device-attributes reply after a frame) and
end-to-end input-to-photon time were not measured. No terminal emulator is
in the timed path.
