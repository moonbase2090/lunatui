# lunatui

Facade for lunatui: re-exports, prelude and `run()`.

The `app` feature supplies `lunatui::run`, `App`, and `RunOptions`. It is enabled by the default `image-presenter` feature. See `examples/demo.rs` and ADR 0003.

The default `image-presenter` feature enables runtime pixel image output through Kitty graphics, iTerm2 inline images, or Sixel when the capability probe succeeds. `RunOptions::pixels(false)` explicitly selects cell fallbacks. Existing `App::render` implementations retain the cell fallback. See the `lunatui-runtime` and `lunatui-widgets` crate documentation for `App::render_frame` and the Image widget sink.
