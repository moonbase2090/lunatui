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

## Let an agent act by name

`Agent` gives a program the same controls a user sees, without keystrokes. It is on by
default and needs nothing beyond the registration above.

```rust,ignore
let snapshot = Agent::snapshot(&hits, focus.focused());
let ticket = agent.act(
    &hits,
    Selector::new(AccessibilityRole::Checkbox, "Notifications"),
    Action::Toggle,
)?;

while let Some(request) = agent.take() {
    match request.id.index_in(first, count) {
        Some(0) => name.act(&request.action),
        _ => Outcome::Ignored,
    };
}
```

A snapshot lists, in registration order, each control's `name`, `role`, `value`, `checked`,
`states`, `actions` and layer `parent`. `act` finds one control by role, exact name and layer and
returns `ActError` when none, several, or a disabled one matches, or when the role does not take
that action. Otherwise it queues a `Request` with `source: Source::Agent` and returns a `Ticket`;
`Ticket::cancel` drops the request if the app has not taken it. The app takes requests with
`Agent::take` during its update step, applies each to the state behind `request.id` and redraws.
`Act` applies the common cases: `bool` toggles and `String` sets its value.

Use `Agent::query` when an agent needs a smaller view than the complete snapshot. A
`ControlQuery` starts with accessible-name text, a semantic kind, or an exact owning layer; add
the other filters with `with_text`, `with_kind`, and `in_layer`. Text is a case-insensitive
substring of the accessible name, filters combine with AND, and `None` selects the base layer.
An empty text filter matches every accessible name. Matches keep registration order. A
`ControlQueryResult` returns the first 100 controls and its `more` count reports the rest;
`write_json` includes both the bounded controls and that count.

```rust,ignore
let save_buttons = Agent::query(
    &hits,
    focus.focused(),
    ControlQuery::text("save").with_kind(AccessibilityRole::Button),
);
```

Tabs and lists register their items too. `Tabs::controls(ItemControls::new(first))` and
`List::controls(ItemControls::new(first))` give each visible tab (role `Tab`) or row (role
`ListItem`, named by its first cell) the id `first + index`, with the chosen item marked
`Selected`. Choose one with `Action::Select`, then subtract `first` from `request.id` to get the
index. A row with a blank name is left out, rows scrolled out of view are not registered, and
rows that share a name make `act` return `ActError::Ambiguous`. What is drawn does not change.

A menu registers its items the same way. `Menu::new(&state).controls(ItemControls::new(first))`
gives every visible item, the whole bar and the rows of each open panel, the role `MenuItem` and
a name from its label. The id is `first` plus the item's number in reading order, parents before
their children, over the whole tree, so it stays the same whichever panels are open; hand
`request.id - first` to `MenuState::path_of` for the item's path. `Action::Select` calls
`MenuState::highlight_with`, which highlights the item and shows the panels that lead to it.
`Action::Activate` calls `MenuState::activate_with`, which does what Enter does on that item: a
group opens and a leaf queues its `MenuEvent::Selected`, with source `Agent`. A disabled item, or
an item in a disabled group, is a disabled control and `act` refuses it. A panel that is not open
has no controls, so open it first with `Select`; the same name in two open panels is ambiguous.
Only what is drawn is a control: a label clipped away by a narrow bar or panel, or mostly covered by a
nested panel (or an open panel over the bar), is left out, so `act` finds nothing to press.

A `TextArea` is a text control too: wrap it in a `NamedControl` (role `TextInput`) or put it in a
`Field` and it shows its text as the control's value. `Act` applies `Action::SetValue` to it as
`TextArea::set_text` would: the whole document is replaced, line endings become `\n`, the cursor
stays on a character boundary, and selection and edit history are dropped. The same text again
changes nothing (`Outcome::Handled`). Mark the control `secret` to keep the text out of the
snapshot.

The other value widgets take an agent's action through the state the app already keeps. In each
case wrap the widget in a `NamedControl` with the role below and apply the request in the update
step; the same value again is `Outcome::Handled`, and text the widget would not accept is
`Outcome::Ignored`.

| Widget | Role (actions) | Applied with |
| --- | --- | --- |
| `Slider` | `Slider` (`SetValue`) | `slider.act(&mut value, &action)`: a decimal, clamped and snapped to the step |
| `NumberInput` | `TextInput` (`SetValue`) | `input.act(&mut value, &action)`: held to `NumberInput::parse`, so it rounds and clamps |
| `Accordion` | `Disclosure` (`Toggle`), one per header | `accordion.act(index, &action)`: opens or closes the section; a disabled section is refused |
| `Tree` | `ListItem` (`Select`, `Activate`), one per visible row | `tree_state.act(&path, &action)`: `Activate` opens or closes a branch and selects a leaf |
| `Table` | `ListItem` (`Select`, `Activate`) per visible row; `Button` (`Activate`) per header | `state.act(row, &action)` puts the cursor on a row; `state.act_header(&source, column, columns, &action)` sorts by the column or reverses it |
| `RadioGroup` | `RadioButton` (`Select`), one per option | `group.act(&mut selected, index, &action)`: the option's index from `option_areas`; a disabled option is refused |
| `DatePicker` | `Calendar` (`SetValue`) | `picker.act(&mut cursor, &mut selected, &action)`: text as `YYYY-MM-DD`; a date outside the bounds, or a disabled picker, is refused |
| `Select` | `ComboBox` (`SetValue`) | `select.act(&mut chosen, &action)`: the exact option label; `Activate` is the app's layer, so it is ignored |
| `TagInput` | `TagInput` (`SetValue`, `Activate`) | `TagState`: `SetValue` replaces the typed text, `Activate` commits it as a tag |
| `Monthly` | `Calendar` (`SetValue`) | `Date`: text as `YYYY-MM-DD`; a day that does not exist is ignored |

`Slider::act` and `NumberInput::act` bound a plain `f64` that the app owns, so they are not `Act`
themselves. `ActWith` lends `Act` to a closure for `Request::apply`:
`request.apply(&mut ActWith(|action| slider.act(&mut value, action)), request.context())`. A tree
branch that is closed hides its children, so they have no controls until `Activate` opens it.

An application can refuse what an agent asks for. `Request::apply(&mut state, request.context().guard(&mut guard))`
asks the guard before the state changes, whatever type implements `Act`: the guard runs in the library, so a
state cannot skip it. The guard sees the reason (`Edit` for a value, `Toggle`, `Select`, `Activate` or `Navigate`) and
the source (`Agent`), and sets `cancel` to refuse. A refusal leaves the state and the screen unchanged and returns a
result with no accepted payload. An approved result carries the state's own `Outcome`, which can be `Ignored` for
an action the state does not take. The guard is asked before the state decides, so it also sees an edit that would
change nothing.

A snapshot also says how a person reaches a control from the keyboard. A menu item reports its
mnemonic letter (the one the menu underlines) as `mnemonic`. `KeyHintBar::controls(ItemControls::new(first))`
registers each key hint drawn in full as a button (role `Button`) named by the command's title, with the
chord it shows as `shortcut` (`Ctrl+S`), and the id `first + n` for the `n`th command that has a chord; a
hint the row clips, and a command with no chord, are left out. `Action::Activate` on one reaches the app as a
request for that id, which the app maps back to its command. Both facets are for the agent only: the
accessibility export does not write them.

An agent that waits for something to change subscribes instead of comparing snapshots itself.
`Agent::subscribe(query)` returns a `Subscription` for the controls a `ControlQuery` matches. Call
`Subscription::observe(&hits, focused)` once per frame: the first call only records the controls,
and each later call returns a `ControlChange` for every matched control whose value, checked state
or states (disabled, focused, selected, invalid) differ from the previous frame, plus those that
appeared (`previous` is `None`) or went away (`current` is `None`), in registration order. A frame
that changes nothing returns nothing.

A change carries its `reason` and `source` when the application recorded them. Every `Change` an
accepted operation returns has `details()`, the `ChangeDetails` with its reason and source; pass
them to the control in the next frame with `ControlMetadata::changed`, or `Field::changed` for a
field. A key that picks a radio option then reports `source` `Keyboard`, a click `Mouse`, and an
`Agent::act` the app applies with `ChangeContext::new(ChangeSource::Agent)` reports `Agent`. A
value that changed without a recorded change, such as a reload, reports `None` for both.

```rust,ignore
let mut watch = Agent::subscribe(ControlQuery::kind(AccessibilityRole::Checkbox));
// each frame, after rendering into `hits`:
for change in watch.observe(&hits, focus.focused()) {
    println!("{} {:?} by {:?}", change.name, change.reason, change.source);
}
```

Some actions should not happen on an agent's word alone. Mark a control whose action destroys
work with `ControlMetadata::destructive(true)`, or `Field::destructive(true)` for a field; the
snapshot shows it as `destructive`. By default `act` then holds any request that would change the
control (anything but `Action::Focus`) and returns `ActError::NeedsConfirmation(id)` instead of
queueing it. `Agent::awaiting()` lists the held requests with the control's name, role and the
action, so the app can ask the person. On their yes, `Agent::confirm(&hits, id)` queues the request
and returns its `Ticket`, after checking that the same control is still there and enabled; on their
no, `Agent::decline(id)` drops it. A request leaves the list either way, and an agent that asks
again gets a new request to wait on, so only the app can let it through.

`Agent::allow(hook)` replaces that rule. The hook sees each registered control (its name, role,
layer and `is_destructive()`) and action that passed the other checks and answers `Permission::Allow`, `Deny` (`act` returns `ActError::Denied`
and queues nothing) or `Confirm`. Call `Permission::default_for(control, action)` for the controls
the hook has no rule for.

```rust,ignore
agent.allow(|control, action| match control.name {
    "Save" => Permission::Allow,
    "Send payment" => Permission::Deny,
    _ => Permission::default_for(control, action),
});
// when the person answers the prompt for `waiting.id`:
let ticket = agent.confirm(&hits, waiting.id)?; // or agent.decline(waiting.id)
```

An agent that needs several steps gives them all at once. `Agent::plan(steps)` takes `Step`s
(`Step::new(selector, action)`) and returns a `PlanId`; nothing is checked yet. Call
`Agent::advance(&hits)` once after each render. It checks the plan's current step against that
frame as `act` would, the allow hook and confirmation included, and queues it. The next step waits
until the app has taken that request and drawn again, so a step can name a control an earlier step
made appear, such as a menu item, or enabled, such as a Submit button that needs the fields above
it. `Agent::plan_status(id)` answers `Running { step }`, `Waiting { step, confirm }` while the
application asks the person, `Done` once the app has taken every step, `Failed { step, error }`,
or `Cancelled`; steps count from 0. The plan stops at the first step that fails: the steps before
it stay applied and none after it is queued. A declined step fails with `ActError::Denied`.
`Agent::cancel_plan(id)` stops a plan, cancelling its queued request or dropping its waiting one.
One plan runs at a time; `plan` returns `ActError::Busy` until the current one ends, and only the
latest plan has a status. A step counts as done when the app takes its request, so a request the
app's guard refuses does not fail the plan.

```rust,ignore
let plan = agent.plan([
    Step::new(Selector::new(AccessibilityRole::TextInput, "Name"), Action::SetValue("Ada".into())),
    Step::new(Selector::new(AccessibilityRole::Checkbox, "Accept terms"), Action::Toggle),
    Step::new(Selector::new(AccessibilityRole::Button, "Submit"), Action::Activate),
])?;
// each frame, after rendering into `hits`:
agent.advance(&hits);
if let Some(PlanStatus::Failed { step, error }) = agent.plan_status(plan) {
    println!("step {step} failed: {error}");
}
```

Values are for the agent only. `write_accessibility_json` never writes them. Mark a password
field with `Field::secret(true)` (or `NamedControl::secret`) and its value is left out of the
snapshot as well.

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
uses the example's injected virtual `Clock` to render 300 fixed frames, then
checks names, roles, disabled state, focus checkpoints and value exclusion in
the actual JSON snapshots. It verifies partial typing, the invalid-email pause,
the completed email, a disabled-field focus skip and saved copy in actual text
output. The GIF rasterizes those real widget bytes with the pinned agg renderer.
The decoded file must last 6 to 12 seconds, remain under 2 MB, contain at least 40
distinct RGB frames, and show each off/on/off/on switch transition. A PNG still
is extracted from the GIF during validation. Set `AGG` to the pinned renderer
when another agg is first on your path.

From a clean committed checkout, `check-named-controls.py` archives HEAD and
verifies that the real assertions reject eight deliberate regressions: dropped
registration, ambiguous first-match selection, disabled activation, lost scroll
translation, missing names, exported input data, and inputs outside a narrow form.
Logs remain in `target/named-controls-check`; temporary source directories are
removed. Repeat the drive in a clean VM for PR proof.
