# lunatui-ratatui

ratatui compatibility for lunatui (PRD §4 L1).

## v0.1.0 / PR 43

- [`LunatuiBackend`](src/backend.rs): implements ratatui's [`Backend`](https://docs.rs/ratatui/latest/ratatui/backend/trait.Backend.html) trait. `draw` copies cells into a `lunatui_core::Grid`; `flush` runs `lunatui_term::Encoder::encode_diff` to the wrapped writer.
- Color and attribute mapping lives in `convert.rs` (shared rules with `lunatui-term` differential tests).

Widget bridge (`RatatuiWidget`) lands in PR 44.
