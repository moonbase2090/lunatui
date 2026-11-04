# Host-provided symbol fonts

The default-off `symbol-font-fallback` feature extends the Graphite metrics
bridge in `lunatui-widgets`. It enables `graphite-render` and is included in
`all-widgets`.

`HostFontMetrics` supplies a font handle type, a primary font for each `Face`,
an ordered slice of symbol fonts, whole-grapheme coverage, and pixel advances.
The host owns font discovery, loading, shaping, and drawing. Coverage and width
must describe the same loaded fonts and shaping options used to draw text.
Widths must be finite and nonnegative.

Construct `HostTextMetrics::new(&host)` to borrow that font set. Its
`TextMetrics::width(face, px, text)` method measures the selected font runs.
Its `runs(face, text)` iterator exposes each selected handle and borrowed text
slice to the renderer. For example, given a host implementing `HostFontMetrics`:

```rust,ignore
use lunatui_widgets::graphite::{Face, HostTextMetrics, TextMetrics};

let metrics = HostTextMetrics::new(&host);
let width_px = metrics.width(Face::Regular, 14.0, "Build ♥");
for run in metrics.runs(Face::Regular, "Build ♥") {
    // Draw run.text with run.font at 14 px using the host's shaping options.
}
```

Selection preserves each extended grapheme cluster, including combining marks,
variation selectors, and joined emoji. The primary font wins whenever it covers
the complete cluster. Otherwise the first covering symbol font wins. If no
font covers the cluster, it stays with the primary font for missing-glyph
drawing and measurement. An empty fallback slice therefore uses only the
primary font.

Adjacent clusters using the same font form one run. The host measures the
complete run, so kerning such as `AV` is preserved within it. The adapter
allocates no text and loads no fonts. Keep the host font set stable between
measurement and drawing.

This contract applies to host-drawn pixel text. Terminal widgets continue to
emit terminal cells, whose fonts the terminal chooses. `CellTextMetrics`
continues to measure fixed terminal columns, independently of the requested
face and font size.

## Verify the contract

Run these commands from the repository root, also in a clean VM at the PR head:

```sh
cargo run --locked -p lunatui-widgets --no-default-features --features symbol-font-fallback --example symbol_fonts
cargo test --locked -p lunatui-widgets --no-default-features
cargo test --locked -p lunatui-widgets --no-default-features --features graphite-render --test graphite_render
cargo test --locked -p lunatui-widgets --no-default-features --features symbol-font-fallback --test symbol_font_fallback --test graphite_render
cargo test --locked -p lunatui-widgets --features all-widgets
cargo clippy --locked -p lunatui-widgets --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

The symbol-font tests assert selected handles, unchanged cluster slices, host
fallback order, run widths with kerning, face and pixel-size propagation,
missing-glyph advances, empty inputs, and a borrowed trait-object host.

The runnable `symbol_fonts` example uses deterministic host metrics and prints:

```text
Text: "Build "
Symbols: "☃"
Width at 12 px: 60
```
