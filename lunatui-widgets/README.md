# lunatui-widgets

Widgets for lunatui (PRD §8). **v0.1.0** includes the Phase 4 set used by `examples/gallery.rs`.

## API

- [`Widget`](src/widget.rs): `render(&self, area, &mut RenderCx)` and `measure(&self, avail: Size) -> Size`.
- [`RenderCx`](src/render_cx.rs): grid, cluster table, and theme for token-aware drawing. Optional `color-profiles` feature adds `profile` for ANSI16/256 downsampling in `token`.
- `WidgetState` (normal, focused, disabled) sits behind the off-by-default `widget-state` feature. `barchart`, `bigtext`, `calendar`, `canvas`, `chart`, `markdown`, `sparkline`, and `radio`, `toast`, and `toggle` forward it. `line-gauge` reserves the same feature.
- `KeyHintBar` is behind the `key-hints` feature. Construct it from the keymap active for the current screen or mode; it renders the first chord registered for each command.
- [`Size`](https://docs.rs/lunatui-core/latest/lunatui_core/struct.Size.html) lives in `lunatui-core` and is re-exported here.

## Widgets (v0.1.0)

`Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `Scrollbar`, `Spinner`, `StatusBar`, `TextInput`, `Overlay`, `StatusDot`, and `Lightcycle`.

`KeyHintBar` is behind `key-hints` and draws the active keymap's shortcuts in registration order.

`BigText` is behind the `bigtext` feature. It draws large letters from an 8×8 bitmap, using full blocks (two columns per pixel), half blocks, quadrants, braille, sextants, or octants. Sextants pad one blank pixel row, and sextants and octants pack as quadrants until `subcell_glyphs` is set. It paints from theme tokens for the current `WidgetState` unless a style is set. Glyph bitmaps for digits, capitals, and `:.-%` are derived from Press Start 2P by `CodeMan38`, SIL Open Font License 1.1 (`assets/fonts/PressStart2P-OFL.txt`). Other printable ASCII is a hollow box. The opt-in `FiveBySeven` clock font draws the digits `0`–`9` and `:`; unsupported non-control characters render a blank 5×7 glyph with the usual two-column inter-glyph gap, while control characters are skipped.

`Monthly` is behind the `calendar` feature. It draws one month in the same 21-column Sunday grid as ratatui, and can color the selected day from theme tokens.

`Chart` is behind the `chart` feature. It plots `Dataset`s of `(x, y)` points as scatter, line, or bar series with braille, half-block, dot, or block markers, draws `Axis` lines with bounds, labels, and titles, and lists named datasets in a legend that is hidden when it would take too much of the plot. Without styles, series cycle through theme tokens and the axes follow the `WidgetState`.

`BarChart` is behind the `barchart` feature. It draws bars from `Bar` values, alone or in labeled `BarGroup`s, vertical or horizontal, with eighth-block ends, bar width, bar gap and group gap, labels, and value text. Without a style the bars take theme tokens for the current `WidgetState`.

`MarkdownView` is behind the `markdown` feature. It lays a parsed `lunatui_text::markdown::Document` out at the width it is given and draws headings, wrapped paragraphs with emphasis, code, and links, lists with task markers, block quotes, code blocks, tables, and rules. `scroll` moves it by rows and `content_height` sizes a scrollbar; every cell of the area is rewritten, and `background` keeps a panel color.

`Breadcrumbs` draws a one-row path. A path wider than the area collapses its middle to `…`, keeping the first segment and as many of the last as fit, and `segment_areas` gives each shown segment's cells for `HitMap` hit testing.

`SplitTree` holds nested splits: each divides its area between two panes or further splits at a ratio (in thousandths) with optional minimum sizes, and one pane can be zoomed to fill everything. `layout` returns each pane's cells and each divider's one-cell line for drawing and hit testing, `SplitDrag` turns pointer press, drag, and release on a divider into cell-exact resizing that stops at the minimums (and can cancel), and `SplitLayout::render_dividers` draws the lines.

`Canvas` is behind the `canvas` feature. It is a free-form drawing surface: a paint closure draws `canvas::Line`, `Rectangle`, `Circle`, and `Points` shapes (or its own `Shape`) in coordinates of your choosing, starts new layers with `Context::layer`, and prints labels with `Context::print`. Lines use braille's 2×4 sub-cell grid by default for fine detail; `Context::marker` switches individual shapes to another marker, such as half blocks or blocks for filled regions. Other markers are sextant, quadrant, and dot. Shapes are clipped to the bounds, every cell of the area is rewritten, and a `background` keeps a panel color.

`Toggle` is behind the `toggle` feature. It draws an on/off switch (`━━━●` on, `●───` off) with an optional label, a configurable track width, and the widget states: focused uses the focus-ring knob and a bold label, disabled dims it. `Toggle::hit_area` gives the cells a click should reach, so it plugs into `HitMap`.

`RadioGroup` is behind the `radio` feature. It draws a group of options of which at most one is selected (`◉` and `○`), one per row or side by side with a gap, with the focused option in the focus ring color with a bold label, per-option and whole-group disabling, and `option_areas` for hit testing. `RadioGroup::step` moves the selection with the keyboard, wrapping around and skipping disabled options.

`Tooltip` is behind the `tooltip` feature. It draws a short hint beside an anchor rectangle and never covers it: it opens on its side (below by default), flips to the opposite side and then to the other two when there is no room, slides along the side to stay inside the area, and is not drawn when no side fits. The text wraps at spaces to `max_width`, padding included, and the tooltip is as wide as its widest row; rendering does not allocate. `Tooltip::area` gives the covered cells for `HitMap`. `TooltipState` shows it after the pointer rests on the anchor for a delay (half a second by default), hides it when the pointer leaves, and `dismiss` hides it until the pointer comes back, with time passed in by the caller.

`ToastStack` is behind the `toast` feature. It holds timed messages and takes the time from the caller (a `Duration`, such as a virtual clock's `now`), so expiry is deterministic. A toast is scoped to a pane (a boxed message in the pane's bottom-right corner, newest nearest the corner) or to the status rows (one line each, right aligned). `placements` returns each shown toast's cells and layer for `HitMap`, and `render` draws them with theme colors for the kind.

`Sparkline` is behind the `sparkline` feature. It draws one column per sample with the eight block steps, matches ratatui when given an explicit style, and otherwise paints from theme tokens for the current `WidgetState`.

`SpinnerPreset` is a catalog of named cycles on the existing `Spinner`: dots, line, arc, bounce, pulse, moon, ring, bars, quadrant, and half-moon. Each preset has its own frames and the interval between them. `Spinner::preset` selects one. The default spinner still draws Braille dots, and the caller still supplies the frame index.

`Image`, `ImageSource`, and `ImageFit` are behind the `image` feature. `ImageSource::rgba8` accepts validated immutable RGBA8 data; `ImageSource::decode` is also available with `image-decode`, for PNG, JPEG, GIF, and WebP. The widget draws a half-block fallback with alpha composited over the `surface.0` theme token. A focused image gets a rounded `focus.ring` border; a disabled image is dimmed over the surface. `ImageFit::Contain`, `Cover`, `Stretch`, and `Native` share pixel-space geometry from `lunatui-image`. To use the off-by-default runtime Kitty presenter, enable `image-presenter`, override `App::render_frame`, and pass its image sink to `RenderCx::image_sink`. Give each image a stable nonzero `ImageId`; increment its generation whenever source pixels change. Other protocols and Prismattyc-native presentation are not implemented yet, so the runtime uses the half-block fallback unless Kitty support was queried successfully.

`Lightcycle::new(elapsed)` draws an eight-way Braille sub-cell trail for the supplied elapsed time. It does not own a clock; the application supplies each frame's time and schedules redraws.

Snapshot tests use `lunatui_test::styled_snapshot`. See [Getting started](../docs/getting-started.md).
