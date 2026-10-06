# lunatui-bench

The benchmark harness for lunatui (PRD §6 and §8 PR 02). It provides the counters every benchmark reports alongside time:

- `CountingAllocator` counts allocations and allocated bytes. Install it as the `#[global_allocator]` of a bench or example binary.
- `CountingWriter` counts `write()` calls and bytes written through any `std::io::Write`.
- `calibration` holds workloads with known costs, used to check the harness itself.
- `ceiling` holds the frame budget (PRD §1.3): every sample of every worst-case benchmark must be at or under the 16 ms ceiling on shared CI runners, and the 8 ms target is reported on a dedicated machine.
- `report` prints a plain-text metrics table.
- `prototype` packs ratatui buffers into the 16-byte cell proposed by ADR 0001 (`docs/adr/0001-cell-format.md`) and diffs them row by row, as a struct and as `u128` words. It is evidence for the design, not lunatui's cell. The wall-clock comparison is `cargo bench -p lunatui-bench --bench criterion_cell_prototype`.
- `scenes::ratatui` holds the ratatui baseline scenes (PRD §6.1 and §8 PR 03), ported 1:1 from the measured ratatui-perf runs and pinned to ratatui 0.30.2: the dashboard, the 5,000-line log, the 500-row table and the truecolor gradient. Its tests pin their deterministic counts. With color output disabled, the counts reproduce the measured `results.txt` exactly, because those runs had `NO_COLOR` set. With colors on, they pin what a color terminal receives.

`CountingAllocator` is the crate's only `unsafe` code: implementing `GlobalAlloc` requires it. Every call forwards unchanged to the wrapped allocator, and the wrapper only adds relaxed atomic counter updates.

The bench targets live in `benches/` and the tests in `tests/`. Run everything with `cargo xtask bench`. The ratatui wall-clock baseline is `cargo bench -p lunatui-bench --bench criterion_ratatui`.
