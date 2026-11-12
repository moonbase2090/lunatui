# Verify the dashboard timing gate

`cargo xtask bench` runs the `metrics` bench, which builds the three
`dashboard_fair` examples in release mode and enforces the timing gate at 80×24
and 200×60. Run that step alone with:

```sh
cargo bench --locked -p lunatui-bench --bench metrics
```

Each size runs five pairs of separate release processes, alternating lunatui
then pinned Ratatui 0.30.2. Both use the same model and scripted ticks. The
measured frame includes widget rendering, diffing, encoding, and writes to a
counting sink. Model advancement, process startup, and compilation are outside
the frame samples. This does not measure terminal rendering or input latency.

The gate compares each side's median of per-run median frame times. The default
limit is **1.05×**, allowing lunatui to take up to 5% longer than Ratatui. Equality
passes. A higher ratio, a child failure, or missing measured time or writes exits
nonzero. Five runs reduce the influence of isolated scheduling delays. Review
the printed per-run times and ranges on busy machines.

This relative gate does not assert that the PRD's absolute dashboard tick targets
are met. The fair scene advances its full animation and scroll script, rather
than changing only the small counter used by the separate byte-count gate.

For the report alone, build all three examples before running it:

```sh
cargo build --release --locked -p lunatui \
  --example dashboard_fair_lunatui \
  --example dashboard_fair_ratatui \
  --example dashboard_fair_report
cargo run --release --locked -p lunatui --example dashboard_fair_report -- \
  --width 200 --height 60 --frames 240 --runs 5 --gate
```

Expect `dashboard timing gate: PASS` and exit code 0. `--max-ratio N` changes the
limit for a manual run and accepts only finite positive values. `--gate` requires
at least five runs. Omitting `--gate` preserves the report-only behavior.
`scripts/run-dashboard-fair.sh --gate` also checks both sizes and preserves the
report's failure through its logging pipeline.

Run the regression and enforcement checks from a clean committed checkout:

```sh
cargo test --locked -p lunatui --example dashboard_fair_report
python3 scripts/check-dashboard-fair-gate.py
```

The script archives HEAD into temporary source directories. It verifies both
real size gates, disables the comparison to prove the boundary test fails, then
adds a 5 ms delay inside every measured lunatui frame. The real `metrics` command
must fail. A disconnected-gate control must accept the same slow implementation.
Logs and the source SHA remain in `target/dashboard-fair-gate-check`; temporary
sources are removed. Repeat this drive in a clean VM for PR proof.
