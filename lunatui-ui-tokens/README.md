# lunatui-ui-tokens

Shared semantic tokens for the `lunatui-<name>` UI component crates. Components ask for a
[`Role`] (background, card, primary, border, ring and so on) or a [`Tone`] (neutral, primary,
success, warning, danger) and this crate maps it onto a core theme [`Token`], so one theme file
restyles every component. It also holds the spacing scale and the border style.
