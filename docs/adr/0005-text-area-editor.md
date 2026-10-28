# ADR 0005: TextArea owns its document and viewport state

**Status:** Accepted

## Context

`Widget::render` borrows a widget immutably. The app owns event routing and focus. `TextInput` is a render-only field, while `lunatui-runtime::keymap` maps key chords to named commands. TextArea needs editable multi-line state without changing those ownership rules.

The editor also needs undo, selection across lines, search, and cursor-follow scrolling. These operations share byte ranges in one UTF-8 document. Per-line buffers would require separate paths for splitting lines, joining lines, replacing selections, and undoing edits.

## Decision

`TextArea` owns one UTF-8 `String`, a line-start index, a byte-offset cursor, an optional selection anchor, a yank buffer, search state, and bounded edit history. Public cursor and selection positions use line numbers and grapheme ordinals. Callers never provide byte offsets.

Every document change goes through one replacement helper. The helper updates the document and line index, records the removed and inserted bytes with the cursor state, clears redo, and refreshes search matches. Undo and redo apply the inverse or forward replacement through the same helper.

After an edit, the editor moves the cursor to a valid grapheme boundary. It uses right affinity when an insertion or deletion joins adjacent graphemes. Undo restores the cursor and selection from the state saved before the edit. CRLF and CR normalize to LF when the editor receives text, including bracketed paste.

The editor implements `Widget` with a read-only render. The app routes events to the editor only when the focus manager marks it focused. `WidgetState` controls focused and disabled appearance. Disabled editors ignore input.

The editor uses `lunatui-runtime::keymap` for its built-in Emacs and Vim command registries. The editor exposes its active registry for shortcut overlays. It does not accept arbitrary command IDs because the editor owns the meaning of each command.

Search has an input state. Emacs `Ctrl+S` and Vim `/` open it. Printable keys and bracketed paste update the query; Enter accepts the current match, and Escape restores the previous cursor. Shift movement extends a directed selection. Vim tracks Normal and Insert modes separately.

Bracketed paste buffers its chunks and applies them as one undo edit at the closing event. Key repeats run repeatable editor commands locally; the runtime registry remains press-only.

The widget stores explicit vertical and horizontal scroll offsets. Rendering computes a temporary viewport from those offsets, the cursor, and the available rectangle. If the cursor falls outside the viewport, the rendered viewport follows it without mutating the widget. Vertical movement keeps the preferred display column across short lines. Horizontal clipping skips a wide grapheme that would be cut at either edge.

The opt-in `text-area` feature enables `WidgetState`, `lunatui-theme/graphite-tokens`, and the runtime keymap and terminal event dependencies. The widget uses semantic theme tokens for text, borders, cursor, selection, search, gutter, and status. It does not store fixed colors.

## Usage

```rust
let mut editor = TextArea::new("Title\nWrite the release note.")
    .keymap_mode(KeymapMode::Vim)
    .line_numbers(true);
editor.set_widget_state(WidgetState::Focused);
editor.handle_event(&event);
editor.render(area, &mut cx);
```

The event result distinguishes ignored input, editor-state changes, document edits, and edits that exceed the undo budget. `text()` returns a borrowed string. `cursor()` and `selection()` return grapheme positions.

## Alternatives

Per-line strings make line-local edits direct, but selection replacement, search, and undo need cross-line conversion helpers. Whole-document snapshots also copy the document for every edit. The selected flat buffer and edit deltas keep those operations in one replacement path and bound history memory.

The Criterion case uses Ratatui `Paragraph` as a rendering baseline. It draws the same document, line numbers, cursor cell, surface colors, and terminal sizes on both sides. It measures drawing only; selection and search behavior are covered by literal interaction tests. It does not claim that Ratatui `Paragraph` provides editor behavior.

## Verification

Behavior fixtures cover line splitting and joining, both selection directions, grapheme recomposition, undo and redo branch clearing, paste transactions, search wrap, disabled input, Vim mode changes, cursor-follow scrolling, resize, and clipping. The widget suite requires exact cell parity at 80×24 and 200×60. Criterion measures the same render-only and full-frame scopes on both sides.

The recording uses actual editor events to show typing, selection, search, and cursor movement at 120×36 or larger. The PR includes the dark-theme GIF and PNG in the README gallery. Separate truecolor and ANSI-16 captures check the palette fallbacks.

Two design sketches were compared against grapheme safety, edit history, API size, rendering, and verification coverage. The flat-buffer design scored 18/25; the per-line snapshot design scored 15/25. The flat buffer was selected. The final design adds directed selection and exact parity from the per-line sketch, plus explicit paste, repeat, viewport, and grapheme-seam rules from review.
