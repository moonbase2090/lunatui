# lunatui-widgets

Widgets for lunatui (PRD §8). **v0.1.0** includes the Phase 4 set used by `examples/gallery.rs`.

## API

- [`Widget`](src/widget.rs): `render(&self, area, &mut RenderCx)` and `measure(&self, avail: Size) -> Size`.
- [`RenderCx`](src/render_cx.rs): grid, cluster table, and theme for token-aware drawing. Optional `color-profiles` feature adds `profile` for ANSI16/256 downsampling in `token`.
- `WidgetState` (normal, focused, disabled) sits behind the off-by-default `widget-state` feature. `calendar` and `sparkline` forward it. `line-gauge` reserves the same feature.
- [`Size`](https://docs.rs/lunatui-core/latest/lunatui_core/struct.Size.html) lives in `lunatui-core` and is re-exported here.

## Widgets (v0.1.0)

`Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `Scrollbar`, `Spinner`, `StatusBar`, `TextInput`, `Overlay`, `StatusDot`, and `Lightcycle`.

`Monthly` is behind the `calendar` feature. It draws one month in the same 21-column Sunday grid as ratatui, and can color the selected day from theme tokens.

`Sparkline` is behind the `sparkline` feature. It draws one column per sample with the eight block steps, matches ratatui when given an explicit style, and otherwise paints from theme tokens for the current `WidgetState`.

`Lightcycle::new(elapsed)` draws an eight-way Braille sub-cell trail for the supplied elapsed time. It does not own a clock; the application supplies each frame's time and schedules redraws.

Snapshot tests use `lunatui_test::styled_snapshot`. See [Getting started](../docs/getting-started.md).
