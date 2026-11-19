// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt;

use lunatui_term::Event;

use crate::LayerId;

/// Where an interaction originated. Synthetic events need an explicit source.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeSource {
    /// A decoded key event.
    Keyboard,
    /// A decoded pointer event.
    Mouse,
    /// An action supplied by an agent.
    Agent,
    /// An application called an imperative operation.
    Application,
    /// Terminal input without a known physical source, such as bracketed paste.
    Terminal,
}

/// The operation being considered, before it changes live state.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeReason {
    /// Move focus, a cursor, or the active item.
    Navigate,
    /// Choose an item.
    Select,
    /// Toggle a selection.
    Toggle,
    /// Activate an item or command.
    Activate,
    /// Open an overlay.
    Open,
    /// Escape would dismiss this layer.
    Escape(LayerId),
    /// A press outside would dismiss this layer.
    OutsidePress(LayerId),
    /// Confirm a dialog.
    Confirm,
    /// Use a dialog's cancel button.
    CancelButton,
    /// Explicitly close this layer.
    Close(LayerId),
    /// Extend or reset a type-ahead query.
    Typeahead,
    /// Insert, delete, or replace text.
    Edit,
    /// Undo an edit.
    Undo,
    /// Redo an edit.
    Redo,
    /// Start, extend, complete, or reject buffered paste.
    Paste,
    /// Change a search query or its match.
    Search,
    /// Copy selected text to the editor's private clipboard.
    Copy,
    /// Change the editor's input mode.
    Mode,
    /// Reorder items.
    Reorder,
    /// Resize a pane.
    Resize,
    /// Begin a pointer gesture.
    DragStart,
    /// Complete a pointer gesture.
    DragEnd,
    /// Abort a pointer gesture.
    DragCancel,
    /// Leave an interaction without activating its current item.
    Cancel,
    /// Use a menu mnemonic, including cycling among duplicate mnemonics.
    Mnemonic,
}

/// Why an operation was proposed and whether the application refused it.
///
/// A guard may set [`cancel`](Self::cancel). Returned details record that decision;
/// changing a returned copy does not undo an operation that already committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeDetails {
    reason: ChangeReason,
    source: ChangeSource,
    /// Set to `true` inside the guard to leave participating live state unchanged.
    pub cancel: bool,
}

impl ChangeDetails {
    /// An operation that has not been refused.
    #[must_use]
    pub const fn new(reason: ChangeReason, source: ChangeSource) -> Self {
        Self {
            reason,
            source,
            cancel: false,
        }
    }

    /// The operation considered by the guard.
    #[must_use]
    pub const fn reason(&self) -> ChangeReason {
        self.reason
    }

    /// The origin supplied by the caller or inferred from a terminal event.
    #[must_use]
    pub const fn source(&self) -> ChangeSource {
        self.source
    }
}

/// An interaction result with the application's change decision.
///
/// Only accepted operations expose a [`value`](Self::value). A canceled event is
/// still taken, so it must not fall through to a second control or a quit binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change<T> {
    state: ChangeState<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangeState<T> {
    Ignored,
    Handled,
    Applied(T, ChangeDetails),
    Canceled(ChangeDetails),
}

impl<T> Change<T> {
    /// Input this state does not recognize.
    #[must_use]
    pub const fn ignored() -> Self {
        Self {
            state: ChangeState::Ignored,
        }
    }

    /// Taken without a proposed operation.
    #[must_use]
    pub const fn handled() -> Self {
        Self {
            state: ChangeState::Handled,
        }
    }

    /// An accepted operation and its provenance.
    #[must_use]
    pub const fn new(value: T, reason: ChangeReason, source: ChangeSource) -> Self {
        Self {
            state: ChangeState::Applied(value, ChangeDetails::new(reason, source)),
        }
    }

    /// Whether this state took the input, including a refused operation.
    #[must_use]
    pub const fn is_taken(&self) -> bool {
        !matches!(self.state, ChangeState::Ignored)
    }

    /// Whether the application refused the proposed operation.
    #[must_use]
    pub const fn is_canceled(&self) -> bool {
        matches!(self.state, ChangeState::Canceled(_))
    }

    /// The accepted payload, absent for ignored, unchanged, or refused input.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match &self.state {
            ChangeState::Applied(value, _) => Some(value),
            _ => None,
        }
    }

    /// The accepted payload, consuming the result.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        match self.state {
            ChangeState::Applied(value, _) => Some(value),
            _ => None,
        }
    }

    /// The original reason, source and decision for a proposed operation.
    #[must_use]
    pub const fn details(&self) -> Option<&ChangeDetails> {
        match &self.state {
            ChangeState::Applied(_, details) | ChangeState::Canceled(details) => Some(details),
            _ => None,
        }
    }

    /// Maps an accepted payload, preserving its details and all other outcomes.
    #[must_use]
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Change<U> {
        Change {
            state: match self.state {
                ChangeState::Ignored => ChangeState::Ignored,
                ChangeState::Handled => ChangeState::Handled,
                ChangeState::Applied(value, details) => ChangeState::Applied(map(value), details),
                ChangeState::Canceled(details) => ChangeState::Canceled(details),
            },
        }
    }
}

/// Source attribution and an optional policy, borrowed for one operation.
///
/// Built-in handlers invoke the guard before committing their live state. Custom
/// handlers must prepare without external effects and use [`commit`](Self::commit)
/// or [`apply`](Self::apply); cloning arbitrary user state does not prove isolation.
pub struct ChangeContext<'a> {
    source: ChangeSource,
    guard: Option<&'a mut dyn FnMut(&mut ChangeDetails)>,
}

impl fmt::Debug for ChangeContext<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChangeContext")
            .field("source", &self.source)
            .field("guarded", &self.is_guarded())
            .finish()
    }
}

impl<'a> ChangeContext<'a> {
    /// Accepts operations from `source` unless a guard is attached.
    #[must_use]
    pub const fn new(source: ChangeSource) -> Self {
        Self {
            source,
            guard: None,
        }
    }

    /// Attributes keys to the keyboard, pointer input to the mouse, and other
    /// decoded input to the terminal. Use [`new`](Self::new) for agent input.
    #[must_use]
    pub const fn for_event(event: &Event) -> Self {
        Self::new(match event {
            Event::Key(_) => ChangeSource::Keyboard,
            Event::Mouse(_) => ChangeSource::Mouse,
            _ => ChangeSource::Terminal,
        })
    }

    /// Calls `guard` before each proposed operation commits.
    #[must_use]
    pub fn guard(mut self, guard: &'a mut dyn FnMut(&mut ChangeDetails)) -> Self {
        self.guard = Some(guard);
        self
    }

    /// The origin of this operation.
    #[must_use]
    pub const fn source(&self) -> ChangeSource {
        self.source
    }

    /// Whether an implementation needs to prepare a proposal before mutation.
    #[must_use]
    pub const fn is_guarded(&self) -> bool {
        self.guard.is_some()
    }

    /// Authorizes a prepared operation, then calls `commit` once if accepted.
    /// Ignored and unchanged outcomes call neither the guard nor `commit`.
    pub fn commit<T>(&mut self, mut proposed: Change<T>, commit: impl FnOnce()) -> Change<T> {
        if let ChangeState::Applied(_, details) = &mut proposed.state {
            let reason = details.reason;
            details.source = self.source;
            if let Some(guard) = &mut self.guard {
                guard(details);
            }
            *details = ChangeDetails {
                reason,
                source: self.source,
                cancel: details.cancel,
            };
            if details.cancel {
                return Change {
                    state: ChangeState::Canceled(*details),
                };
            }
            commit();
        }
        proposed
    }

    /// Authorizes an operation whose result is already known, then applies it.
    pub fn apply<T>(&mut self, reason: ChangeReason, value: T, commit: impl FnOnce()) -> Change<T> {
        self.commit(Change::new(value, reason, self.source), commit)
    }

    /// Authorizes before executing an operation that computes its own result.
    /// Place payload cloning and action callbacks inside the operation; they run
    /// only after approval.
    pub fn perform<T>(&mut self, reason: ChangeReason, operation: impl FnOnce() -> T) -> Change<T> {
        self.apply(reason, (), || {}).map(|()| operation())
    }
}
