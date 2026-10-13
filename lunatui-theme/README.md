# lunatui-theme

Theme tokens, color profiles and serde themes for lunatui.

PR 13 adds `lunatui_theme::Profile` behind the off-by-default `color-profiles` feature: truecolor, 256, 16, and mono. `Profile::downsample` picks the nearest xterm palette color in `OKLab`. Enable with `lunatui-theme/color-profiles` or `lunatui/color-profiles`.

PR 26 adds `Token`, `Theme`, and JSON/TOML loaders. Cells keep `Color::Token` ids; the encoder resolves them at present time so a theme toggle re-encodes without re-rendering.
