# Porting from ratatui

lunatui keeps ratatui’s mental model—widgets render into a rectangle on a cell grid—but the storage and pipeline differ. **v0.1.0** ships `lunatui-ratatui` for incremental migration (PRD §4 L1/L2).

## Concept mapping

| ratatui | lunatui v0.1.0 |
|---|---|
| `Buffer` / `Terminal` draw | `lunatui_core::Grid` + `lunatui_term::Encoder` or `Session` |
| `Backend` + `Terminal::draw` | `lunatui_ratatui::LunatuiBackend` (L1) |
| `Widget::render(area, buf)` | `Widget::render(area, &mut RenderCx)` or `RatatuiWidget` (L2) |
| `Rect` | `lunatui_core::Rect` |
| `Style`, `Color` | `lunatui_core::Style`, `Color`; helpers `style_from_ratatui` / `style_to_ratatui` |
| `Span` / `Line` / `Text` | `StyledSpan` / `StyledLine` / `StyledText` with `From` both ways |
| `Layout::split` | `lunatui_layout::Layout::split` with `Constraint` (ADR 0002) |
| `Paragraph`, `Block`, `List`, `Table` | `lunatui_widgets` natives **or** ratatui widgets via `RatatuiWidget` |
| Event loop / `crossterm::event` | `lunatui_term::Parser`, `lunatui_runtime::run()` (`app` feature) |

## Migration levels (PRD §4)

### L1 — ratatui app, lunatui encoder

Keep ratatui widgets and `Terminal::draw`. Swap the backend for [`LunatuiBackend`](https://github.com/moonbase2090/lunatui/blob/main/lunatui-ratatui/src/backend.rs): cells accumulate in a lunatui `Grid` and `flush` runs `Encoder::encode_diff`.

- Demo: `cargo run --example demo --features app -- --ratatui-bridge --snapshot`
- Oracle tests: `lunatui-ratatui/tests/backend_oracle.rs`

### L2 — ratatui widgets inside lunatui apps

Wrap a ratatui widget with [`RatatuiWidget`](https://github.com/moonbase2090/lunatui/blob/main/lunatui-ratatui/src/widget.rs) or [`RatatuiStatefulWidget`](https://github.com/moonbase2090/lunatui/blob/main/lunatui-ratatui/src/widget.rs) for `List` and other stateful types. The bridge renders into a scratch `ratatui::Buffer`, converts cells into your `RenderCx`, respects `CellDiffOption::Skip`, and memoizes unchanged scratch buffers.

- Examples: `port_ratatui_block`, `port_ratatui_paragraph`, `port_ratatui_list` (see below)
- Demo: `cargo run --example demo --features app -- --porting --snapshot`

### Style and text (PR 45)

Use `color_from_ratatui` / `color_to_ratatui` and `style_from_ratatui` / `style_to_ratatui` for the indexed/RGB subset. Theme `Token` colors map to ratatui reset on export. Convert rich text with `StyledLine::from(rat_line)` and `Line::from(styled_line)`.

## Drawing a native lunatui frame

1. Allocate `Grid::new(width, height)` and `ClusterTable::new()`.
2. Build `RenderCx::new(&mut grid, &mut clusters, &theme)`.
3. Call `widget.render(rect, &mut cx)` for each widget.
4. Encode with `Encoder::encode_diff` or present through `Session` on Unix.

See `examples/hello_box.rs` and `examples/gallery.rs`.

## Ported ratatui examples

These three examples show L2 ports side by side with lunatui chrome. Each supports `--snapshot` for captures.

| Example | Command | What it shows |
|---|---|---|
| `port_ratatui_block` | `cargo run --example port_ratatui_block -- --snapshot` | lunatui `Block` + ratatui `Block` via `RatatuiWidget` |
| `port_ratatui_paragraph` | `cargo run --example port_ratatui_paragraph -- --snapshot` | Styled ratatui `Paragraph` inside a lunatui frame |
| `port_ratatui_list` | `cargo run --example port_ratatui_list -- --snapshot` | Stateful ratatui `List` via `RatatuiStatefulWidget` |

## Styled snapshots in tests

Use `lunatui_test::styled_snapshot(&grid, &clusters)` instead of comparing raw buffer strings. Ratatui parity for the bridge lives in `lunatui-ratatui` tests (`backend_oracle`, `widget_snapshots`, `style_roundtrip`).

## Baselines to compare

| ratatui baseline | lunatui |
|---|---|
| `ratatui_dashboard` | `live_dashboard` (native scheduler) or L1 bridge snapshot on `demo` |
| `gallery` | native widget showcase |
| `demo` | `lunatui::run`, focus, motion, ratatui bridge flags |

Run dashboard-style examples in release when comparing frame times.

## Lossy cases

- ratatui cells may hold multi-grapheme symbols; lunatui stores one grapheme per cell (ADR 0001 §7).
- Mouse/focus for ratatui widgets inside lunatui apps require manual mapping (PRD §4 L2).
- ratatui crates under MIT remain compatible; keep MIT notices if you adapt their source.
