# Register named controls

Use one `HitMap` per frame. Attach it to the rendering context and give each
interactive field a stable `HitId`. Labels become names; `.name()` provides a
name when the visual label is empty or when a more specific name is needed.

```rust,ignore
use lunatui_field::{Field, Form};
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{RenderCx, TextInput, Widget};

let input = TextInput::new(value, cursor);
let form = Form::new([Field::new("Email").body(&input)])
    .first_id(HitId(100));
let mut hits = HitMap::new();
let mut cx = RenderCx::new(grid, clusters, theme).hit_map(&mut hits);
form.render(area, &mut cx);
```

`Field::id` overrides the form's base ID plus field index. For a standalone
field, set its ID directly. Rendering through an attached map without an ID is
an error. Clear the map before the next frame to remove controls that disappear.
Body widgets supply their role and disabled state when available; use `.role()`
for a custom body and `.disabled(true)` to disable the whole field. `.layer()`
and `.z()` preserve overlay ownership and the layer stack's exact depth.

For a button, toggle, or another labeled widget, wrap the widget itself:

```rust,ignore
let control = NamedControl::new(id, AccessibilityRole::Button, &button);
control.render(area, &mut cx);
```

`NamedControl` uses the widget's visible control label and disabled state.
For an icon or an unlabeled input, add `.name("Search")`. A text input's value
never supplies its name. Empty or whitespace-only names are hard errors in all
build profiles, including plain rendering without an attached `HitMap`.
`Field::new` defers validation so `.name()` can follow it; `Form::new` validates
at construction, and standalone `Field` and `NamedControl` validate at render.
An explicit name overrides the visible label and must itself be nonblank.
The wrapper paints the same widget without additional chrome.

Read `hits.controls()` for borrowed records with `id`, `name`, `role`, `disabled`,
`area`, `z` and optional `layer`. Resolve role/name/layer selectors with
`find_control`; handle its ambiguity error rather than choosing a duplicate.
Disabled records remain queryable. Compare IDs with `FocusManager::focused()`
for focus; register only enabled controls in the focus ring.

`write_accessibility_json` exports version 2. It contains names and control
availability, with no input values, help text or validation messages. See
[ADR 0008](adr/0008-accessibility-role-export.md) for the schema and legacy
role-only annotations.

## Migrate existing forms

Import `Field`, `Form` and `FieldArea` from `lunatui-field`. The old
`lunatui-widgets::FormField` type and `Form` exports are removed. Replace
`FormField::new` with `Field::new`, `.hint(text)` with `.help(text)`, and
`.disabled()` with `.disabled(true)`. Add `lunatui-field` as a dependency.

`Form<'a>` now borrows any body widgets held by its fields. Use `Form<'static>`
for owned or static labels without borrowed bodies. `Field` remains cloneable
and accepts borrowed or owned label/message strings; it is no longer `Copy`
and `new`, `help` and `error` are no longer const functions. Construct fields at
runtime and clone them when reuse is needed. Empty `.error("")` clears the
optional error and restores help. `Form` remains
cloneable; its former structural equality derives do not apply to widget bodies.

Pass each input with `.body(&input)` to have the form render it. Existing callers
can still draw their input into `Form::layout` rectangles. The form owns label
columns, required markers, error/help rows and tab traversal. Input bodies are
constrained to `input_rows`, one by default; standalone stacked fields retain
the body's measured height.

## Verify the feature

```sh
cargo test --locked -p lunatui-runtime --no-default-features --features app --test controls --test accessibility
cargo test --locked -p lunatui-field --tests
python3 scripts/check-accessibility-roles.py
python3 scripts/check-named-controls.py
python3 scripts/record-named-controls.py
```

The tests render mixed real controls and drive pointer and Tab `Event`s. They
check disabled nonactivation, focus, scoped duplicate names, missing-name errors,
scroll/list clipping and exclusion of actual rendered values from exports.
There is no time-dependent behavior or sleeping in those tests. The recording
uses the example's injected virtual `Clock` to render sixty fixed frames, then
checks names, roles, disabled state, focus checkpoints and value exclusion in
the actual JSON snapshots. The GIF rasterizes those real widget bytes.

From a clean committed checkout, `check-named-controls.py` archives HEAD and
verifies that the real assertions reject eight deliberate regressions: dropped
registration, ambiguous first-match selection, disabled activation, lost scroll
translation, missing names, exported input data, and inputs outside a narrow form.
Logs remain in `target/named-controls-check`; temporary source directories are
removed. Repeat the drive in a clean VM for PR proof.
