# Changelog

All notable changes to lunatui are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every PR adds its entry under **Unreleased**.

## [Unreleased]

### Added

- `TabsDrag` turns pointer press, drag, and release over the regions from `Tabs::tab_areas` into a `TabMove` that reorders tabs: dropping on another tab moves the dragged tab there, a press that stays on its own tab is a click, and leaving the strip or `cancel` drops nothing. `TabMove::apply` reorders a `Vec` and `TabMove::selection` keeps the selection on the same tab.
- `KeyHintBar`, behind the `key-hints` feature, renders the first registered shortcut for each command in the active `Keymap`.
- `kitty_image` demonstrates a sharp 1920×1920 NASA Blue Marble image through Kitty graphics at native source resolution, with cell fallback elsewhere; the image is credited in `NOTICE`.
- `kitty_image` example runs the `Image` widget through Kitty graphics when available and uses its cell fallback in other terminals.
- `List::highlight_symbol`, `List::icons`, and `List::marks` add a selection marker, a styled icon before each item, and a right-aligned mark per row.

### Changed

- `List` clears every row of its area and paints the selected row's highlight across the full width.

## [0.1.13] - 2026-10-28

### Added

- `Breadcrumbs` widget: a one-row path of segments with a separator, styles for the segments, the last segment, the separators, and a hovered segment. A path wider than the area collapses its middle to `…`, keeping the first segment and as many of the last as fit. `Breadcrumbs::segment_areas` reports where each shown segment sits, for `HitMap`.
- `SplitTree` for resizable, nestable split panes with a zoomed pane: `split` and `close` change the tree, `layout` gives every pane's cells and every divider's line, `SplitDrag` resizes a divider cell by cell from pointer events (stopping at minimum sizes, with cancel), and `SplitLayout::render_dividers` draws the lines.
- `ToastStack` widget, behind the `toast` feature: timed toasts scoped to a pane (boxed, stacked up from the pane's bottom-right corner) or to the status rows (one line each), with `ToastKind` colors from theme tokens. The caller supplies the time, so expiry follows a virtual clock exactly; `placements` gives each shown toast's rectangle and layer so clicks can be routed through `HitMap`, with newer toasts above older ones and status toasts above pane toasts.
- `Canvas` widget, behind the `canvas` feature: a paint closure draws `Line`, `Rectangle`, `Circle`, and `Points` shapes or its own `Shape` through a `Painter`, in layers, with text labels, clipped to the `x_bounds` and `y_bounds`, and follows `WidgetState` and an optional background style. `Marker` gains `Quadrant` and `Sextant`, and `Chart` draws with them too.
- `Chart` widget, behind the `chart` feature, with `Dataset`, `Axis`, `GraphType` (scatter, line, bar), `Marker` (braille, half block, dot, block), and `LegendPosition`. Axes carry bounds, labels, and titles; a legend lists named datasets and is hidden when it would take more than half the plot, which `hidden_legend_constraints` changes. Series colors cycle through theme tokens and the axes follow `WidgetState`.
- `Tabs::tab_areas` and `List::row_areas` report where each visible tab or row sits in an area, and `HitMap::push_indexed` with `HitId::index_in` registers those regions as a group and maps a click back to the item index, so what is drawn is what a click reaches. Z-order follows the `HitMap`: a higher layer over the widget takes the click, and a later registration wins at the same layer.
- `Gauge::sub_cell` draws the fractional end of the fill as a left eighth block, and `Gauge::track_style` styles the unfilled cells.
- A `LineGauge` widget behind the off-by-default `line-gauge` feature: a one-row `━`/`─` progress line with filled and unfilled styles.
- `StatusBar::left` and `StatusBar::right` draw styled segments from each edge over a row filled with the bar style.
- `Spinner::glyph` returns the glyph a spinner draws at its frame, for use inside text such as a table cell.
- `Table` gains a header row, per-column styles and alignments, per-cell styles through the new `CellStyle` trait, a highlight symbol, an odd-row style, and a row gap.
- `BarChart` widget, behind the `barchart` feature, with `Bar`, `BarGroup`, and `BarDirection`. It draws vertical or horizontal bars with eighth-block ends, bar width, bar gap and group gap, labels, and value text on or beside the bar. Without a style, bars take theme tokens for the `WidgetState`.
- `MarkdownView` widget, behind the `markdown` feature: it draws a parsed Markdown `Document` wrapped to the area's width, with headings and rules, lists and task items, block quotes, code blocks, boxed tables, and styled inline text, scrolls by rows, reports its `content_height` for a scrollbar, and follows `WidgetState` and an optional background style.
- An `Image` widget, behind the opt-in `image` feature, with bounded PNG, JPEG, GIF, and WebP decoding under `image-decode`, a half-block fallback, and optional Kitty pixel presentation under `image-presenter`.

### Changed

- The `Table` highlight style now covers the selected row's full width, gaps included.
- `Table` clears every row of its area, including rows below the last data row, so a re-render leaves no stale rows.
- The MarkdownView README demo is recorded at 2x as a lossless animated WebP, and `scripts/record-showcase.sh` now records at 2x and writes WebP.

### Fixed

- `StatusBar` fills its whole row with the bar style before drawing, so shorter text or a text-only render no longer leaves the previous status behind.
- `lunatui-widgets` builds with no features enabled; `Table` styling no longer depends on a widget feature being on.
- A focused `Image` draws its focus ring over the background already in its area instead of the terminal default.

## [0.1.12] - 2026-10-27

### Added

- `Layout::limits` and `aspect_fit` in `lunatui-layout`, behind the opt-in `size-limits` feature: each segment can have a floor and a cap (`Limit::at_least`, `at_most`, `between`) so panes keep a usable size on resize, and cells a capped segment cannot take go to the other growing segments. `aspect_fit` returns the largest rectangle of a given shape inside an area, centred.
- `Layout::split_wrapped` in `lunatui-layout`, behind the opt-in `flex-wrap` feature: segments that do not fit move onto the next line (or column), each line is split with the layout's flex and spacing, and one `Constraint` sizes every line. `Layout::gap` sets the spacing between segments and between lines in one call.
- Named spinner presets on the existing `Spinner`: dots, line, arc, bounce, pulse, moon, ring, bars, quadrant, and half-moon. Each preset has its own frames and interval. `Spinner::preset` selects one. The default spinner is unchanged, and the caller still supplies the frame index. The example is a production deploy board.
- `lunatui_text::markdown`, behind the `markdown` feature: a Markdown parser that follows CommonMark 0.31.2 and adds GFM tables, strikethrough, task lists, and bare autolinks. It returns a syntax tree (`Document`, `Block`, `Inline`), does no rendering or I/O, and runs in time linear in the input size.
- `BigTextFont::FiveBySeven`, a compact 5×7 clock font for the digits `0`–`9` and `:`, drawn with full blocks.
- A CI guard for the site widget gallery: `scripts/check-gallery-real-output.sh` fails when a site capture example writes cells after a widget renders, and `lunatui/tests/site_gallery_repro.rs` re-renders each scene and compares it, and its committed PNG, with a recorded manifest.
- `CONTRIBUTING.md` and a pull request template carry the rule that demos show real widget output only.
- `Block::title_inset` keeps border cells between the left corner and a left-aligned title, and `Block::title_at` adds a title with its own position, alignment, and style.
- `Tabs::boxed` draws the selected tab in a rounded box that opens into a divider line, `Tabs::join_parent_border` joins that divider to a parent block's side borders, and `Tabs::indent` sets the space before the first tab.
- `TextInput::field_style`, `TextInput::padding`, and `TextInput::show_cursor` style the cells inside the field, inset the text, and hide the cursor of an unfocused field.
- `Monthly::weekend_style` styles Saturdays and Sundays under event styles, and `Monthly::selected_style` styles the selected day as a chip that covers the blank cell on each side.

### Changed

- The website gallery shows the full-screen Calendar, BigText, Sparkline, Spinner, and TextInput recordings. The README recordings for Calendar, BigText, and Sparkline are those same clips.

### Fixed

- The PTY integration tests for `lunatui` and `lunatui-test` carry their crate docs on Windows, where the tests are compiled out, so `missing_docs` no longer fails the Windows build.
- The website hero sky no longer drifts side to side; the sky image is static.
- `RenderCx::put_str` keeps text attributes such as bold, so widgets given a bold style now draw bold text.

## [0.1.11] - 2026-10-27

### Added

- The Lunatui website, a static page under `site/` with a widget gallery, benchmark results, and documentation links.
- Added twelve opt-in Graphite semantic tokens and semantic ANSI-16 palette mappings.

### Changed

- CI deploys the website to Cloudflare Pages and runs the Graphite token tests.
- The README names the porting guide "Migration guide".
- The calendar gallery walks from the branch cut into the next month and ends on the on-call drill. The agenda and the week list fill the screen. The clip opens on the schedule.
- Widget recordings pad the side-by-side frame so an odd height still encodes.
- `RenderCx::profile(profile)` is replaced by prepared palettes: create a palette with `theme.prepare(profile)` when the theme or profile changes, then pass it to `RenderCx::palette(&palette)`. This avoids downsampling tokens during widget rendering.
- The opt-in `graphite-tokens` feature changes the built-in light/dark palettes and the bundled `prismattyc` variants to canonical Graphite colors. Existing token ids 0–7 stay stable; twelve new ids are available when the feature is enabled. Cargo feature unification means enabling the feature through any dependency exposes the new variants across that build. `Token::COUNT` is therefore 8 or 20 according to the unified feature set, and should not be used as a persistent format size. `Token` is non-exhaustive; external matches must include a fallback arm.

## [0.1.10] - 2026-10-27

### Added

- `BigText` widget, behind the `bigtext` feature. An 8×8 bitmap is packed into full blocks (two columns per pixel), half blocks, quadrants, braille, sextants, or octants. Sextants pad one blank pixel row, and sextants and octants pack as quadrants until `subcell_glyphs` is set. An explicit style is painted as given. Without one, letters follow theme tokens for normal, focused, and disabled.

### Changed

- The big text gallery is a cutover board: a live clock, a countdown that reaches zero, ticking metrics, a moving selection, and an error count that turns red while it is hot.
- Big text digits, capitals, and `:.-%` use an 8×8 bitmap derived from Press Start 2P (SIL Open Font License 1.1, `lunatui-widgets/assets/fonts/PressStart2P-OFL.txt`). Other printable ASCII is a hollow box. Full blocks are two columns wide. Sextants pad one blank pixel row. The gallery draws those digits with the square-pixel sizes.

## [0.1.9] - 2026-10-26

### Added

- `Monthly` calendar, behind the `calendar` feature: a Sunday-based month grid with month and weekday headers, surrounding days, event styles, block padding, and theme-token styling for the selected day.
- `Sparkline` widget, behind the `sparkline` feature. Bar heights, direction, max, missing samples, and per-bar styles. Without a style, bars use theme tokens for normal, focused, and disabled, and the latest sample is emphasized. `Baseline::Minimum` lifts the floor to the smallest sample.
- `Grid::put_char` writes one width-1 character, including its text attributes, without grapheme segmentation.

### Changed

- The calendar gallery is a team schedule: a month beside the day's agenda, colored event days, a today highlight, and a selection that walks into the next month.
- The sparkline gallery is a live service board: labeled series, a moving selection, per-row value and min/max, and an error row that turns red over its threshold.

## [0.1.8] - 2026-10-26

### Added

- Add off-default subcell glyph lookup with generated octants, braille, block and ASCII fallbacks, plus eighth-block bar heads.
- Pixel layer in `lunatui-term`, behind the off-by-default `pixel-layer` feature: `PixelMode::from_caps` chooses kitty graphics only when the terminal answered the query and no multiplexer is present, and `PixelLayer` keeps RGBA images on screen with the least traffic (transmit on a new content generation, place again only on a move or depth change, delete when no longer listed). It allocates nothing in steady state; in `Cells` mode it emits nothing so callers draw a cell fallback.
- Optional allocation-free prepared color ramps, shared OKLab conversion and fixed-point compositing in the core, with a solid ANSI fallback.

## [0.1.7] - 2026-10-26

### Changed

- `Grid::fill` and `Grid::set_style` work a row of packed cells at a time and mark the row dirty once, instead of placing and marking every cell: about 30 times faster for `fill` and 13 to 18 times faster for `set_style` on a full 80x24 or 200x60 grid. Wide glyphs cut by an edge of the area keep the previous per-cell handling, and the result is identical.

### Added

- `scripts/record-showcase.sh` records a widget preview as an asciinema cast, a size-capped GIF, and a PNG screenshot for the README gallery.

## [0.1.6] - 2026-10-26

### Changed

- `Grid::put_str` writes printable ASCII straight into the cell words, with no grapheme segmentation or per-cell cluster work, when the target cells are plain one-column cells: about 17 to 19 times faster for a screenful of short strings. Text with other characters, and writes that touch wide glyphs, take the previous path and give the same result.
- README and `docs/performance.md` present lunatui's own dashboard numbers on one machine and one scope, with the method and limits.

### Added

- Motion core in `lunatui-runtime`, behind the off-by-default `motion` feature: the design standard's out-cubic and in-cubic easing curves, the 80, 160 and 240 ms durations with a 300 ms cap, a damped spring with explicit stiffness, damping and mass, an interruptible transition, a frame pacer that drops from 60 to 30 fps after two slow frames, the `LUNATUI_REDUCED_MOTION` policy, and `Scheduler::set_frame_period`.
- Widget recording script `scripts/record-widget.sh` and the `widget_preview` example: a widget case drawn at a fixed frame rate with a chosen size, frame count, fps, color profile and theme, recorded to an MP4 for pull requests and the README gallery.

## [0.1.5] - 2026-10-26

### Changed

- README and `docs/performance.md` compare lunatui and ratatui on the same machine with the same dashboard scene and scope (frame time, bytes, writes, allocations, RSS), replacing figures measured on different machines and over different scopes.
- The terminal encoder no longer allocates per frame once its buffers are warm: diff runs are collected into a reused buffer (new `diff_runs_into` and `collect_runs_in_row_simd_into` in `lunatui-core`) and SGR sequences are written straight into the output buffer. Output is byte-for-byte the same; a dashboard frame makes about half the allocations.

### Added

- Widget parity bench suite: each widget is compared with its ratatui equivalent on identical data at 80x24 and 200x60, in a render-only and a full-frame scope, with a cell-for-cell parity test and a gate in `cargo xtask bench` (frame ratio and allocations) for new widgets. The six existing widgets are tracked, not gated. `docs/DESIGN_STANDARD.md` records the visual and performance standard widgets follow. A shared `WidgetState` (normal, focused, disabled) is available behind the off-by-default `widget-state` feature of `lunatui-widgets`.

## [0.1.4] - 2026-10-25

### Changed

- Terminal encoder emits SGR as a delta: it no longer resets before every color change or re-sends both colors, and only resets when an attribute turns off. The rendered result is the same; a dashboard frame is 12-20% fewer bytes (#100).

### Added

- **Keymap registry:** a keymap registry with shortcut overlay and command palette renderers behind the `keymap` feature, with an example and PTY tests (#90).
- Fair Lunatui vs stock Ratatui dashboard comparison: separate release binaries with the same model and scripted ticks, a report runner (frame time, CPU, RSS, bytes, writes and allocations per frame), and a test that keeps the two scenes identical cell for cell. Run it with `scripts/run-dashboard-fair.sh` (#101).
- **Native protocol discovery:** `lunatui-prismattyc` parses capability 0.4 `native.grid.v1` advertisements behind an off-by-default `NativePolicy::DirectLocal` opt-in. Discovery never activates native ownership; ANSI remains the renderer, with a documented fallback repaint prefix (#94, #96).

### Fixed

- Wrapping with `trim` no longer starts an empty row when a line's trailing whitespace overflows the margin, which shifted every following row by one and showed up as stray fragments in wrapped logs (#101).
- Paint path performance: grapheme width no longer allocates per character, wrapping scans only the rows a `Paragraph` shows and no longer rescans lines per row, ASCII skips Unicode segmentation, and `Grid::set_style` no longer builds a per-row set. Output is unchanged; the dashboard frame is about 15x faster at 80x24 (#99).
- Release notes exclude tag signature blocks, fall back to the selected tagged changelog when the annotation body is empty, and fail before build if no prose is available (#95).
- Release checkout preserves signed annotated tag objects for signature-block checks and release metadata (#92).
- Release publication can recover an existing signed tag through a manual main-branch workflow, with verified source pinning and existing-release protection (#93).

## [0.1.3] - 2026-10-23

### Added

- **In-process grid sink:** `lunatui-prismattyc` gets a public in-process backend that sends packed grids and damage runs to a host renderer without ANSI encoding. Dimension-only changes notify the host even when the cell diff is empty (#84).
- **Lightcycle widget:** `lunatui-widgets` gets a stateless `Lightcycle` widget that draws an eight-way trail through Braille's 2×4 sub-cells, sampled by caller-supplied elapsed time with a configurable cycle period (#85).

### Changed

- **Encoder fast path:** same-style printable ASCII runs skip the per-cell decode. The emitted bytes are unchanged; `Grid::row_bits` is added for the fast path (#76).
- **Release workflow:** a `v*` tag now publishes a GitHub release from hosted runners after the tag is verified and the release text is scanned (#82).
- **Review policy:** behavior-changing PRs must include proof of a build and run in a clean local VM (#86).
- **Public-readiness cleanup:** `REVIEW_POLICY.md` is rewritten as a generic policy, committed local tooling state is removed and git-ignored, and the README quick start uses the `live_dashboard` example (#83).

### Fixed

- Terminal sessions flush each presented frame before returning and propagate flush failures, so buffered output reaches the screen without waiting for another write or writer drop (#88).

## [0.1.2] - 2026-10-20

### Added

- **Paired live dashboard benchmarks:** the `live_dashboard` example gets a paired benchmark for side-by-side comparison, with a PTY test that covers the live dashboard and its comparison (#80).

### Changed

- **SIMD row diff:** `first_mismatch` is vectorized and drives the row-diff path, with scalar fallback and differential tests against the scalar kernel (#73).
- **SIMD printable-ASCII width:** a vectorized fast path for printable-ASCII width, selected at runtime and checked against the scalar kernel with property tests (#72).

### Fixed

- Dashboard benchmark and baseline rendering are corrected, so the paired numbers compare like with like (#80).

## [0.1.1] - 2026-10-17

### Added

- **Ratatui compatibility layer:** `lunatui-ratatui` backend bridge (#62), `RatatuiWidget` / `RatatuiStatefulWidget` bridges (#63), and style/text conversions (#64).
- **Porting guide** rewrite with runnable `port_ratatui_block`, `port_ratatui_paragraph`, and `port_ratatui_list` examples (#65).
- **SIMD dispatch groundwork** with scalar reference kernels; no behavior change in production paths (#66).
- **Diff height-growth** regression test (#67).
- **Benchmark baselines** for ASCII text width (#68) and encoder throughput (#70).
- **Pre-commit `rustfmt`** hook and install scripts (#69).

### Changed

- Vectorized row-diff and ASCII-width kernels (roadmap PRs 48–49) and encoder fast paths (PR 50) remain deferred pending multi-machine performance gates.

## [0.1.0] - 2026-10-16

First usable preview on the roadmap (PR 35): workspace crates at **0.1.0**, Phase 4 widgets, `lunatui-test` styled snapshots and oracle, docs (`getting-started`, `performance`, `porting-from-ratatui`), and README performance table vs ratatui v0.30.2.

### Fixed

- The `gradient` example command is `cargo run --example gradient --features color-profiles`. Each profile step is a swatch painted with SGR foreground and background (`38;2` / `48;2` for truecolor, `38;5` / `48;5` for indexed) instead of hex and palette numbers. `examples/captures/gradient.cast` records that output. The `color-profiles` feature stays off by default.

### Added

- `lunatui_test::styled_snapshot` and `Clock`; widget tests use the public test crate (PR 34).
- Phase 4 widgets: `Paragraph`, `List`, `Table`, chrome widgets, and `Overlay` (`examples/gallery`).
- `lunatui::run()` demo with focus, hit-testing, and motion (`examples/demo`, `app` feature).

- `lunatui_runtime::Input` reads terminal input on a worker thread and coalesces event bursts into frames (PR 21, stacked on #32).
- ADR 0002 (DD-2, `docs/adr/0002-layout-engine.md`): constraint semantics, the closed-form split, the kasuari boundary, ratatui parity, and the plan for memo, `Grid`, `Stack` and `Auto`. The prototype is `layout_prototype` in `lunatui-bench`; `lunatui-layout` stays a stub until PR 14.
- `lunatui_term::Parser` decodes legacy keys, kitty keyboard sequences, SGR and X10 mouse, bracketed paste and focus (PR 20). `fuzz_input_parser` checks that arbitrary bytes stay bounded and are fully consumed. The `keys` example prints a captured key, mouse, paste and focus stream.
- `lunatui_theme::Profile` (feature `color-profiles`, off by default): truecolor, 256, 16, and mono. `downsample` picks the nearest xterm color in `OKLab` (PR 13). The `gradient` example prints one hue ramp in all four profiles when the feature is enabled.
- `lunatui_term::Caps`: environment plus one query batch ending in primary device attributes, including DECRQM for modes 2026 and 2027. Mode 2027 is enabled when the terminal reports it reset. Otherwise `WidthPolicy` is Unicode or ambiguous-wide from the terminal name and locale (PR 12). The `caps` example prints that result in a box.
- `lunatui_test` vt100 oracle: `diff_matches` checks `oracle(old + encode(diff(old, new))) == new`, a differential harness under `lunatui-term/tests/differential/`, and `fuzz_encoder` (PR 11). `hello_box --oracle` prints the oracle result for the demo's two frames. The encoder now places an absolute cursor move on the first cell of every frame, because a later frame cannot assume the cursor is still at the origin.
- `lunatui_term::Session` (Unix): raw mode, alternate screen, synchronized-output wrapping, one `write_all` per frame, and restore on drop or panic (PR 10). `hello_box --session` draws the encoded demo fullscreen and restores the main screen on exit.
- `lunatui_term::Encoder` (v1): pen-state SGR deltas, CUP cursor moves, printing through on the same row, and a crossterm-compatible reset trailer (PR 09). `hello_box --encode` draws the demo through the encoder.
- `lunatui_core::diff_runs` and `ChangedRun`: scalar diff over dirty row spans into contiguous changed-cell runs (PR 08). `hello_box --diff` prints runs between two demo frames.
- Leaf/trunk PR review policy with proof, testing, feature-gating and merge requirements (`REVIEW_POLICY.md`).
- `lunatui_core::Grid` with `put_str`, `fill`, `set_style`, per-row dirty spans and an
  internal styled-snapshot serializer; `hello_box` renders through the grid (PR 07).
- `lunatui_core::Cell`, `Color`, `Attrs`, `Style` and packed `to_bits` / `from_bits`
  (PR 06, ADR 0001). The `cell_dump` example prints sample 128-bit words; `hello_box`
  accepts `--cells` to dump packed cells for its corner glyphs.
- `lunatui_core` grapheme slot types, per-frame `ClusterTable` with ID seeding,
  and cluster-aware `width` / `text_display_width` (PR 05). `hello_box` adds a
  line of ZWJ emoji, VS16 and a flag cluster.
- ADR 0001 (DD-1, `docs/adr/0001-cell-format.md`): the 16-byte cell format,
  with the cell prototype in `lunatui-bench`, the `criterion_cell_prototype`
  bench and the `cell_prototype` example as evidence.
- `lunatui_core::char_width` and `lunatui_core::width`: terminal display widths
  from tables generated from Unicode 18.0.0, with `docs/width-policy.md`
  listing every difference from `unicode-width`. `hello_box` now measures
  its lines with them (PR 04).
- `cargo xtask gen-width` generates the Unicode 18.0.0 width tables for
  `lunatui-core`, and `unicode-width` 0.2.2 is a dev-dependency of
  `lunatui-core` for the differential width tests. The `lunatui` facade's
  examples can use `lunatui-core` (PR 04a, #11).
- ratatui baseline scenes (dashboard, log, table, gradient) ported from
  ratatui-perf, with fidelity tests, ratatui rows in the `metrics` bench, a
  `criterion_ratatui` wall-clock bench, and the `ratatui_dashboard` example
  (PR 03).
- `ratatui` 0.30.2 (pinned) as a dependency of `lunatui-bench` and a
  dev-dependency of the `lunatui` facade (for examples), for the ratatui
  baseline scenes in PR 03. Its crates are on the cargo-deny allowlist, and
  Zlib joins the allowed licenses for `foldhash` (#7).
- typos skips `*.cast` capture recordings, which are raw terminal output (#7).
- `cargo xtask check` enforces PRD M14 and M16: the fixed license header, the
  comment ban with its approved exceptions, `///` only on public items, crate
  docs through `include_str!` of a README, the `#[expect]` inventory in
  `docs/lints.md`, and workspace lints in every member (#6).
- Restriction lints: no `#[allow]`, no undocumented `unsafe`, no `unwrap`,
  `expect`, `panic!`, `todo!` or `unimplemented!` (#6).
- cargo-deny dependency allowlist, CODEOWNERS for lint, CI and dependency
  configuration, and a Sign-off workflow (`cargo xtask signoff`) requiring the
  `mb2090-signoff` label (#6).

- PRD rules M14 (no comments: code is the memory), M15 (mistakes become
  lints) and M16 (locked down: conventional and standardized), with
  `docs/conventions.md` and `docs/lints.md`.
- Bench harness crate `lunatui-bench` in `benches/`: a counting allocator, a
  `write()` and byte counter, calibration workloads, criterion and gungraun
  benchmarks, and `cargo xtask bench` to run them (PR 02, #4).
- Frame-time ceiling gate: `cargo xtask bench` fails if any sample of any
  worst-case benchmark exceeds 8 ms (PR 02, #4).
- `cargo run --example hello_box -- --metrics` shows the `write()` calls,
  bytes and allocations spent drawing the box (PR 02, #4).
- Cargo workspace with the eleven crate stubs, CI on Linux, macOS and Windows,
  `cargo deny` and typos checks (PR 01, #3).
- `hello_box` example: a bordered box with text (PR 01, #3).
- PRD, architecture overview and README (#1, #2).

### Changed

- REVIEW_POLICY.md gains a Review exceptions section: docs-only and changelog-only PRs merge on green checks plus the release-text check, a SHIP carries over an unchanged-diff rebase or label change, leaf Windows fixes batch their runtime proof before release, and security, trunk, CI, release and public-API PRs keep a full cross-family review at the exact tip.
- Frame budget: the CI hard gate is a 16 ms maximum on shared runners, and
  the 8 ms maximum is the target, measured on a dedicated machine and reported
  in every Proof section (#6).
- Crate-level docs moved from `//!` comments to per-crate `README.md` files
  included with `include_str!`. Comments other than the license header and
  `// SAFETY:` were removed (#6).
