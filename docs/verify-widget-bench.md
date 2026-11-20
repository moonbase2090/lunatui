# Verify the widget benchmark gate

The widget suite reports wall-clock timings at 80×24 and 200×60. Those timings do not affect the exit status. Its allocation bar remains a comparison against Ratatui. On Linux with Valgrind installed, `cargo xtask bench` also rejects native frame instruction counts more than 3% above the matching workload on `main`, as specified by PRD §6.

## Run the checks

```sh
python3 scripts/test-widget-instructions.py
cargo test --locked -p lunatui-bench --test widget_suite_gate
cargo test --locked -p lunatui-bench --test widget_suite_parity
cargo test --locked -p xtask
python3 scripts/check-widget-instructions.py
```

The instruction checker fetches `main` into a unique temporary ref and freezes its SHA. `--base REVISION` selects an already available revision instead, including for an offline run. It builds each revision's native worker with the same release profile, features and toolchain, then measures every gated case at both sizes. A new case with no baseline reports `new case`; its allocation and parity checks still apply, and it enters instruction regression checking after it reaches `main`.

Each worker warms 200 frames outside collection and runs 32 complete frames inside `instruction_frames`. [Callgrind's collection toggle](https://snapshots.sourceware.org/valgrind/trunk/2026-01-11_19-41_1768160461/html/cl-manual.html#cl-manual.options) counts the named function and its callees. Constructors, warm-up, report formatting and process startup are excluded. The worker verifies its completed batch and propagates frame-write errors. The parser requires a positive `Ir` count. Missing tools, failed workers, an unmatched collection symbol or malformed output fail the check.

A baseline that predates the worker target receives the same worker driver in an isolated worktree. Its production crates and scene implementations remain those of the baseline revision. The temporary worktree and ref are removed after measurement. Raw Callgrind files, build logs, elapsed profiling times and a commit-stamped `summary.json` stay under `target/widget-instructions/`. Only instruction counts decide the 3% regression threshold; comparisons use exact integer arithmetic.

`widget_suite --gate` and `scripts/bench-widget.py` enforce allocation counts while retaining the timing, write and byte reports. `--max-ratio` is rejected by the timing report; instruction limits belong to `check-widget-instructions.py --max-regression N`. When Valgrind is absent, `cargo xtask bench` reports that instruction checking was skipped. CI installs Valgrind and runs the complete gate.

## Check the regression guard

Policy tests inject elapsed-time noise while keeping instruction counts fixed, require an instruction regression to fail even when its elapsed time looks faster, and check the exact threshold with large counters. Parser fixtures reject missing or invalid counts. The Rust allocation tests independently reject increased and non-finite allocation counts.

For a real counter failure, rename `instruction_frames` and its call site in the current worker without changing the collection pattern. Run the checker against an unmodified baseline. It must fail because no instructions were collected. Restore the source before proceeding.

For a real work regression, add computation consumed by `std::hint::black_box` inside the current native frame implementation, such as `LunatuiRun::frame`. The instruction checker must reject the added work against the unmodified baseline. Keep the worker driver unchanged, because initial baselines receive the same driver. Restore the source and require the original check to pass. Do not introduce sleeps, retries or a larger threshold to obtain a passing result.
