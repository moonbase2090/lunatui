# Lunatui Design Standard v0.1
Principle: terminals don't have to be ugly. Browser-like smoothness, zero cost to frame time. Every visual rule below must be expressible as precomputed cells/styles; nothing here may allocate per frame.

## 1. Color (Graphite tokens)
Semantic tokens only; widgets never use raw colors.

| Token | Dark (truecolor) | Light (truecolor) | 256 | 16 |
|---|---|---|---|---|
| bg | #16181D | #F7F8FA | 234 / 255 | Black / White |
| surface | #1F2229 | #FFFFFF | 235 / 231 | Black / BrightWhite |
| surface-raised | #282C35 | #EEF0F3 | 236 / 254 | BrightBlack / White |
| border | #3A3F4B | #D5D9E0 | 238 / 252 | BrightBlack |
| border-strong | #5A6172 | #A9B0BC | 241 / 248 | White / BrightBlack |
| fg | #E6E8EC | #1B1E24 | 254 / 234 | White / Black |
| fg-muted | #9AA1AE | #5D6472 | 247 / 242 | BrightBlack |
| accent | #7AA2F7 | #3A66D1 | 111 / 26 | BrightBlue / Blue |
| accent-soft | #2A3A5E | #DCE6FB | 237 / 189 | Blue |
| success | #6BCB8B | #1F8A4C | 114 / 28 | Green |
| warning | #E8B85C | #A86A00 | 179 / 130 | Yellow |
| danger | #EF6F6C | #C0392B | 203 / 160 | Red |
| series-1..6 | #7AA2F7 #6BCB8B #E8B85C #C792EA #5FD3D3 #EF6F6C | darkened 25% | nearest | Blue Green Yellow Magenta Cyan Red |

- Detect capability once at startup (COLORTERM, terminfo); resolve tokens to a palette table then. No per-cell conversion.
- 256 fallback: nearest by OKLab distance, precomputed. 16 fallback: the mapping above, never "nearest".
- Gradients (gauges, sparklines) interpolate in OKLab at truecolor only; below that, snap to the solid token.
- Contrast: fg on bg ≥ 7:1, fg-muted ≥ 4.5:1, accent on surface ≥ 4.5:1, in both themes.

## 2. Spacing, borders, glyphs
- Spacing scale in cells: 0, 1, 2, 3, 4, 6. Horizontal padding default 1; vertical 0. Gaps between blocks: 1 row / 2 cols.
- Block titles: inset 1 cell from the corner, padded with one space each side.
- Borders, by preference: Rounded (`╭╮╰╯─│`) default; Plain for dense layouts; Thick only for focus; Double never. "None + surface fill" is a valid border and often looks best.
- Sub-cell detail order: octants (U+1CD00, when the font supports it; probe once) > braille (2×4) > half/quarter blocks (`▀▄▌▐`) > eighth blocks (`▁▂▃▄▅▆▇█`). Bars use eighths for smooth ends; lines use braille.
- Always keep an ASCII fallback set (`+-|#`) for `TERM=linux` and dumb terminals.

## 3. Motion
- Animation is opt-in per widget, driven by a frame clock; skip all frames when nothing is moving (no idle redraw).
- Durations: micro 80ms (hover/focus), standard 160ms (value change), emphasis 240ms (open/close). Never more than 300ms.
- Easing: ease-out cubic `(0.2, 0, 0, 1)` for entering/value; ease-in `(0.4, 0, 1, 1)` for exiting.
- Spring default: stiffness 300, damping 30, mass 1 (critically damped-ish, no visible overshoot). Settle when |Δ| < 1/8 cell.
- Target 60 fps, degrade to 30 if frame budget (16ms incl. diff+write) is missed twice in a row. Honor a reduced-motion flag (env `LUNATUI_REDUCED_MOTION`): snap instantly.
- Values animate at sub-cell resolution (eighth blocks), so motion looks continuous.

## 4. States
- Focus: border switches to accent + Thick, title in accent bold. Never rely on color alone: focused element also gets a `▌` left marker in lists.
- Hover (mouse terminals): background → surface-raised. No border change.
- Selection: background accent-soft, fg unchanged, bold off. Selected + focused: add the `▌` accent marker.
- Disabled: fg-muted, no hover. Error: danger border, message in danger below.
- 16-color mode: focus = Bold + reverse title; selection = reverse video.

## 5. Widget looks
**LineGauge** — 1 row. Filled track in accent (or status token by threshold: ≥80% warning, ≥95% danger), unfilled track `─` in border. Filled uses `━` with eighth-precision head via half-block cap. Label left, value right-aligned in fg-muted, tabular digits. Value changes animate (standard 160ms).

**Sparkline** — eighth blocks by default, braille mode for 2× horizontal density. Single series color, last point emphasized in accent with fg-strong value label optional. Baseline at min unless `zero_baseline`. No border by default.

**BarChart** — bars 1–3 cells wide with 1-cell gap; eighth-block tops for smooth heights. Value labels above bars in fg-muted; category labels below, truncated with `…`. Multi-series uses series-1..n, grouped not stacked by default. Hover/selected bar: accent-soft background column.

**Chart** — braille canvas (octants when available) for lines; points as `•`. Axes in border, tick labels in fg-muted, max 5 ticks per axis with "nice" numbers. Gridlines optional, dotted (`┄`) in border at 50% visual weight. Legend top-right inside the plot, swatch `■` + label. Series colors fixed order series-1..6.

## 6. Performance guardrails (non-negotiable vs ratatui)
- Styles resolve to an interned `StyleId`; cells store ids, not structs.
- Theme switch = swap one palette table; no widget rebuild.
- Damage-only diffing; animation only marks its own rect dirty.
- Every rule here gets a benchmark in CI; a visual feature that regresses frame time >5% doesn't ship.
