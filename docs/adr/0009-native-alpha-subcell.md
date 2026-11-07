# ADR 0009: Real alpha and sub-cell placement in native mode

## Status

Accepted. Nothing here is active: the host has no native feature today (see
`docs/native-protocol.md`). This ADR fixes the data model so the app-side API and
the later protocol revision agree, and so ANSI output stays identical in layout.

## Problem

Widgets approximate translucency by blending a tint into the cells underneath
(`RenderCx::blend_backdrop`) and approximate smooth motion with eighth blocks and
octants. Inside a native host both can be exact: the host composites alpha on the
GPU and places glyphs at fractional cell offsets. The packed cell has no field for
either, and ring ABI 1.0 forbids unknown cell metadata.

## Options considered

| Option | Cost | Verdict |
|---|---|---|
| A. Widen the cell (alpha bytes in the 16-byte cell) | Cell no longer fits one `u128` compare; every diff slows | Rejected: breaks the ADR 0001 layout and the diff targets |
| B. Per-cell side attributes in the `ExtTable` | One table entry per translucent or offset cell; opaque cells unchanged | Chosen |
| C. A separate overlay layer of floating glyph sprites | A second scene graph, its own hit testing and damage rules | Rejected: duplicates the grid and breaks "layout is the cell grid" |
| D. Encode alpha in the color payload (RGBA colors) | Only 24 payload bits per color; no room, and it changes `Color` equality everywhere | Rejected |

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
   features are on by default.

## Memory layout and invariants

`ExtTable` is specified in ADR 0001 but not implemented yet; the first code change
under this ADR introduces it with the existing rules: 16-bit index, 0 means none,
deduplicated within a frame, equal data has equal indices, entries seeded across
frames so the diff stays an integer compare.

An entry's alpha and offset fields are four bytes: `fg_alpha`, `bg_alpha`, `dx`,
`dy` (each `i8` or `u8`). Invariants:

- `dx` and `dy` are in -7..=7; a larger request is clamped, never wrapped.
- An all-default entry (255, 255, 0, 0) is never stored; the cell keeps index 0.
- A continuation cell carries the same entry as its lead cell.
- Overwriting a cell replaces its entry; clearing a cell clears it.
- On table overflow the attributes are dropped for the overflowing cells for that
  frame and the frame stays correct, as ADR 0001 decided for other rare data.

## Benchmarks and targets

Baseline (`docs/performance.md`): dashboard frame 60 µs at 80x24 and 249 µs at
200x60, about 39 and 69 allocations per frame, and the 200x60 unchanged-grid diff
at about 4.6 µs. Targets:

- No gated benchmark more than 3% slower than `main` (PRD M2). Scenes that never
  set alpha or offsets must show no change in instruction counts or allocations.
- No allocation added on opaque paths; a translucent cell may add one table entry,
  amortized by deduplication.
- A scene with 10% translucent cells must stay within the 16 ms CI frame budget
  with the same margin as the dashboard.

The code PR adds a benchmark scene with translucent and offset cells before the
feature lands, and records its numbers in that PR.

## Migration and compatibility

- Cells with index 0 are byte-identical to today's, so existing grids, snapshots,
  the diff and the ANSI encoder do not change.
- `lunatui-ratatui` conversion ignores the new attributes in both directions.
- ABI 1.0 frames stay valid; hosts without the feature never see the attributes.
- Public API additions are additive; no existing signature changes.

## Amendment: what a translucent cell stores

Decision 3 said widgets write the existing approximation into the cell. For
backgrounds that cannot hold in both modes, because a blended color and a tint with
an alpha are different cell values. The rule is now: the app declares per frame
whether the host draws native attributes. With native off, `translucent_fill`
writes the blended background (the existing approximation) and no attributes. With
native on, it writes the tint color and `bg_alpha`, which the host composites over
its own backdrop. Glyph, width and the set of changed cells are identical in both
modes; only color values and attributes differ. Offsets follow the original rule:
the glyph is drawn in the nearest cell in both modes and native adds the remaining
eighths.

## Fallback rule (testable)

For any scene, rendering with the attributes stripped must produce the same glyph
widths, the same set of changed cells and the same wide-cell pairing as rendering
with them. Tests build every scene both ways and compare these.

## Test plan

- Property tests: setting then clearing attributes restores the original cell;
  equal attributes get equal indices within and across consecutive frames; clamping
  never wraps; continuation cells match their lead.
- The fallback rule above, on translucent overlays, a dialog backdrop and a
  sub-cell moving glyph, each rendered with and without attributes.
- Encoder transcripts for scenes using the attributes are byte-identical to the
  same scenes without them.
- Overflow: filling the table drops attributes and leaves glyphs and widths intact.
- Encode side (later change): fake-host tests for negotiated, not negotiated, and
  downgrade-on-failure paths, and for ABI 1.0 parsing of frames that omit the
  attributes.

## Open questions

1. Offset units: 1/8 cell is chosen to match eighth-block glyphs. Finer units can
   be added later through a minor ABI revision if a host needs them.
2. Whether foreground alpha should composite against the cell's own background or
   the blended result under it. Recommendation: the cell's own background after
   its alpha is applied, matching `blend_backdrop` order.
3. Table bound: 65,535 entries is far above the expected count; confirm with the
   benchmark scene.

## Out of scope

The host implementation, shared-memory activation and the PTY ownership rules,
which stay as specified in `docs/native-protocol.md`.
