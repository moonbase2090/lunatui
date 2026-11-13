# lunatui-runtime

Runtime for lunatui. `Input` reads terminal bytes on a worker thread, decodes parser events, and coalesces available bursts into frames. `Scheduler` drives idle, dirty, and animating frame timing with backpressure (PR 22). `Clock`, `Ease`, `Spring`, and `Tween` provide frame-rate-independent animation primitives (PR 25). With the `app` feature, `App` and `run` wire input, scheduler, grid diff, and session present for interactive examples (PR 23). With the `app` feature, `HitMap` and `FocusManager` support pointer hit-testing and keyboard focus (PR 24).

The default `image-presenter` feature adds `App::render_frame` and a runtime-owned Kitty, iTerm2 inline PNG, or Sixel image layer. Existing `App::render` implementations continue to work. An app passes `RenderFrame::parts().2` to `RenderCx::image_sink` during `render_frame`. Image widgets keep a stable nonzero ID and a generation; `PixelPieChart` manages its own raster revisions. Runtime probes once and selects only positively reported support. Unsupported terminals and multiplexers use each widget's cell fallback. `RunOptions::pixels(false)` skips graphics probing and selects the fallbacks explicitly. Disable default features to omit the runtime presenter. Prismattyc-native presentation remains separate.

The optional `effects` feature (it implies `motion`) adds `effect::Effect`: fade, dissolve, sweep, slide and HSL shift, with delay, duration and `Ease` on each, joined by `Effect::sequence` and `Effect::parallel`. Draw the widgets into the `Grid` as usual, then call `Effect::render(elapsed, &mut grid, area, &palette)`. An effect is a pure function of elapsed time, so a frame at a given time is reproducible and testable. `Effect::reversed` runs an effect backward (fade-out to fade-in) and `Effect::with_policy(MotionPolicy)` jumps to the end state under reduced motion. A `Palette` built from the active theme resolves default colors, palette indexes and theme tokens to RGB, so effects use theme colors and not hard-coded ones. Colors blend in `OKLab` and come out as RGB, which the encoder reduces to the terminal's 256 or 16 colors. This is cell-level animation; sub-cell precision and real alpha belong to the render core and are not part of it.

With `app`, `HitMap` records named controls with their role, disabled state,
exact depth and optional layer identity. `controls()` returns those records;
`find_control(role, name, layer)` reports ambiguous selectors instead of choosing
one. Disabled controls remain queryable but do not activate through pointer
picking. Register with `push_control(id, area, z, ControlMetadata::new(name, role))`
or render a `Field`, `Form`, or `NamedControl` through `RenderCx::hit_map`.

`write_accessibility_json(writer, focus.focused())` emits experimental version-2
JSON with control names and availability, excluding input values, help and
validation text. It is a flat snapshot, not a platform accessibility tree.
Raw geometric regions are omitted. Legacy role-only annotations retain a null
name and are not named selector targets. Clearing the map clears the metadata.
The `accessibility` feature remains a compatibility alias for `app`.
See [the format decision](../docs/adr/0008-accessibility-role-export.md).

Run the headless example and its JSON/focus check:

```sh
cargo run --locked -p lunatui-runtime --no-default-features --features app --example accessibility_roles
python3 scripts/check-accessibility-roles.py
cargo test --locked -p lunatui-runtime --no-default-features --features app --test accessibility
```
