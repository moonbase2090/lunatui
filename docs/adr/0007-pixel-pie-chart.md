# ADR 0007: Keep derived pie pixels in an owned widget

## Usage

Create `PixelPieChart::new(id, PieChart::new(slices).donut(0.5))`, then render it through the existing frame image sink. Replace the specification with `set_chart`; use `chart()` for layout and hit testing.

## Decision

The wrapper owns a derived raster cache keyed by the complete chart specification, resolved colors, measured size, cell dimensions, and existing backgrounds. It increments revisions internally. The existing PieChart geometry remains authoritative for sector rays, radius, hole, reveal, and highlight displacement. tiny-skia antialiased coverage masks blend shared sector edges before compositing over the actual panel.

The shared presenter owns capabilities and wire protocols. Its optional target observation reports physical dimensions, terminal background, and text-underlay support. Custom sinks default to no measured target. Unsupported paths render Braille in the same pass.

Kitty retains uploads and placements. iTerm2 and Sixel redraw current images after a complete cell repaint while previous or current images exist, so movement and removal erase old images. Those protocols reject negative depth; center and caption text therefore retain Braille. Multiplexers remain on cells until a passthrough path is verified.

## Synthesis decision

An owned wrapper was preferred over adding image identity and a caller-maintained revision to PieChart. Theme and background changes can occur outside chart setters, so caller revision tracking risks stale pictures. The wrapper preserves PieChart's existing value semantics and marker defaults. The alternative contributed coverage accumulation at shared edges and explicit resource bounds.

Pixel rendering and the runtime presenter are enabled by default by owner decision. Both the widget and run options expose `pixels(false)` for an explicit cell fallback. Existing Braille/text chart specifications remain available for mirrored views.

## Tradeoffs

The cache uses private interior mutability to implement `Widget::render(&self)`. App interaction state remains outside it. Immediate protocols retransmit images and repaint cells for correctness. Arbitrary text composition, terminal resizing, and multiplexer passthrough are separate work.

## Protocol evidence

The [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) supplies its existing query and retained-image operations. [Xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) supply DA parameter 4 and cell measurements. The [iTerm2 feature report](https://iterm2.com/feature-reporting/) assigns F to both focus and file support. The live iTerm2 implementation emits both, so inline selection requires its queried identity and two standalone F tokens; a single token stays unknown. Its [scaled cell-size report](https://iterm2.com/documentation-escape-codes.html) supplies physical Retina dimensions. Inline payloads follow the [image protocol](https://iterm2.com/documentation-images.html).
