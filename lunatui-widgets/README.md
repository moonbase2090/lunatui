# lunatui-widgets

Widgets for lunatui (PRD §8, roadmap PR 28).

## PR 28

- [`Widget`](src/widget.rs): `render(&self, area, &mut RenderCx)` and `measure(&self, avail: Size) -> Size`.
- [`RenderCx`](src/render_cx.rs): grid, cluster table, and theme for token-aware drawing. Optional `color-profiles` feature adds `with_profile` for ANSI16/256 downsampling in `token_color`.
- [`StatusDot`](src/status_dot.rs): sample widget from the PRD §3.2 sketch.

Later PRs add Block, Paragraph, List, and the rest of the widget set.
