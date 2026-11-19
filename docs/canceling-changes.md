# Refuse a widget change

Use `Handle::handle_with` when an application needs to approve a change before the widget commits it. The default `handle` accepts recognized operations and reports their reason and source.

```rust
use lunatui_runtime::{ChangeContext, ChangeDetails, ChangeSource, Handle};

let mut policy = |details: &mut ChangeDetails| {
    details.cancel = details.source() == ChangeSource::Agent;
};
let result = state.handle_with(
    &event,
    ChangeContext::new(ChangeSource::Agent).guard(&mut policy),
);
if result.is_canceled() {
    // Keep the event consumed. No accepted payload is available.
}
```

Use `ChangeContext::for_event(&event)` for terminal input. Key events report `Keyboard`, pointer events report `Mouse`, and paste reports `Terminal`. Synthetic events need an explicit source. A guard may set `cancel`; it cannot replace the operation's reason or source.

Read the result with `value()` or consume it with `into_value()`. Both expose a payload only after approval. Use `is_taken()` to route input, because a refused operation still consumes its event. `details()` reports accepted and refused operations. Changing the returned details cannot undo a committed operation.

## Refuse Escape before closing a dialog

Route an event through the layer stack before its underlying controls. Pass the same receipt to the overlay state before processing the next event.

```rust
let mut policy = |details: &mut ChangeDetails| {
    details.cancel = unsaved && matches!(details.reason(), ChangeReason::Escape(_));
};
let receipt = layers.handle_with(
    &event,
    &mut focus,
    ChangeContext::for_event(&event).guard(&mut policy),
);
let closed = dialog.sync(&layers, &receipt);
if receipt.delivers() {
    // Deliver to the current control.
}
```

A refused close leaves the layer stack, dialog, and focus unchanged. A refused outside press also prevents click-through. `LayerStack::close_with` supplies the same receipt for an explicit close. If application code uses immediate `remove`, `pop`, or `close` instead, synchronization still clears an absent layer once and reports `Close(layer)` from `Application`. `DialogState::activate_with` and `click_with` approve before changing button focus or closing the layer. `PopoverState::close_with` and `toggle_with` approve their complete operation.

`DialogOutcome::Cancelled` is the user's accepted answer. `Change::is_canceled()` means the application refused an operation, so no answer occurred.

## Refuse an external document replacement

```rust
let result = editor.set_text_with(
    proposed_text,
    ChangeContext::new(ChangeSource::Agent).guard(&mut |details| {
        details.cancel = !replacement_approved;
    }),
);
```

A refusal preserves the complete editor, including selection, search, paste buffering, undo, and redo. An accepted `set_text_with` replaces the document and clears history, just like `set_text`. For undoable edits, use `handle_with`, `handle_event_with`, or `execute_command_with` instead.

Run `cargo run -p lunatui --example change_guard` to try both refusals. Press F2 to grant permission, Escape to close the dialog, F3 to propose the external replacement, and F10 to exit.

## Migrate existing callers

| Previous call or result | Current form |
| --- | --- |
| `Handle::handle` returned `Outcome` | Returns `Change<Outcome>`. Match `result.value()` or `result.into_value()` and route with `is_taken()`. |
| Custom `impl Handle` supplied `handle` | Implement `handle_with(event, context)` and approve before changing live state. The default `handle` supplies terminal provenance. |
| `DialogState::activate` and `click` | Return `Change<DialogClosed>` and `Change<Option<DialogClosed>>`. Use guarded variants to install a policy. |
| Overlay `sync(&layers)` | Supply `sync(&layers, &receipt)`. `LayerSlot::sync` returns optional details; `DialogState`, `PopoverState`, `SelectState`, and `Dismissal` return a `Change`. |
| `LayerEvent::Dismissed { layer, deliver }` | Also carries `details`. Match `LayerEvent::Canceled` or use `delivers()` to consume refusals. |
| `MenuEvent::Selected(value)` | Match `MenuEvent::Selected { value, details }`. `select_with` and `press_mnemonic_with` approve before cloning the action or queuing it. |
| `TabsDrag::release` returned `Option<TabMove>` | Returns `Change<Option<TabMove>>`. Extract with `into_value().flatten()`. Refusal retains the pending gesture. |
| `TabMove::apply` | `apply_with` guards the item order and selected index together. The deferred `TabMove` record itself does not mutate the items. |
| `SplitDrag::drag` | Returns `Change<()>`. `drag_with` guards tree ratios and gesture state together. |
| `TextArea::handle_event` and `execute_command` | Return `Change<EditResult>`. Their `_with` variants accept a context. Clearing paste or preferred-column state reports `StateChanged`. |

Immediate helpers remain immediate. Examples include `LayerStack::remove`, `pop`, and `close`, overlay `open` and `close`, `RovingFocus::handle_key`, `FormState::step`, and editor movement, configuration, undo, and redo methods. A policy applies at the guarded entry point you choose. Route overlay dismissal through receipt-producing methods to preserve its original reason and source. Immediate removals synchronize with application-close provenance. For a composite application operation, use `ChangeContext::perform(reason, || operation())` to run the whole operation after approval.

## Why approval happens inside the operation

An outcome returned after mutation cannot prevent a close or restore focus safely. `ChangeContext` asks the application before committing. It returns a shared `Change<T>` without exposing an accepted payload on refusal.

Small operations approve before touching live fields. Complex built-in states run their existing interpreter on a private owned draft only when a guard is installed, then replace live state after approval. Unguarded input keeps the direct path. The guarded editor draft copies its document and history, so its cost grows with their size. Buffered paste chunks approve before appending instead of repeatedly copying the growing buffer.

The private draft helper is limited to audited built-in state. A downstream `Handle` implementation owns its transaction strategy. Cloning a state with shared mutable references would not provide cancellation.

Menu action payloads can have user-defined `Clone` implementations. Menu decisions therefore plan paths without cloning payloads, approve, and only then clone and queue the chosen action.
