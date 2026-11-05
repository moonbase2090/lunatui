# lunatui-runtime

Runtime for lunatui. `Input` reads terminal bytes on a worker thread, decodes parser events, and coalesces available bursts into frames. `Scheduler` drives idle, dirty, and animating frame timing with backpressure (PR 22). `Clock`, `Ease`, `Spring`, and `Tween` provide frame-rate-independent animation primitives (PR 25). With the `app` feature, `App` and `run` wire input, scheduler, grid diff, and session present for interactive examples (PR 23). With the `app` feature, `HitMap` and `FocusManager` support pointer hit-testing and keyboard focus (PR 24).

The default `image-presenter` feature adds `App::render_frame` and a runtime-owned Kitty, iTerm2 inline PNG, or Sixel image layer. Existing `App::render` implementations continue to work. An app passes `RenderFrame::parts().2` to `RenderCx::image_sink` during `render_frame`. Image widgets keep a stable nonzero ID and a generation; `PixelPieChart` manages its own raster revisions. Runtime probes once and selects only positively reported support. Unsupported terminals and multiplexers use each widget's cell fallback. `RunOptions::pixels(false)` skips graphics probing and selects the fallbacks explicitly. Disable default features to omit the runtime presenter. Prismattyc-native presentation remains separate.

The optional `accessibility` feature implies `app` and adds experimental flat role
export. Register a semantic region with
`HitMap::push_with_role(id, area, z, AccessibilityRole::Button)` and call
`write_accessibility_json(writer, focus.focused())`. Each call writes one
version-1 JSON object and a newline, with annotated regions in registration
order, their IDs, roles, cell rectangles, depth and current focus. Ordinary
`push` regions are omitted. Clearing the hit map clears the metadata too.
Repeated IDs and empty rectangles are preserved; export does not infer
visibility or hierarchy. This exports role metadata, not a platform
accessibility tree. See [the format decision](../docs/adr/0008-accessibility-role-export.md).

Run the headless example and its JSON/focus check:

```sh
cargo run --locked -p lunatui-runtime --no-default-features --features accessibility --example accessibility_roles
python3 scripts/check-accessibility-roles.py
cargo test --locked -p lunatui-runtime --no-default-features --features accessibility --test accessibility
```
