# lunatui

lunatui is a hyperspeed Rust TUI library being built from scratch to replace ratatui, keeping its simple "render widgets into a rectangle" model while fixing its structural costs with a 16-byte packed cell, damage-tracked diffing and a single-write output encoder, plus first-class input, focus, animation, capability and theme layers. It is open source under the [MIT](LICENSE) license.

**Status: v0.1.3 preview** — first usable release on the roadmap (Phase 4 widgets, encoder, runtime, tests). APIs and backends still evolve before 1.0. Start with [Getting started](docs/getting-started.md), the [architecture overview](docs/architecture.md), and the [PRD](docs/PRD.md).

## Quick start

```sh
git clone https://github.com/moonbase2090/lunatui
cd lunatui
cargo run --example live_dashboard
```

The dashboard draws 240 frames (about four seconds) and exits. Add `-- --bench` for a headless lunatui vs ratatui timing run. More examples are listed in [examples/README.md](examples/README.md).

See the [review policy](REVIEW_POLICY.md) for PR classification, proof and merge requirements.

## Performance vs ratatui (preview)

Same-machine, same-scope numbers (this Mac, October 2026). Full context and reproduction: [docs/performance.md](docs/performance.md). Do not mix older Xeon ratatui-perf figures with Mac lunatui times.

| Metric | ratatui v0.30.2 | lunatui |
|---|---|---|
| Cell size | 48 B | **16 B** |
| Diff, unchanged 200×60 | ~63 µs | **~3.8 µs** (packed `[u128]`, ADR 0001) |
| `write()` calls, ~68 KiB frame | 67 | **1** |
| Dashboard E2E 80×24 (median) | ~104 µs | ~1338 µs (`dashboard_parity_bench`; real painters) |
| Dashboard E2E 200×60 (median) | ~401 µs | ~1624 µs (same; lunatui slower — Paragraph log clone) |
| Allocs / dashboard frame 80×24 | ~513 | ~20376 |

Porting notes: [docs/porting-from-ratatui.md](docs/porting-from-ratatui.md). CI and manual sign-off: [docs/ci.md](docs/ci.md). Examples: [examples/README.md](examples/README.md).
