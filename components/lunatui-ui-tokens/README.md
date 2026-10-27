# lunatui-ui-tokens

Semantic tokens shared by the `lunatui-ui` component crates (card, badge, button and the rest).

- `UiToken` names the roles a component paints with: `Background`, `Foreground`, `Card`, `Popover`, `Primary`, `Muted`, `Accent`, `Destructive`, `Border` and `Ring`.
- `Tone` picks a component's color family: `Neutral`, `Primary`, `Success`, `Warning` or `Destructive`.
- `UiToken::color` resolves a role through `RenderCx`, so color profile downsampling stays canonical.

Each role maps onto a lunatui theme `Token`. With the `graphite-tokens` feature the roles use the dedicated Graphite tokens (`Border`, `TextMuted`, `SurfaceRaised`). Without it they fall back to the eight core tokens.
