# lunatui-dialog

Dialog and Alert Dialog for lunatui. A [`Dialog`] is a centered panel over a dimmed screen: a title,
a wrapped message, an optional text field, and Cancel and OK buttons on the bottom row. An
[`AlertDialog`] is the same panel for an action that needs an explicit yes, with a danger-colored
confirm button. Both wrap the core `Dialog`, draw a rounded border, and take colors from the theme.
The app keeps a [`DialogState`] for focus and the open flag, and uses `control_at` to hit test clicks.
