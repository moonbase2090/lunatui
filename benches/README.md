# lunatui-bench

The benchmark harness for lunatui (PRD §6 and §8 PR 02). It provides the counters every benchmark reports alongside time:

- `CountingAllocator` counts allocations and allocated bytes. Install it as the `#[global_allocator]` of a bench or example binary.
- `CountingWriter` counts `write()` calls and bytes written through any `std::io::Write`.
- `calibration` holds workloads with known costs, used to check the harness itself.
- `ceiling` holds the frame budget (PRD §1.3): every sample of every worst-case benchmark must be at or under the 16 ms ceiling on shared CI runners, and the 8 ms target is reported on a dedicated machine.
- `report` prints a plain-text metrics table.

`CountingAllocator` is the crate's only `unsafe` code: implementing `GlobalAlloc` requires it. Every call forwards unchanged to the wrapped allocator, and the wrapper only adds relaxed atomic counter updates.

The bench targets live in `benches/` and the harness self-test in `tests/`. Run everything with `cargo xtask bench`.
