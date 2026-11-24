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

The `metrics` step also runs the `dashboard_fair` timing gate at 80×24 and 200×60.
It compares the median frame times across five paired release runs, with a 1.05×
lunatui/Ratatui limit. See [verification commands and failure proof](../docs/verify-dashboard-fair.md)
for its scope and tolerance.

## Graphite cases

`graphite_cases` (feature `graphite-pixels`) measures Graphite mode's pixel path as it exists today. Each frame does two things:
- it rasterizes an activity-monitor-shaped scene (header chips, three seat rows with lanes, a timeline and a key row) with `graphite-core`'s CPU `PixelBuffer`;
- it hands the result to `PixelLayer`, which encodes what changed as terminal images into a counting sink.

Cells are 8×16 pixels at 1x.

| Case | Each frame |
| --- | --- |
| `graphite-changed-1x`, `graphite-changed-2x` | Redraws only the seat rows, the rectangle a poll changes (about half the width), and sends them as their own image over the unchanged window image. This is a normal Graphite frame. |
| `graphite-full-1x`, `graphite-full-2x` | Redraws and resends the whole window, as a first frame, a resize or a mode change does. |

- **Instructions.** The instruction check runs the four cases at 80×24 and 200×60 cells with the widget cases. Each one is gated at 3% over `main`. Graphite frames are large and deterministic, so the worker warms 2 frames and collects 8, instead of 200 and 32.
- **Ceiling.** The `ceiling` bench times 200 frames of each case in a 185×51 window, which is 1480×816 at 1x, the window the [spike](../docs/graphite-mode-spike.md) measured. The changed cases are gated on the 16 ms ceiling. The full repaints are reported but not gated, because the design lets a full repaint spread over several frames ([frame ceiling](../docs/graphite-mode-design.md#the-8-ms-frame-ceiling)).
- **Proof.** The `graphite_cases` test checks three things:
  - a tick changes pixels only inside the changed rectangle;
  - the changed repaint equals the same rectangle cropped from a full repaint;
  - the image stream of a changed frame carries only that rectangle, and of a full frame only the window, pixel for pixel.

### Widget cases

`graphite_monitor` draws the activity monitor with lunatui's own Graphite widgets, as an application frame does: `Badge`, `Kbd`, `List` and `Sparkline` paint through `RenderCx` and `PixelCx` in Graphite mode, the cells are encoded as a diff, and each widget image goes out through the Kitty `PixelLayer` into the counting sink. The scene fits any window from 40×8 cells.

| Case | Each frame |
| --- | --- |
| `graphite-monitor-changed-1x`, `graphite-monitor-changed-2x` | The monitor: a badge header, up to eight seat rows with lanes, a detail list and key hints. Redraws only the seat rows and their lanes, as a poll does; the other images are listed again without being resent. |
| `graphite-monitor-full-1x`, `graphite-monitor-full-2x` | Redraws and resends the whole monitor. |
| `graphite-dense-changed-1x`, `graphite-dense-changed-2x` | The worst case: a seat row with a lane on every row between the header and the key hints, across the whole width. Redraws every seat row. |
| `graphite-dense-full-1x`, `graphite-dense-full-2x` | Redraws and resends the whole dense scene. |

- **Instructions.** All eight cases are gated at 3% over `main` at 80×24 and 200×60, like the raster cases.
- **Ceiling.** The changed cases are gated on the 16 ms ceiling in the 185×51 window, including the dense worst case at 1x. Full repaints are reported. The dense worst case at 2x is reported, not gated: its changed frame sends about 24 MB of base64, which fits on a developer machine (about 8 ms) but not on the CI runner, which is about three times slower.
- **Proof.** The `graphite_monitor` test checks that the widgets paint every cell of a full frame (no text is left in cells) with at most 256 images, the most `PixelLayer` lists in a frame; that a changed frame's seat rows match a full frame while the cells outside stay as first drawn; that a changed frame transmits fewer images and fewer bytes than a full one; that the dense scene draws more than the monitor; and that frames are deterministic and change each tick.

## Widget parity suite

`widget_suite` compares each lunatui widget with its ratatui 0.30.2 equivalent on identical data. A case is one entry in `widget_suite::CASES`: a lunatui drawing function, the equivalent ratatui drawing function, and a flag saying whether the performance gate applies. Every case runs at 80×24 and 200×60 with a `tick` that changes the content a little each frame (scroll offset, ratio, selected tab), so the full-frame scope has a real diff to do.

- **Parity first.** `tests/widget_suite_parity.rs` renders both sides and compares every cell (symbol, foreground, background, bold, dim) at several ticks. A case marked `Parity::Exact` must match cell for cell, so both sides do the same visible work. A case that cannot match says why in `Parity::Documented` and is reported but never gated.
- **Same work, both sides.** Both sides start each render by clearing the whole area and painting the base style, use explicit RGB styles, and draw the same text. Ratatui's `style()` paints blank cells too, so the lunatui side applies the same full-area style before the widget. Virtualized widgets (table, list) draw from a borrowed source on the lunatui side and from the visible slice on the ratatui side, which is how each library is used. The `nested-menu` case compares cascading menus with `tui-menu` 0.3.1 on the same cached state and theme colors. The `popup` and `scrollview` cases compare with `tui-popup` 0.7.7 and `tui-scrollview` 0.6.8, which are dependencies of this crate only. Both libraries center the popup the same way when the margin is even; with an odd margin `tui-popup` puts the spare cell before the popup and lunatui after, as `Flex::Center` already does, so the dialog leaves an even margin at both sizes. The scroll view runs with scrollbars hidden, because the two libraries draw different scrollbars. Side-by-side recordings of the release console on both libraries, from `scripts/record-widget.sh` (`CASE=popup-scrollview`): [80×24](../docs/media/popup-scrollview-80x24-truecolor.mp4), [200×60](../docs/media/popup-scrollview-200x60-truecolor.mp4), and 16 colors at [80×24](../docs/media/popup-scrollview-80x24-ansi16.mp4) and [200×60](../docs/media/popup-scrollview-200x60-ansi16.mp4).
- **Two scopes.** *Render* clears and draws into a reused `Grid` or `Buffer`. *Frame* draws, diffs, encodes and writes: lunatui runs `Encoder::encode_diff` into a counting sink, and ratatui runs `Terminal::draw` on a line-buffered counting sink with a fixed viewport. Terminal-emulator rendering is not measured.
- **Same machine, release builds.** Both sides run in one process, one after the other, and each is the median of 41 samples of 50 iterations after a warm-up. Allocations per frame come from `CountingAllocator` over 100 frames; bytes and `write()` calls per frame come from the sink.
- **The gate.** A gated case fails `cargo xtask bench` when its native Callgrind instruction count exceeds the same case on `main` by more than 3%, or it allocates more per frame than Ratatui. Both wall-clock scopes remain reported, not gated. Instruction collection measures 32 complete frames after 200 warm-up frames, excluding setup and reporting. See [verification and negative controls](../docs/verify-widget-bench.md). Widgets that predate the gate are *tracked*: reported, not failing. A new widget is added with `gated: true`.

- **Effects case.** `effects` fades a widget-drawn panel in, holds and fades it out: the native side applies `lunatui_runtime::effect::Effect`, the ratatui side a small hand-written loop with the same `OKLab` blend (ratatui has no effect system; tachyonfx is not used). The native effect is faster over a whole frame, though its render-only scope is slower than the bare loop, as the report shows. See [docs/recording-effects.md](../docs/recording-effects.md).
- **Color policy.** Both entrypoints pin color output on (`pin_color_output`), so ratatui's crossterm backend writes colors whatever `NO_COLOR` says and both sides encode truecolor. Byte counts do not depend on the environment.
- **Arguments.** `--gate-all` also gates the tracked cases, to see how far they are from the bar. `--case NAME` selects one registered case, and `--baseline-first` measures the Ratatui side first.

Run it with `cargo bench -p lunatui-bench --features text-area --bench widget_suite` (add `-- --gate` to enforce the allocation bar). `cargo xtask bench` also enforces instruction regressions on Linux with Valgrind installed. The standalone instruction checker is `python3 scripts/check-widget-instructions.py`; its `--max-regression N` sets the permitted percentage increase, default 3. `--max-ratio` no longer applies to wall-clock reports and is rejected. For Criterion timings of the `TextArea` case at both sizes, run `cargo bench -p lunatui-bench --features text-area --bench criterion_widgets -- text-area`.

The gated `checkbox` case renders six native `lunatui-choice::Checkbox` controls
and the same scene with pinned `tui-checkbox` 0.5.3. Its documented custom-symbol
and styling APIs match the native three-cell glyphs, mixed marker and semantic
colors. Both sides use the same labels, geometry, focus schedule and checked
states. The dark baseline theme is reused across frames. The baseline dependency
is confined to this unpublished benchmark crate.

Run `python3 scripts/bench-widget.py` for six paired rounds at both sizes with
alternating measurement order. It retains raw logs, medians and ranges, validates
the selected case and emitted bytes, and enforces the allocation
gate. The separate instruction check compares against a frozen `main` revision. These are complete scene render/frame measurements, including headings,
backgrounds, diff and encoding. Terminal-emulator rendering is outside the scope.
See [Checkbox recording](../docs/recording-checkbox.md) for the visual proof.


The gated `widget-list` case renders a mixed-height activity feed with native
`WidgetList` and the same scene with pinned `tui-widget-list` 0.15.3. Both sides use
the same items, heights, colors and scripted selection. The baseline dependency is
confined to this unpublished benchmark crate. Run
`python3 scripts/bench-widget.py --case widget-list` for six paired rounds at both
sizes. See [`WidgetList` recording](../docs/recording-widget-list.md) for the visual proof.


The `terminal` case includes identical VT parsing and screen allocation on both sides, followed by the native terminal widget or an independent Ratatui buffer adapter. Both use the same changing colored CLI bytes and sizes. It is an exact, gated case and participates in `criterion_widgets`. Both timing entrypoints abort with a nonzero exit code if any frame write fails. `scripts/check-terminal-mutants.py` injects a counting-sink failure to verify that the Criterion run rejects it.

The gated `node-graph` case compares the native graph with pinned `tui-nodes` 0.10.0.
Construction and automatic layout happen inside both timed render paths. Both use
the same port rows, labels, sizes and theme styles. Run
`python3 scripts/bench-widget.py --case node-graph` for six alternating rounds.
See the [recording guide](../docs/recording-node-graph.md) for scope and visual proof.
