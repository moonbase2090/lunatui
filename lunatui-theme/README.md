# lunatui-theme

Theme tokens, color profiles and serde themes for lunatui.

PR 13 adds `lunatui_theme::Profile` behind the off-by-default `color-profiles` feature: truecolor, 256, 16, and mono. `Profile::downsample` picks the nearest xterm palette color in `OKLab`. Enable with `lunatui-theme/color-profiles` or `lunatui/color-profiles`.

PR 26 adds `Token`, `Theme`, and JSON/TOML loaders. Cells keep `Color::Token` ids; the encoder resolves them at present time so a theme toggle re-encodes without re-rendering.

The crate ships **29** bundled themes ported from Prismattyc host palettes (`themes/*.toml`; Omarchy variants are omitted from this crate). Load with `load_bundled("prismattyc-dark")` or parse your own JSON/TOML via `theme_from_json` / `theme_from_toml` (strict variants require the eight base file tokens). With `color-profiles`, `Theme::palette_for` downsamples tokens to 256-, 16-, and mono-friendly indexed colors. Third-party palette licenses are listed in `THIRD-PARTY-NOTICES.md`.

## Graphite tokens

The off-default `graphite-tokens` feature makes the built-in light/dark palettes
and bundled `prismattyc`, `prismattyc-dark`, and `prismattyc-light` themes use
Graphite colors. The first eight token ids keep their meanings. Twelve additional
tokens cover borders, muted/strong/idle text, focus/hover surfaces, danger text,
and the motion head. `Token::all()` returns all 20 when this feature is enabled.
Cargo unifies dependency features, so enabling `lunatui-theme/graphite-tokens`
through any dependency enables the extra token variants for the whole build.
`Token::COUNT` is 8 without the feature and 20 with it; do not use it as a
persistent format size or assume it is fixed across feature sets. `Token` is
non-exhaustive, so downstream matches need a fallback arm.

Enabling the feature also changes the colors returned by `Theme::builtin` and
the three bundled `prismattyc` themes. Applications that depend on their prior
palette should keep this feature disabled or provide explicit token overrides.

Colors come from Graphite commit `337ab5d8a33bf1a968291d4a858dc89c9905df8c`.
The reference focus blue is `#62A8FF`; the light accent is Graphite's contrast-adjusted
`#4F89D1`. Applications can override the accent and other tokens at startup.
Custom theme files inherit unspecified extension tokens from the built-in variant.
Strict loaders require the eight base file tokens; extension tokens are optional.

With `color-profiles`, `Theme::palette_for(Profile::Ansi16)` uses semantic ANSI
indices, including index 4 for accent/focus, 3 for warning and 1 for danger.
RGB overrides do not change those semantic indices. Truecolor preserves overrides;
256-color mode precomputes nearest `OKLab` colors at palette preparation time.

```sh
cargo test --locked -p lunatui-theme --features graphite-tokens,color-profiles
python3 scripts/generate-graphite-tokens.py path/to/graphite --check
```

The generator reads the pinned Git commit rather than working files. ANSI-16
mappings follow `docs/DESIGN_STANDARD.md`; they are Lunatui policy. This feature
does not implement host `ThemeSource` derivation or terminal capability probing.

`Theme::prepare(profile)` returns a fixed-size `PreparedPalette`. Prepare it once
at startup and after a theme/profile change, then pass a borrow to
`RenderCx::palette`. Token resolution during widget rendering is an array lookup.
`PreparedPalette::colors()` supplies the same table to the terminal encoder.
The previous `RenderCx::profile(profile)` setter is replaced by preparing a
palette outside the render loop with `theme.prepare(profile)` and supplying it
to the context with `RenderCx::new(...).palette(&palette)`. Rebuild the palette
when the theme or terminal profile changes. The previous setter downsampled
token colors during each widget render; the prepared palette does that work
once, before rendering starts.
