# Changelog

All notable changes to lunatui are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every PR adds its entry under **Unreleased**.

## [Unreleased]

### Added

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

- Frame budget: the CI hard gate is a 16 ms maximum on shared runners, and
  the 8 ms maximum is the target, measured on a dedicated machine and reported
  in every Proof section (#6).
- Crate-level docs moved from `//!` comments to per-crate `README.md` files
  included with `include_str!`. Comments other than the license header and
  `// SAFETY:` were removed (#6).
