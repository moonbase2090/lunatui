# ADR 0004: Widget trait and RenderCx (PR 28)

## Status

Accepted (2026-10-14). Implements PRD §3.2 / roadmap PR 28; follow-up to ADR 0003.

## Context

PR 28 adds `lunatui-widgets` with `Widget`, `RenderCx`, and `StatusDot`. checkpoint CP-3 reviewed the public API at `9254e4d` and required several decisions before Block, Paragraph, and List land.

## Decision

### `Size`

`Size` (width/height in cells) lives in **`lunatui-core`** next to `Rect`, because `lunatui-layout` will need it for Auto sizing without depending on `lunatui-widgets`. The widgets crate re-exports `lunatui_core::Size`.

### Token colors

- Canonical color lookup: `RenderCx::token(Token) -> Color`, applying optional profile downsampling when the `color-profiles` feature is enabled.
- Theme-file names: `RenderCx::token_named(&str) -> Option<Color>` (unknown names are `None`, not silent defaults).
- `RenderCx::theme() -> &'a Theme` shares the frame lifetime so widgets can hold the theme while drawing; widgets should still prefer `token` so profiles stay consistent.

### Clipping

`RenderCx::put_str` does **not** clip to the widget `Rect`. Widgets must guard empty areas and keep writes inside their allocated rectangle. Reference widgets (`StatusDot`) return early when `area.width == 0 || area.height == 0`. Area-scoped clipping in `RenderCx` is deferred until layout-owned child areas are common (List/Paragraph); if added later, it will be an explicit scoped helper, not a silent change to `put_str`.

### `put_str` argument order

`RenderCx::put_str(x, y, text, style)` matches the PRD sketch (text before style). `Grid::put_str` keeps `(x, y, style, text, clusters)` as the low-level buffer API.

### Extensibility

- `StatusState` is `#[non_exhaustive]` so states like `Warn` can be added without breaking downstream `match`es.
- `RenderCx` fields stay private; frame context such as time and hit registration will arrive via builder methods (not `new` signature changes), same pattern as `profile` under `color-profiles`.

### Facade

`lunatui` does not re-export widgets in PR 28. When it does, forward `color-profiles` to `lunatui-widgets/color-profiles` behind a default-off feature per ADR 0003.

### PRD sketch deviations

- `Idle` uses `Token::TextPrimary` because there is no `dim` semantic token yet.
- PRD `RenderCx` includes time and hits; this slice only carries grid, clusters, and theme.

## Consequences

- Layout and widgets share one `Size` type without a widgets → layout dependency cycle.
- Widget authors must respect `area` bounds until optional scoped clipping exists.

## Proof sketch

See compiled doc tests on [`Widget`](../../lunatui-widgets/src/widget.rs) and [`RenderCx::new`](../../lunatui-widgets/src/render_cx.rs).
