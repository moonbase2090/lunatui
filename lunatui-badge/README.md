# `lunatui-badge`

Badge and Kbd for lunatui. A [`Badge`] is a one-line label on a tone fill (neutral, primary,
success, warning, danger), filled or subtle. A [`Kbd`] draws a key, or a chord, as raised chips.
Both take their colors from `lunatui-ui-tokens`.

## Graphite mode

In Graphite mode a [`Badge`] is a rounded pill with proportional text, and a subtle badge's pill
is a tint of its tone. A [`Marker`] before the text is a glyph in cells and a drawn shape in
Graphite mode. A [`Kbd`] key is a rounded keycap. Both cover the same cells as in Cell mode, and
when the frame cannot show pixels they draw exactly their Cell mode cells.

The Graphite form needs the `graphite-text` feature, on by default, which links the fonts bundled
in `lunatui-fonts` (about 5 MB). With `default-features = false` and without that feature, badges
and keys draw their Cell mode cells in every mode.

![Badges and key hints in Graphite mode, real output of the `graphite_chips` example](../docs/media/graphite-chips.gif)

The same frame in Cell mode (top) and Graphite mode (bottom):

![The `graphite_chips` example in Cell mode above Graphite mode](../docs/media/graphite-chips.png)
