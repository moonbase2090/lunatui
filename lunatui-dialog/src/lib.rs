// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::borrow::Cow;

use lunatui_core::{Rect, Size};
use lunatui_ui_tokens::BORDER;
use lunatui_widgets::{BorderType, Dialog as Core, RenderCx, Widget};

pub use lunatui_widgets::{DialogClosed, DialogFocus, DialogKind, DialogOutcome, DialogState};

/// A centered panel over a dimmed screen, with a message, an optional field and buttons.
#[derive(Clone, Debug)]
pub struct Dialog<'a> {
    core: Core<'a>,
}

impl<'a> Dialog<'a> {
    fn wrap(core: Core<'a>) -> Self {
        Self {
            core: core.border_type(BORDER),
        }
    }

    /// A message and an OK button.
    #[must_use]
    pub fn alert(title: impl Into<String>, message: impl Into<Cow<'a, str>>) -> Self {
        Self::wrap(Core::alert(title, message))
    }

    /// A message with Cancel and OK buttons.
    #[must_use]
    pub fn confirm(title: impl Into<String>, message: impl Into<Cow<'a, str>>) -> Self {
        Self::wrap(Core::confirm(title, message))
    }

    /// A message, a field showing `text` with its caret at byte `caret`, and Cancel and OK buttons.
    #[must_use]
    pub fn prompt(
        title: impl Into<String>,
        message: impl Into<Cow<'a, str>>,
        text: impl Into<Cow<'a, str>>,
        caret: usize,
    ) -> Self {
        Self::wrap(Core::prompt(title, message, text, caret))
    }

    /// Dimmed text shown in an empty field.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.core = self.core.placeholder(placeholder);
        self
    }

    /// The confirm button's label, `OK` by default.
    #[must_use]
    pub fn confirm_label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.core = self.core.confirm_label(label);
        self
    }

    /// The cancel button's label, `Cancel` by default.
    #[must_use]
    pub fn cancel_label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.core = self.core.cancel_label(label);
        self
    }

    /// Draws the confirm button in the destructive color.
    #[must_use]
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.core = self.core.destructive(destructive);
        self
    }

    /// Panel width in columns, including the border.
    #[must_use]
    pub fn width(mut self, columns: u16) -> Self {
        self.core = self.core.width(columns);
        self
    }

    /// Shows the focus kept in `state`.
    #[must_use]
    pub fn state(mut self, state: &DialogState) -> Self {
        self.core = self.core.state(state);
        self
    }

    /// Border line style; rounded by default.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.core = self.core.border_type(border_type);
        self
    }

    /// How far the screen behind fades, from `0.0` to `1.0`.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32) -> Self {
        self.core = self.core.backdrop(alpha);
        self
    }

    /// The kind of dialog.
    #[must_use]
    pub fn kind(&self) -> DialogKind {
        self.core.kind()
    }

    /// The cells the panel covers inside `area`.
    #[must_use]
    pub fn area(&self, area: Rect) -> Rect {
        self.core.area(area)
    }

    /// The control drawn at `(column, row)` inside `area`, for [`DialogState::click`].
    #[must_use]
    pub fn control_at(&self, area: Rect, column: u16, row: u16) -> Option<DialogFocus> {
        self.core.control_at(area, column, row)
    }
}

impl Widget for Dialog<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.core.render(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        self.core.measure(avail)
    }
}

/// A confirmation for an action that can't be undone: Cancel, and a danger-colored action button.
#[derive(Clone, Debug)]
pub struct AlertDialog<'a> {
    dialog: Dialog<'a>,
}

impl<'a> AlertDialog<'a> {
    /// A title and description, with Cancel and Continue buttons.
    #[must_use]
    pub fn new(title: impl Into<String>, description: impl Into<Cow<'a, str>>) -> Self {
        Self {
            dialog: Dialog::confirm(title, description)
                .confirm_label("Continue")
                .destructive(true),
        }
    }

    /// The action button's label, `Continue` by default.
    #[must_use]
    pub fn action(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.dialog = self.dialog.confirm_label(label);
        self
    }

    /// The cancel button's label, `Cancel` by default.
    #[must_use]
    pub fn cancel(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.dialog = self.dialog.cancel_label(label);
        self
    }

    /// Whether the action button uses the destructive color; on by default.
    #[must_use]
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.dialog = self.dialog.destructive(destructive);
        self
    }

    /// Panel width in columns, including the border.
    #[must_use]
    pub fn width(mut self, columns: u16) -> Self {
        self.dialog = self.dialog.width(columns);
        self
    }

    /// Shows the focus kept in `state`.
    #[must_use]
    pub fn state(mut self, state: &DialogState) -> Self {
        self.dialog = self.dialog.state(state);
        self
    }

    /// How far the screen behind fades, from `0.0` to `1.0`.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32) -> Self {
        self.dialog = self.dialog.backdrop(alpha);
        self
    }

    /// The cells the panel covers inside `area`.
    #[must_use]
    pub fn area(&self, area: Rect) -> Rect {
        self.dialog.area(area)
    }

    /// The control drawn at `(column, row)` inside `area`, for [`DialogState::click`].
    #[must_use]
    pub fn control_at(&self, area: Rect, column: u16, row: u16) -> Option<DialogFocus> {
        self.dialog.control_at(area, column, row)
    }
}

impl Widget for AlertDialog<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.dialog.render(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        self.dialog.measure(avail)
    }
}
