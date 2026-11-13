// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_runtime::{Handle, Orientation, Outcome, RovingFocus};
use lunatui_term::{Event, KeyCode, KeyKind, Modifiers};

use crate::Form;

/// Which field of a [`Form`] has focus, moved by Tab and Shift+Tab.
///
/// Call [`sync`](Self::sync) with the form after building it, so focus skips disabled fields and
/// stays on a field that still exists. Tab moves to the next enabled field and Shift+Tab to the
/// one before, both wrapping around; arrows and other keys are left to the focused control. Draw
/// with `Form::focus(state.focused())`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormState {
    roving: RovingFocus,
    enabled: Vec<bool>,
}

impl FormState {
    /// No field focused.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            roving: RovingFocus::new(Orientation::Vertical),
            enabled: Vec::new(),
        }
    }

    /// Records the form's fields and which can take focus, and moves focus off a field that is
    /// gone or disabled to the nearest enabled one before it.
    pub fn sync(&mut self, form: &Form<'_>) {
        self.enabled.clear();
        self.enabled.extend(form.enabled());
        let enabled = &self.enabled;
        self.roving.sync(enabled.len(), |index| enabled[index]);
    }

    /// The focused field.
    #[must_use]
    pub const fn focused(&self) -> Option<usize> {
        self.roving.current()
    }

    /// Focuses field `index` when it can take focus. Returns whether it did.
    pub fn focus(&mut self, index: usize) -> bool {
        let ok = self.enabled.get(index).copied().unwrap_or(false);
        if ok {
            self.roving.set(Some(index));
        }
        ok
    }

    /// Moves focus to the next enabled field, or the one before, wrapping around, as Tab and
    /// Shift+Tab do. Apps call it for other keys that should move between fields, such as Up and
    /// Down on a phone keyboard with no Shift+Tab.
    pub fn step(&mut self, forward: bool) -> Outcome {
        let enabled = &self.enabled;
        let delta = if forward { 1 } else { -1 };
        if self
            .roving
            .move_by(enabled.len(), delta, |index| enabled[index])
        {
            Outcome::Changed
        } else {
            Outcome::Handled
        }
    }

    /// Focuses the first field with a validation message, such as after
    /// [`Form::check_required`]. Returns whether one took focus.
    pub fn focus_first_error(&mut self, form: &Form<'_>) -> bool {
        form.first_error().is_some_and(|index| self.focus(index))
    }
}

impl Handle for FormState {
    fn handle(&mut self, event: &Event) -> Outcome {
        let Event::Key(key) = event else {
            return Outcome::Ignored;
        };
        if !matches!(key.kind, KeyKind::Press | KeyKind::Repeat) || key.code != KeyCode::Tab {
            return Outcome::Ignored;
        }
        if key.modifiers == Modifiers::new() {
            self.step(true)
        } else if key.modifiers == Modifiers::new().shift() {
            self.step(false)
        } else {
            Outcome::Ignored
        }
    }
}
