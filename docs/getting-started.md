# Getting started (v0.1.0 preview)

lunatui **v0.1.0** is the first usable preview on the roadmap: core grid and encoder, terminal session and input, theme tokens, runtime scheduler, and the Phase 4 widget set. APIs will still change before 1.0.

## Workspace crates

| Crate | Role |
|---|---|
| `lunatui-core` | 16-byte cells, `Grid`, diff runs, width tables |
| `lunatui-term` | Encoder, capabilities, session, input parser |
| `lunatui-theme` | Semantic tokens and optional color profiles |
| `lunatui-layout` | `Layout::split`, closed-form solver (ADR 0002); kasuari fallback and `Grid`/`Stack` later |
| `lunatui-text` | Wrapping and row sources for lists and tables |
| `lunatui-widgets` | `Block`, `Paragraph`, `List`, `Table`, chrome widgets, `Overlay` |
| `lunatui-runtime` | Scheduler, focus, hit-testing, animation helpers |
| `lunatui-test` | Styled snapshots, virtual `Clock`, vt100 oracle |
| `lunatui` | Facade; enable `app` for `lunatui::run()` |

## Quick start

From the repository root:

```sh
cargo run --example hello_box
cargo run --example gallery -- --snapshot
cargo run --example demo --features app -- --counter --snapshot --width 80 --height 24
```

`hello_box` draws through `lunatui_widgets::Block` on a `lunatui_core::Grid`. `gallery` shows the Phase 4 widgets. `demo` uses the optional app runtime (`--features app`).

## Tests and snapshots

```sh
cargo test -p lunatui-widgets
cargo test -p lunatui-test
cargo test -p lunatui --example gallery
```

Widget tests use `lunatui_test::styled_snapshot` for readable frame diffs.

## Further reading

- [Architecture overview](architecture.md)
- [Performance vs ratatui](performance.md)
- [Porting notes (preview)](porting-from-ratatui.md)
- [Examples](../examples/README.md)
