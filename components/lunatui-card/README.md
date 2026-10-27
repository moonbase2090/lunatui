# lunatui-card

A card: a bordered raised surface with a title, an optional description and a body widget.

```rust
use lunatui_card::Card;

let card = Card::new().title("Billing").description("Plan and invoices");
```

The card paints from `lunatui-ui-tokens`: the `Card` surface, a `Border` outline, `Foreground` title and `Muted` description. Override the outline with `Card::border_type`.
