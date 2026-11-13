// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_runtime::{Handle, HitId, Orientation, Outcome, RovingFocus};
use lunatui_term::{Event, KeyCode, KeyKind, Modifiers};

use crate::Form;

/// Which field of a [`Form`] has focus, moved by Tab and Shift+Tab.
///
/// Call [`sync`](Self::sync) with the form after building it, so focus skips disabled fields and
/// follows the focused field when fields are added, removed or reordered. A field is known by its
/// [`Field::id`](crate::Field::id) when it has one, and otherwise by its accessible name. Tab moves to the next enabled field and Shift+Tab to the
/// one before, both wrapping around; arrows and other keys are left to the focused control. Draw
/// with `Form::focus(state.focused())`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormState {
    roving: RovingFocus,
    enabled: Vec<bool>,
    keys: Vec<Key>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Key {
    Id(HitId),
    Name(String),
}

impl Key {
    fn matches(&self, id: Option<HitId>, name: &str) -> bool {
        match self {
            Self::Id(key) => id == Some(*key),
            Self::Name(key) => id.is_none() && key == name,
        }
    }
}

impl FormState {
    /// No field focused.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            roving: RovingFocus::new(Orientation::Vertical),
            enabled: Vec::new(),
            keys: Vec::new(),
        }
    }

    /// Records the form's fields and which can take focus. Focus stays on the same field wherever
    /// it moved. When that field is gone or disabled, focus moves to the nearest field before it
    /// in the old order that is still there and enabled, then the nearest one after it, then the
    /// first enabled field.
    pub fn sync(&mut self, form: &Form<'_>) {
        let keys: Vec<(Option<HitId>, &str)> = form.keys().collect();
        self.enabled.clear();
        self.enabled.extend(form.enabled());
        let next = match self.roving.current() {
            Some(old) if old < self.keys.len() => self.follow(old, &keys),
            _ => {
                let enabled = &self.enabled;
                self.roving.sync(enabled.len(), |index| enabled[index]);
                self.roving.current()
            }
        };
        self.roving.set(next);
        self.keys.clear();
        self.keys.extend(keys.iter().map(|(id, name)| match id {
            Some(id) => Key::Id(*id),
            None => Key::Name((*name).to_owned()),
        }));
    }

    fn follow(&self, old: usize, keys: &[(Option<HitId>, &str)]) -> Option<usize> {
        let order = (0..=old).rev().chain(old + 1..self.keys.len());
        order
            .filter_map(|index| self.find(index, keys))
            .find(|index| self.enabled[*index])
            .or_else(|| self.enabled.iter().position(|enabled| *enabled))
    }

    fn find(&self, old: usize, keys: &[(Option<HitId>, &str)]) -> Option<usize> {
        let key = &self.keys[old];
        let rank = self.keys[..old]
            .iter()
            .filter(|other| *other == key)
            .count();
        keys.iter()
            .enumerate()
            .filter(|(_, (id, name))| key.matches(*id, name))
            .nth(rank)
            .map(|(index, _)| index)
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
