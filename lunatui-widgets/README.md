# lunatui-widgets

Widgets for lunatui (PRD §8). **v0.1.0** includes the Phase 4 set used by `examples/gallery.rs`.

## API

- [`Widget`](src/widget.rs): `render(&self, area, &mut RenderCx)` and `measure(&self, avail: Size) -> Size`.
- [`RenderCx`](src/render_cx.rs): grid, cluster table, and theme for token-aware drawing. Optional `color-profiles` feature adds `profile` for ANSI16/256 downsampling in `token`.
- `WidgetState` (normal, focused, disabled) sits behind the off-by-default `widget-state` feature. `bigtext`, `calendar`, and `sparkline` forward it. `line-gauge` reserves the same feature.
- [`Size`](https://docs.rs/lunatui-core/latest/lunatui_core/struct.Size.html) lives in `lunatui-core` and is re-exported here.

## Widgets (v0.1.0)

`Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `Scrollbar`, `Spinner`, `StatusBar`, `TextInput`, `Overlay`, `StatusDot`, and `Lightcycle`.

`BigText` is behind the `bigtext` feature. It draws large letters from an 8×8 bitmap, using full blocks (two columns per pixel), half blocks, quadrants, braille, sextants, or octants. Sextants pad one blank pixel row, and sextants and octants pack as quadrants until `subcell_glyphs` is set. It paints from theme tokens for the current `WidgetState` unless a style is set. Glyph bitmaps for digits, capitals, and `:.-%` are derived from Press Start 2P by `CodeMan38`, SIL Open Font License 1.1 (`assets/fonts/PressStart2P-OFL.txt`). Other printable ASCII is a hollow box. The opt-in `FiveBySeven` clock font draws the digits `0`–`9` and `:`; unsupported non-control characters render a blank 5×7 glyph with the usual two-column inter-glyph gap, while control characters are skipped.

`Monthly` is behind the `calendar` feature. It draws one month in the same 21-column Sunday grid as ratatui, and can color the selected day from theme tokens.

`Sparkline` is behind the `sparkline` feature. It draws one column per sample with the eight block steps, matches ratatui when given an explicit style, and otherwise paints from theme tokens for the current `WidgetState`.

`SpinnerPreset` is a catalog of named cycles on the existing `Spinner`: dots, line, arc, bounce, pulse, moon, ring, bars, quadrant, and half-moon. Each preset has its own frames and the interval between them. `Spinner::preset` selects one. The default spinner still draws Braille dots, and the caller still supplies the frame index.

`Image` is behind the `image` feature. It accepts a validated immutable RGBA8 source and draws a half-block fallback using the source alpha composited over the `surface.0` theme token. A focused image gets a rounded `focus.ring` border; a disabled image is dimmed over the surface. `Fit::Contain`, `Cover`, `Stretch`, and `Native` share pixel-space geometry from `lunatui-image`. The runtime currently presents this cell fallback; terminal image protocol output is not wired yet.

`Lightcycle::new(elapsed)` draws an eight-way Braille sub-cell trail for the supplied elapsed time. It does not own a clock; the application supplies each frame's time and schedules redraws.

Snapshot tests use `lunatui_test::styled_snapshot`. See [Getting started](../docs/getting-started.md).
