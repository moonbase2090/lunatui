# lunatui-runtime

Runtime for lunatui. `Input` reads terminal bytes on a worker thread, decodes parser events, and coalesces available bursts into frames. `Scheduler` drives idle, dirty, and animating frame timing with backpressure (PR 22). `Clock`, `Ease`, `Spring`, and `Tween` provide frame-rate-independent animation primitives (PR 25). With the `app` feature, `App` and `run` wire input, scheduler, grid diff, and session present for interactive examples (PR 23). With the `app` feature, `HitMap` and `FocusManager` support pointer hit-testing and keyboard focus (PR 24).

The default `image-presenter` feature adds `App::render_frame` and a runtime-owned Kitty, iTerm2 inline PNG, or Sixel image layer. Existing `App::render` implementations continue to work. An app passes `RenderFrame::parts().2` to `RenderCx::image_sink` during `render_frame`. Image widgets keep a stable nonzero ID and a generation; `PixelPieChart` manages its own raster revisions. Runtime probes once and selects only positively reported support. Unsupported terminals and multiplexers use each widget's cell fallback. `RunOptions::pixels(false)` skips graphics probing and selects the fallbacks explicitly. Disable default features to omit the runtime presenter. Prismattyc-native presentation remains separate.

The optional `effects` feature (it implies `motion`) adds `effect::Effect`: fade, dissolve, sweep, slide and HSL shift, with delay, duration and `Ease` on each, joined by `Effect::sequence` and `Effect::parallel`. Draw the widgets into the `Grid` as usual, then call `Effect::render(elapsed, &mut grid, area, &palette)`. An effect is a pure function of elapsed time, so a frame at a given time is reproducible and testable. `Effect::reversed` runs an effect backward (fade-out to fade-in) and `Effect::with_policy(MotionPolicy)` jumps to the end state under reduced motion. A `Palette` built from the active theme resolves default colors, palette indexes and theme tokens to RGB, so effects use theme colors and not hard-coded ones. Colors blend in `OKLab` and come out as RGB, which the encoder reduces to the terminal's 256 or 16 colors. This is cell-level animation; sub-cell precision and real alpha belong to the render core and are not part of it.

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
