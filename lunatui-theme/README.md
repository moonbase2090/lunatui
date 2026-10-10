# lunatui-theme

Theme tokens, color profiles and serde themes for lunatui.

PR 13 adds `lunatui_theme::Profile` behind the off-by-default `color-profiles` feature: truecolor, 256, 16, and mono. `Profile::downsample` picks the nearest xterm palette color in `OKLab`. Enable with `lunatui-theme/color-profiles` or `lunatui/color-profiles`. Theme tokens and serde themes land in a later roadmap PR.
