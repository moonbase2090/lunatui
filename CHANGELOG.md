# Changelog

All notable changes to lunatui are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every PR adds its entry under **Unreleased**.

## [Unreleased]

### Changed

- README and `docs/performance.md` present lunatui's own dashboard numbers on one machine and one scope; the public comparison with ratatui and the side-by-side media are removed.
## [0.1.5] - 2026-10-26

### Changed

- README and `docs/performance.md` compare lunatui and ratatui on the same machine with the same dashboard scene and scope (frame time, bytes, writes, allocations, RSS), replacing figures measured on different machines and over different scopes.
- The terminal encoder no longer allocates per frame once its buffers are warm: diff runs are collected into a reused buffer (new `diff_runs_into` and `collect_runs_in_row_simd_into` in `lunatui-core`) and SGR sequences are written straight into the output buffer. Output is byte-for-byte the same; a dashboard frame makes about half the allocations.

### Added

- Widget parity bench suite: each widget is compared with its ratatui equivalent on identical data at 80x24 and 200x60, in a render-only and a full-frame scope, with a cell-for-cell parity test and a gate in `cargo xtask bench` (frame ratio and allocations) for new widgets. The six existing widgets are tracked, not gated. `docs/DESIGN_STANDARD.md` records the visual and performance standard widgets follow. A shared `WidgetState` (normal, focused, disabled) is available behind the off-by-default `widget-state` feature of `lunatui-widgets`.

### Added

- Motion core in `lunatui-runtime`, behind the off-by-default `motion` feature: the design standard's `Ease::OutCubic` and `Ease::InCubic` curves, the 80/160/240 ms durations with a 300 ms cap, `DampedSpring` (stiffness, damping and mass; `standard` is 300/30/1 and settles within an eighth of a cell), interruptible `Transition`, `FramePacer` (60 to 30 fps after two slow frames), `MotionPolicy` for the `LUNATUI_REDUCED_MOTION` flag, and `Scheduler::set_frame_period`.

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
