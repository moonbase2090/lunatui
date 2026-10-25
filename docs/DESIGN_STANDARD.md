# Lunatui Design Standard v0.4
Principle: terminals don't have to be ugly. Browser-like smoothness, zero cost to frame time. Every visual rule below must be expressible as precomputed cells/styles; nothing here may allocate per frame.

**Source of truth:** `graphite-core` (github.com/moonbase2090/graphite, `src/tokens.rs`, `src/derive.rs`, `src/color.rs`, `src/scale.rs`, `src/motion.rs`). Where this document and the repo disagree, the repo wins; open items are listed in §7 as proposals.

## 1. Color (Graphite tokens)
Semantic tokens only; widgets never use raw colors. Canonical names are graphite-core `Tokens` fields; the Lunatui alias is what widget docs use.

### 1.1 Defaults (`graphite_core::DARK` / `LIGHT`, used verbatim for the built-in `prismattyc`, `prismattyc-dark`, `prismattyc-light` themes)
256 = nearest xterm-256 index by OKLab distance, recomputed from these hex values (v0.3's column predated them).

| Lunatui alias | graphite token | Dark | Light | 256 (D / L) | 16 |
|---|---|---|---|---|---|
| bg | `ground` | #101216 | #E9ECF0 | 233 / 255 | Black / White |
| surface | `panel` | #181B21 | #FFFFFF | 234 / 231 | Black / BrightWhite |
| surface-raised (hover) | `title_hover` | #252A33 | #E2E6EB | 235 / 254 | BrightBlack / White |
| border | `hairline` | #262A32 | #D5D9DF | 235 / 253 | BrightBlack |
| border-title | `title_line` | #22262D | #E3E6EA | 235 / 254 | BrightBlack |
| hover-outline | `hover_outline` | #3D5F8F | #9DBBE8 | 24 / 146 | Blue |
| fg | `text` | #E6E9EE | #1F2329 | 254 / 235 | White / Black |
| fg-strong | `text_strong` | #F2F4F7 | #1F2329 | 255 / 235 | BrightWhite / Black |
| fg-muted | `muted` | #A3ABB8 | #5B6472 | 248 / 241 | BrightBlack |
| fg-idle | `idle` | #6B7380 | #8A929E | 243 / 246 | BrightBlack |
| focus-fill (accent-soft) | `title_focus` | #1C2330 | #EAF1FC | 235 / 255 | Blue |
| focus-line | `title_focus_line` | #2A3140 | #CDDCF3 | 236 / 189 | Blue |
| fg-muted-on-focus | `muted_focus` | #B4BCC9 | #4A525E | 250 / 239 | BrightBlack |
| success | `working` | #4CC98A | #1F8A55 | 42 / 29 | Green |
| warning | `unseen` | #F2B84B | #B87A0A | 214 / 136 | Yellow |
| danger | `attention` | #FF7A6B | #C2392B | 209 / 160 | Red |
| on-danger | `on_attention` | #1A0D0B | #FFFFFF | 232 / 231 | Black / White |
| motion-head | `cycle_head` | #D6E8FF | #163F80 | 189 / 24 | BrightWhite / Blue |

Chrome tokens (`bar`, `bar_line`, `divider`, `tab_*`, `field*`, `key*`, `status_*`, `chip_active`, `separator`, `unseen_text`) also exist in graphite-core; see `tokens.rs`. 256 values for rows not listed in the computation (border-title, fg-muted-on-focus, on-danger, motion-head) are best-effort nearest and should be confirmed by the §7 generator proposal.

### 1.2 Accent / focus color
graphite-core has **no fixed accent hex**. The focus tint is the theme's `ansi[4]` (blue). `color::accent(tokens, focus)` returns it unchanged if it reaches **3:1 against `bar`**, otherwise mixes it toward black (light bars) or white (dark bars) in steps of 24/256, up to 24 steps.
- With the reference blue `#62A8FF` (used in graphite tests): dark → `#62A8FF` (256: 75), light → `#4F89D1` (256: 68). These are the Lunatui mockup defaults.

### 1.3 Derivation from a host theme (`derive_tokens`)
Input `ThemeSource`: `chrome_bg`, `chrome_fg`, `default_bg`, `default_fg`, `tab_active_bg`, `pane_backdrop`, `pane_border`, `active_badge`, `unseen_badge`, `attention_badge`, `ansi[16]`. `toward(a, b, p)` = integer mix of a toward b by p% (`p*256/100` weight).
- `ground` = pane_backdrop; `panel` = `field` = default_bg; `hairline` = pane_border; `bar` = chrome_bg; `text` = chrome_fg.
- `text_strong` = dark: toward(fg, white, 30); light: fg. `muted` = toward(fg, bar, 30). `idle` = toward(fg, bar, 50). `tab_text` = toward(fg, bar, 25).
- `working` / `unseen` / `attention` = the three badges; `on_attention` = white or `#1A0D0B`, whichever contrasts more.
- `title_line` = toward(default_bg, default_fg, 8). `title_hover` = hover_rgb(default_bg → default_fg, 10%; ×0.8 in light; channel delta capped at 30).
- `title_focus` = toward(default_bg, ansi[4], 10); `title_focus_line` = toward(default_bg, ansi[4], 22); `hover_outline` = toward(default_bg, ansi[4], 40); `muted_focus` = toward(default_fg, default_bg, 22).
- `bar_line` = toward(bar, fg, 7 dark / 12 light); `divider` = toward(bar, fg, 12); `separator` = toward(bar, fg, 25); `field_line` 16; `key_line` 18; `status_bar` = dark: toward(bar, black, 38), light: toward(bar, fg, 5).
- `cycle_head` is never derived; it comes from the built-in variant.
- **Readability pass** (`keep_text_readable`): `muted` ≥ 4.5:1 on bar/tab_active/field; `tab_text` ≥ 4.5:1 on all bars; `text` and `text_strong` ≥ max(4.5, muted's and tab_text's ratios); `key_text`, `muted_focus`, `unseen_text`, `on_attention` ≥ 4.5:1 on their grounds. Fixes push toward white/black in 24/256 steps (≤ 24).
- Built-in themes whose chrome matches the brief are returned verbatim (`uses_brief`); others are derived. `bar_tokens` can override `bar`/`status_bar` with a `BarColor` preset, then re-runs the readability pass.

### 1.4 Capability fallbacks (Lunatui-side)
- Detect capability once at startup (COLORTERM, terminfo); resolve tokens to a palette table then. No per-cell conversion.
- 256: precomputed OKLab nearest (table above). 16: the mapping above, never "nearest"; in 16-color mode Lunatui emits ANSI indices so the host palette decides hues.
- Gradients interpolate in OKLab at truecolor only; below that, snap to the solid token.
- Theming: a `Theme` is a full token set or a `ThemeSource`; partial overrides inherit from DARK/LIGHT. Widgets reference tokens only.

## 2. Spacing, borders, glyphs
graphite-core (`scale.rs`) defines pixel-renderer geometry: `WINDOW_PAD` 8, `PANE_GAP` 8, `PANE_PAD` 12, `PANE_RADIUS` 8, `PANE_HEADER_H` 28, `TABS_BAR_H` 44, `RAIL_H` 30, `SIDE_GAP` 4, `SIDE_PAD` 8, `SIDEBAR_W` 256 (design px, scaled). Lunatui maps these to cells at 8 px/col, 16 px/row: window pad 1 col, pane gap 1 col / 1 row (graphite has one gap value; the v0.3 "2 cols" is now a proposal), pane pad 1 col (12 px rounds to 1), radius 8 → Rounded corners, header 28 px → 1 title row (proposal).
- Spacing scale in cells: 0, 1, 2, 3, 4, 6. Horizontal padding default 1; vertical 0. Gaps between blocks: 1 row / 2 cols.
- Block titles: inset 1 cell from the corner, padded with one space each side.
- Borders, by preference: Rounded (`╭╮╰╯─│`) default; Plain for dense layouts; Thick only for focus; Double never. "None + surface fill" is a valid border and often looks best.
- Sub-cell detail order: octants (U+1CD00, when the font supports it; probe once) > braille (2×4) > half/quarter blocks (`▀▄▌▐`) > eighth blocks (`▁▂▃▄▅▆▇█`). Bars use eighths for smooth ends; lines use braille.
- Always keep an ASCII fallback set (`+-|#`) for `TERM=linux` and dumb terminals.

## 3. Motion
graphite-core (`motion` feature) defines only `MotionCurve::{Linear (default), InOutCubic}` and `light_cycle_traced_samples(progress, len, curve)` for the light-cycle trace, drawn in `cycle_head`. Everything else in this section is Lunatui policy, not repo-defined (see §7).
- Animation is opt-in per widget, driven by a frame clock; skip all frames when nothing is moving (no idle redraw).
- Durations: micro 80ms (hover/focus), standard 160ms (value change), emphasis 240ms (open/close). Never more than 300ms.
- Easing: ease-out cubic `(0.2, 0, 0, 1)` for entering/value; ease-in `(0.4, 0, 1, 1)` for exiting.
- Spring default: stiffness 300, damping 30, mass 1 (critically damped-ish, no visible overshoot). Settle when |Δ| < 1/8 cell.
- Target 60 fps, degrade to 30 if frame budget (16ms incl. diff+write) is missed twice in a row. Honor a reduced-motion flag (env `LUNATUI_REDUCED_MOTION`): snap instantly.
- Values animate at sub-cell resolution (eighth blocks), so motion looks continuous.

## 4. States
graphite-core state tokens: hover = `title_hover` fill + `hover_outline`; focus = `title_focus` fill + `title_focus_line` line, muted text → `muted_focus`; pane outline = `hairline`, title rule = `title_line`. Below, "accent" means the §1.2 focus color and "accent-soft" means `title_focus`.
- Focus: border switches to accent + Thick, title in accent bold. Never rely on color alone: focused element also gets a `▌` left marker in lists.
- Hover (mouse terminals): background → `title_hover`; outline → `hover_outline` (repo). v0.3 said "no border change"; superseded.
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

## 7. Conflicts and proposals
**Conflicts (v0.3 vs repo; repo now wins):**
- v0.3 `accent` #5AA2FF / #2F6FD0 is not in graphite; focus comes from `ansi[4]` via `accent()` (3:1 vs bar, not v0.3's 4.5:1 vs surface).
- v0.3 `accent-soft` dark #2A3A55 ≠ `title_focus` #1C2330.
- v0.3 `border-strong` reused `hover_outline`; it is a hover outline, not a strong border.
- v0.3 contrast floors (fg ≥ 7:1, accent ≥ 4.5:1) are stricter than graphite's (4.5:1 text, 3:1 accent).
- v0.3 256 column was off by one or more steps for most tokens (fixed above).
- Hover: graphite changes the outline; v0.3 said no border change.
- Graphite motion's default curve is Linear / InOutCubic; v0.3 used ease-out cubic (0.2,0,0,1) and ease-in.
- The `graphite-tui` (Lunatui widgets) crate was not found in the repo's main branch; only graphite-core exists.

**Proposals (spec has, repo lacks):**
- Series palette series-1..6, plus a light variant and 256/16 mappings; suggest adding to `Tokens` or a `Series` table.
- 256 and 16-color fallback tables (a generator in graphite-core so they can't drift).
- Accent default constant (suggest `#62A8FF` as the brief `ansi[4]`).
- Motion durations 80/160/240ms, ≤300ms cap, ease-out/ease-in cubic-beziers, spring (300/30/1, settle < 1/8 cell), 60→30 fps degrade, `LUNATUI_REDUCED_MOTION`; add as `MotionCurve` variants + constants.
- Cell spacing scale (0,1,2,3,4,6), block-title inset, border styles (Rounded/Plain/Thick, no Double), sub-cell glyph order and ASCII fallback, 16-color focus/selection (bold/reverse), disabled and error states, and all §5 widget looks.
- Theme-switch 240ms crossfade and TOML theme files.
