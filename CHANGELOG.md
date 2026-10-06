# Changelog

All notable changes to lunatui are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every PR adds its entry under **Unreleased**.

## [Unreleased]

### Added

- PRD rules M14 (no comments: code is the memory) and M15 (mistakes become
  lints), and `docs/lints.md` to list the lint rules M15 adds.
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
