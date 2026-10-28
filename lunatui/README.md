# lunatui

Facade for lunatui: re-exports, prelude and `run()`.

PR 23 adds the optional `app` feature: `lunatui::run`, the `App` trait, and `RunOptions` (off by default). Enable with `--features app`; see `examples/demo.rs` and ADR 0003.

The optional `image-presenter` feature enables runtime pixel image output through Kitty graphics when the capability probe succeeds. Existing `App::render` implementations retain the cell fallback. See the `lunatui-runtime` and `lunatui-widgets` crate documentation for `App::render_frame` and the Image widget sink.
