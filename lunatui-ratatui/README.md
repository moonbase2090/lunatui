# lunatui-ratatui

ratatui compatibility for lunatui (PRD §4 L1).

## v0.1.0 / PR 43

- [`LunatuiBackend`](src/backend.rs): implements ratatui's [`Backend`](https://docs.rs/ratatui/latest/ratatui/backend/trait.Backend.html) trait. `draw` copies cells into a `lunatui_core::Grid`; `flush` runs `lunatui_term::Encoder::encode_diff` to the wrapped writer.
- Color and attribute mapping lives in `convert.rs` (shared rules with `lunatui-term` differential tests).

## v0.1.0 / PR 44

- [`RatatuiWidget`](src/widget.rs): implements lunatui [`Widget`](https://docs.rs/lunatui-widgets/latest/lunatui_widgets/trait.Widget.html) for any ratatui [`Widget`](https://docs.rs/ratatui/latest/ratatui/widgets/trait.Widget.html) (clone per frame). Renders into a scratch buffer, maps cells into the lunatui grid (`CellDiffOption::Skip` preserved), and memoizes conversion when the scratch buffer is unchanged.
- [`RatatuiStatefulWidget`](src/widget.rs): same for ratatui [`StatefulWidget`](https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html) with owned state.
- Demo: `cargo run --example demo --features app -- --ratatui-widget --snapshot --width 80 --height 24` (`captures/demo_ratatui_widget.cast`).
