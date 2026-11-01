# lunatui-field

Field for lunatui. A [`Field`] puts a bold label above any control widget and keeps a muted help
line under it. An error message replaces the help line in the destructive color, a required field
marks its label with `*`, a focused field draws its label in the ring color, and a disabled field
dims. The control is any [`Widget`](lunatui_widgets::Widget), so a text input, a toggle or a
button sit in a field alike. All colors come from `lunatui-ui-tokens`.
