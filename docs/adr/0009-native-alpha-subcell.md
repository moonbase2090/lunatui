# ADR 0009: Real alpha and sub-cell placement in native mode

## Status

Proposed. Nothing here is active: the host has no native feature today (see
`docs/native-protocol.md`). This ADR fixes the data model so the app-side API and
the later protocol revision agree, and so ANSI output stays identical in layout.

## Problem

Widgets approximate translucency by blending a tint into the cells underneath
(`RenderCx::blend_backdrop`) and approximate smooth motion with eighth blocks and
octants. Inside a native host both can be exact: the host composites alpha on the
GPU and places glyphs at fractional cell offsets. The packed cell has no field for
either, and ring ABI 1.0 forbids unknown cell metadata.

## Decision

1. **Alpha is a per-cell side attribute, not a color change.** A cell may carry
   `fg_alpha` and `bg_alpha` (u8, 255 = opaque, the default). They live in the
   frame `ExtTable`, so the 16-byte cell and the 99% opaque case are unchanged.
2. **Sub-cell offset is a per-cell side attribute.** A glyph may carry `dx` and
   `dy` in 1/8 cell, each in -7..=7. It also lives in the `ExtTable`. The cell the
   glyph belongs to never changes, so hit testing, wrapping and damage stay in
   whole cells.
3. **ANSI never sees them.** The encoder ignores the new attributes. Widgets that
   use them also write the existing approximation into the cell (a blended
   background, an eighth-block glyph), so an ANSI frame and a native frame touch
   the same cells with the same widths. Native mode only refines what is drawn
   inside those cells.
4. **Protocol.** A later revision adds feature `native.alpha.v1` and ring ABI 1.1.
   A client may set the new attributes only after the host affirms the feature in
   ACTIVE. Without it the client strips them before encoding. ABI 1.0 frames are
   unchanged and a 1.1 host still accepts them.
5. **Defaults.** The widget API and the `native-alpha` and `native-subcell` crate
   features are on by default. With no native host they cost one `ExtTable` entry
   per translucent or offset cell and nothing otherwise.

## Fallback rule (testable)

For any scene, rendering with the attributes stripped must produce the same glyph
widths, the same set of changed cells and the same wide-cell pairing as rendering
with them. Tests build every scene both ways and compare these.

## Out of scope

The host implementation, shared-memory activation and the PTY ownership rules,
which stay as specified in `docs/native-protocol.md`.
