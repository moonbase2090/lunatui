# Porting from ratatui (v0.1.0 preview)

lunatui keeps ratatui’s mental model—widgets render into a rectangle on a cell grid—but the storage and pipeline differ. **v0.1.0 is a preview:** there is no drop-in ratatui backend yet (`lunatui-ratatui` remains a stub). Use this map when starting new apps or when reading the examples.

## Concept mapping

| ratatui | lunatui v0.1.0 |
|---|---|
| `Buffer` / `Terminal` draw | `lunatui_core::Grid` + `lunatui_term::Encoder` or `Session` |
| `Rect` | `lunatui_core::Rect` |
| `Style`, `Color` | `lunatui_core::Style`, `Color`; theme via `lunatui_theme::Token` |
| `Widget::render(area, buf)` | `Widget::render(area, &mut RenderCx)` |
| `Layout::split` | `lunatui_layout` stub; closed-form prototype in `layout_prototype` example (ADR 0002) |
| `Paragraph`, `Block`, `List`, `Table` | `lunatui_widgets` equivalents (Phase 4) |
| Event loop / `crossterm::event` | `lunatui_term::Parser`, `lunatui_runtime` scheduler and `run()` (`app` feature) |

## Drawing a frame

1. Allocate `Grid::new(width, height)` and `ClusterTable::new()`.
2. Build `RenderCx::new(&mut grid, &mut clusters, &theme)`.
3. Call `widget.render(rect, &mut cx)` for each widget.
4. Encode with `Encoder::encode_diff` or present through `Session` on Unix.

See `examples/hello_box.rs` and `examples/gallery.rs`.

## Styled snapshots in tests

Use `lunatui_test::styled_snapshot(&grid, &clusters)` instead of comparing raw buffer strings. The format is documented in `lunatui-test/README.md`.

## Compatibility path

Later roadmap PRs add a ratatui `Backend` and widget bridge in `lunatui-ratatui` (PRD §4 L1/L2). Until then, port screens by reusing layout ideas and redrawing through `RenderCx`.

## Examples to compare

| ratatui baseline | lunatui |
|---|---|
| `ratatui_dashboard` | `live_dashboard` (dogfood scheduler) |
| N/A | `gallery` (widget showcase) |
| N/A | `demo` (`lunatui::run`, focus, motion) |

Run both dashboard-style examples in release when comparing frame times.
