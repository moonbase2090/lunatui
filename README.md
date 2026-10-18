# lunatui

lunatui is a hyperspeed Rust TUI library being built from scratch to replace ratatui, keeping its simple "render widgets into a rectangle" model while fixing its structural costs with a 16-byte packed cell, damage-tracked diffing and a single-write output encoder, plus first-class input, focus, animation, capability and theme layers. It is open source under the [MIT](LICENSE) license.

**Status: v0.1.1 preview** — first usable release on the roadmap (Phase 4 widgets, encoder, runtime, tests). APIs and backends still evolve before 1.0. Start with [Getting started](docs/getting-started.md), the [architecture overview](docs/architecture.md), and the [PRD](docs/PRD.md).

See the [review policy](REVIEW_POLICY.md) for PR classification, proof and merge requirements.

## Performance vs ratatui (preview)

Ratatui numbers are the measured **v0.30.2** baselines in [PRD §1.3](docs/PRD.md). Lunatui numbers are from this repo’s bench harness (release, October 2026). Full context and reproduction commands: [docs/performance.md](docs/performance.md).

| Metric | ratatui v0.30.2 | lunatui v0.1.1 |
|---|---|---|
| Cell size | 48 B | **16 B** |
| Diff, unchanged 200×60 | 122 µs | **~4.6 µs** (packed prototype) |
| `write()` calls, ~68 KiB frame | 67 | **1** |
| Dashboard render 200×60 (CPU) | ~0.48 ms E2E baseline | **~215 µs** render-only on dev hardware |

Porting notes: [docs/porting-from-ratatui.md](docs/porting-from-ratatui.md). Examples: [examples/README.md](examples/README.md).
