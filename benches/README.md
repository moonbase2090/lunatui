# lunatui-bench

The benchmark harness for lunatui (PRD §6 and §8 PR 02). It provides the counters every benchmark reports alongside time:

- `CountingAllocator` counts allocations and allocated bytes. Install it as the `#[global_allocator]` of a bench or example binary.
- `CountingWriter` counts `write()` calls and bytes written through any `std::io::Write`.
- `calibration` holds workloads with known costs, used to check the harness itself.
- `ceiling` holds the frame budget (PRD §1.3): every sample of every worst-case benchmark must be at or under the 16 ms ceiling on shared CI runners, and the 8 ms target is reported on a dedicated machine.
- `report` prints a plain-text metrics table.
- `prototype` packs ratatui buffers into the 16-byte cell proposed by ADR 0001 (`docs/adr/0001-cell-format.md`) and diffs them row by row, as a struct and as `u128` words. It is evidence for the design, not lunatui's cell. The wall-clock comparison is `cargo bench -p lunatui-bench --bench criterion_cell_prototype`.
- `layout_prototype` is the closed-form split proposed by ADR 0002 (`docs/adr/0002-layout-engine.md`). It is evidence for the design, not `lunatui-layout`. `cargo test -p lunatui-bench --test layout_prototype` checks it, including a parity corpus against ratatui 0.30.2.
- `scenes::ratatui` holds the ratatui baseline scenes (PRD §6.1 and §8 PR 03), ported 1:1 from the measured ratatui-perf runs and pinned to ratatui 0.30.2: the dashboard, the 5,000-line log, the 500-row table and the truecolor gradient. Its tests pin their deterministic counts. With color output disabled, the counts reproduce the measured `results.txt` exactly, because those runs had `NO_COLOR` set. With colors on, they pin what a color terminal receives.

`CountingAllocator` is the crate's only `unsafe` code: implementing `GlobalAlloc` requires it. Every call forwards unchanged to the wrapped allocator, and the wrapper only adds relaxed atomic counter updates.

The bench targets live in `benches/` and the tests in `tests/`. Run everything with `cargo xtask bench`. The ratatui wall-clock baseline is `cargo bench -p lunatui-bench --bench criterion_ratatui`.

## Widget parity suite

`widget_suite` compares each lunatui widget with its ratatui 0.30.2 equivalent on identical data. A case is one entry in `widget_suite::CASES`: a lunatui drawing function, the equivalent ratatui drawing function, and a flag saying whether the performance gate applies. Every case runs at 80×24 and 200×60 with a `tick` that changes the content a little each frame (scroll offset, ratio, selected tab), so the full-frame scope has a real diff to do.

- **Parity first.** `tests/widget_suite_parity.rs` renders both sides and compares every cell (symbol, foreground, background, bold, dim) at several ticks. A case marked `Parity::Exact` must match cell for cell, so both sides do the same visible work. A case that cannot match says why in `Parity::Documented` and is reported but never gated.
- **Same work, both sides.** Both sides start each render by clearing the whole area and painting the base style, use explicit RGB styles, and draw the same text. Ratatui's `style()` paints blank cells too, so the lunatui side applies the same full-area style before the widget. Virtualized widgets (table, list) draw from a borrowed source on the lunatui side and from the visible slice on the ratatui side, which is how each library is used.
- **Two scopes.** *Render* clears and draws into a reused `Grid` or `Buffer`. *Frame* draws, diffs, encodes and writes: lunatui runs `Encoder::encode_diff` into a counting sink, and ratatui runs `Terminal::draw` on a line-buffered counting sink with a fixed viewport. Terminal-emulator rendering is not measured.
- **Same machine, release builds.** Both sides run in one process, one after the other, and each is the median of 41 samples of 50 iterations after a warm-up. Allocations per frame come from `CountingAllocator` over 100 frames; bytes and `write()` calls per frame come from the sink.
- **The gate.** A gated case fails `cargo xtask bench` when its frame ratio (lunatui over ratatui) is above 1.05 or it allocates more per frame than ratatui. The render-only scope is reported, not gated. Widgets that predate the gate are *tracked*: reported, not failing. A new widget is added with `gated: true`.

- **Color policy.** Both entrypoints pin color output on (`pin_color_output`), so ratatui's crossterm backend writes colors whatever `NO_COLOR` says and both sides encode truecolor. Byte counts do not depend on the environment.
- **Arguments.** `--max-ratio N` must be a finite number above zero; anything else (including `NaN`) is an error, not a disabled gate. `--gate-all` also gates the tracked cases, to see how far they are from the bar.

Run it with `cargo bench -p lunatui-bench --features text-area --bench widget_suite` (add `-- --gate` to enforce, `--max-ratio N` to change the limit). For Criterion timings of the TextArea case at both sizes, run `cargo bench -p lunatui-bench --features text-area --bench criterion_widgets -- text-area`.

