# lunatui-core

Core types for lunatui: cells, grid, styles, geometry, diff, grapheme slot and width tables.

It provides display widths (PRD §8 PR 04): `char_width` and `width` from Unicode 18.0.0 tables (`docs/width-policy.md`), grapheme slots and a per-frame `ClusterTable` (PR 05), the 16-byte packed `Cell` with `Color`, `Attrs` and `Style` (PR 06, ADR 0001), a `Grid` with write APIs and row dirty tracking (PR 07), and `diff_runs` to collect changed-cell runs between frames (PR 08). The `simd` module (roadmap PR 47+) holds runtime dispatch, scalar reference kernels, vectorized `first_mismatch` used by row diff (PR 48), and vectorized printable-ASCII width used by ASCII `text_display_width` (PR 49), each with a scalar fallback. Encoder types land in later roadmap PRs.

## Prepared rendering colors

The off-by-default `render-color` feature exposes shared `oklab` conversion and
`paint::{PreparedRamp, RampOutput, Opacity, blend_rgb}`. Core stays `no_std`; its
optional math implementation uses the already-pinned `libm` dependency.

Resolve semantic theme colors and the terminal profile once. Prepare a ramp when
its theme changes, then sample by column or a 16-bit phase during rendering:

```rust
# #[cfg(feature = "render-color")]
# fn prepared_example() -> Result<(), lunatui_core::paint::RampError> {
use lunatui_core::paint::{PreparedRamp, RampOutput};

let ramp = PreparedRamp::<256>::new([0, 0, 0], [255, 255, 255], RampOutput::TrueColor)?;
let midpoint = ramp.sample(32_768);
# Ok(())
# }
```

For reduced-color output, use `RampOutput::Solid` with the resolved semantic token
or ANSI index. It intentionally snaps to a solid color instead of converting a
gradient per cell. `blend_rgb` uses Graphite's foreground weight of 0 through 256
and integer truncation; `OKLab` interpolation is a separate operation.

Preview a truecolor ramp or a solid ANSI fallback with caller-provided colors:

```sh
cargo run -p lunatui-core --features render-color --example prepared_color -- 101216 62a8ff
cargo run -p lunatui-core --features render-color --example prepared_color -- 101216 62a8ff 4
```

The conversion follows [the OKLab reference](https://bottosson.github.io/posts/oklab/).
RGB endpoints and opacity are values supplied by the application, rather than a
new widget palette. Theme-wide Graphite token mapping and animation remain in
their owning layers.
