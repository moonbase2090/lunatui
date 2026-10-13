# lunatui-theme

Theme tokens, color profiles and serde themes for lunatui.

PR 13 adds `lunatui_theme::Profile` behind the off-by-default `color-profiles` feature: truecolor, 256, 16, and mono. `Profile::downsample` picks the nearest xterm palette color in `OKLab`. Enable with `lunatui-theme/color-profiles` or `lunatui/color-profiles`.

PR 26 adds `Token`, `Theme`, and JSON/TOML loaders. Cells keep `Color::Token` ids; the encoder resolves them at present time so a theme toggle re-encodes without re-rendering.

The crate ships **29** bundled themes ported from Prismattyc host palettes (`themes/*.toml`; Omarchy variants are omitted from this crate). Load with `load_bundled("prismattyc-dark")` or parse your own JSON/TOML via `theme_from_json` / `theme_from_toml` (strict variants require every semantic token). With `color-profiles`, `Theme::palette_for` downsamples tokens to 256-, 16-, and mono-friendly indexed colors. Third-party palette licenses are listed in `THIRD-PARTY-NOTICES.md`.
