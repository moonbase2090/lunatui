// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::overlay::FocusTrap;
use crate::{BorderType, Popup, RenderCx, Widget};

const DEFAULT_WIDTH: u16 = 50;
const SIDE: u16 = 2;
const BUTTON_PAD: u16 = 2;
const BUTTON_GAP: u16 = 2;

/// Which kind of [`Dialog`] is open: what it asks and which controls it has.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogKind {
    /// A message with one button that acknowledges it.
    #[default]
    Alert,
    /// A question with a cancel and a confirm button.
    Confirm,
    /// A question with a one-line text field, a cancel and a confirm button.
    Prompt,
}

impl DialogKind {
    fn order(self) -> &'static [DialogFocus] {
        match self {
            Self::Alert => &[DialogFocus::Confirm],
            Self::Confirm => &[DialogFocus::Cancel, DialogFocus::Confirm],
            Self::Prompt => &[
                DialogFocus::Input,
                DialogFocus::Cancel,
                DialogFocus::Confirm,
            ],
        }
    }

    fn first(self) -> DialogFocus {
        match self {
            Self::Prompt => DialogFocus::Input,
            Self::Alert | Self::Confirm => DialogFocus::Confirm,
        }
    }
}

/// The control inside a [`Dialog`] that has focus, or that a click landed on.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogFocus {
    /// The prompt's text field.
    Input,
    /// The cancel button.
    Cancel,
    /// The confirm button, or the alert's only button.
    #[default]
    Confirm,
}

/// How a [`Dialog`] was answered.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogOutcome {
    /// The confirm button, or Enter in the prompt's field.
    Confirmed,
    /// The cancel button or Escape.
    Cancelled,
}

/// What [`DialogState`] reports when a dialog closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogClosed {
    outcome: DialogOutcome,
    restore: Option<u32>,
}

impl DialogClosed {
    /// How the dialog was answered.
    #[must_use]
    pub const fn outcome(&self) -> DialogOutcome {
        self.outcome
    }

    /// The focus id passed to [`DialogState::open`], to give focus back to.
    #[must_use]
    pub const fn restore(&self) -> Option<u32> {
        self.restore
    }
}

/// Whether a [`Dialog`] is open and which of its controls has focus, kept by the app between
/// frames.
///
/// [`open`](Self::open) remembers the focus id to give back when the dialog closes. While it is
/// open, [`next`](Self::next) and [`previous`](Self::previous) (Tab and Shift+Tab) cycle through
/// the dialog's own controls and never leave it. [`activate`](Self::activate) (Enter),
/// [`cancel`](Self::cancel) (Escape) and [`click`](Self::click) close it and report the outcome
/// with the id to restore.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DialogState {
    kind: Option<DialogKind>,
    focus: DialogFocus,
    restore: Option<u32>,
}

impl DialogState {
    /// A closed dialog.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            kind: None,
            focus: DialogFocus::Confirm,
            restore: None,
        }
    }

    /// Opens a dialog of `kind`, focusing the prompt's field or else the confirm button, and
    /// remembers `restore`, the focus id the app had before.
    pub fn open(&mut self, kind: DialogKind, restore: Option<u32>) {
        self.kind = Some(kind);
        self.focus = kind.first();
        self.restore = restore;
    }

    /// Whether a dialog is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    /// The kind of the open dialog.
    #[must_use]
    pub const fn kind(&self) -> Option<DialogKind> {
        self.kind
    }

    /// The control with focus.
    #[must_use]
    pub const fn focus(&self) -> DialogFocus {
        self.focus
    }

    /// Moves focus to `focus` when the open dialog has that control. Returns whether it moved.
    pub fn set_focus(&mut self, focus: DialogFocus) -> bool {
        let has = self.kind.is_some_and(|kind| kind.order().contains(&focus));
        if has {
            self.focus = focus;
        }
        has
    }

    /// Moves focus to the next control, wrapping from the last to the first.
    pub fn next(&mut self) {
        self.step(true);
    }

    /// Moves focus to the previous control, wrapping from the first to the last.
    pub fn previous(&mut self) {
        self.step(false);
    }

    /// The focus trap to give the runtime focus manager: active while a dialog is open.
    #[must_use]
    pub fn focus_trap(&self) -> FocusTrap {
        if self.kind.is_some() {
            FocusTrap::enabled()
        } else {
            FocusTrap::disabled()
        }
    }

    /// Answers with the focused control: cancel cancels, the field and confirm confirm.
    pub fn activate(&mut self) -> Option<DialogClosed> {
        let outcome = match self.focus {
            DialogFocus::Cancel => DialogOutcome::Cancelled,
            DialogFocus::Input | DialogFocus::Confirm => DialogOutcome::Confirmed,
        };
        self.close(outcome)
    }

    /// Cancels the open dialog, as Escape does.
    pub fn cancel(&mut self) -> Option<DialogClosed> {
        self.close(DialogOutcome::Cancelled)
    }

    /// Handles a click on `target`, from [`Dialog::control_at`]. A button answers the dialog; the
    /// field takes focus. A click outside the dialog (`None`) does nothing, since it is modal.
    pub fn click(&mut self, target: Option<DialogFocus>) -> Option<DialogClosed> {
        let target = target?;
        if !self.set_focus(target) {
            return None;
        }
        match target {
            DialogFocus::Input => None,
            DialogFocus::Cancel | DialogFocus::Confirm => self.activate(),
        }
    }

    fn step(&mut self, forward: bool) {
        let Some(kind) = self.kind else {
            return;
        };
        let order = kind.order();
        let at = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next = if forward {
            (at + 1) % order.len()
        } else {
            (at + order.len() - 1) % order.len()
        };
        self.focus = order[next];
    }

    fn close(&mut self, outcome: DialogOutcome) -> Option<DialogClosed> {
        self.kind.take()?;
        Some(DialogClosed {
            outcome,
            restore: self.restore.take(),
        })
    }
}

/// A modal dialog: an alert, a confirmation or a one-line prompt, drawn over a faded screen.
///
/// The dialog is a centred [`Popup`]: a titled panel with the message wrapped inside it, the
/// prompt's text field, and its buttons on the bottom row, right aligned with confirm last. The
/// focused control is drawn in `focus.ring`; a [`destructive`](Self::destructive) confirm button
/// uses `fail`. The app keeps the prompt's text and caret and a [`DialogState`], and passes the
/// state to [`state`](Self::state) so the dialog shows its focus.
///
/// The panel is [`width`](Self::width) columns wide (52 by default) and as tall as its content
/// needs, kept inside the area. [`area`](Self::area) and [`control_at`](Self::control_at) give
/// its cells and controls for hit testing. A built dialog draws without allocating.
#[derive(Clone, Debug)]
pub struct Dialog<'a> {
    popup: Popup<Body<'a>>,
}

#[derive(Clone, Debug)]
struct Body<'a> {
    kind: DialogKind,
    message: Cow<'a, str>,
    text: Cow<'a, str>,
    caret: usize,
    placeholder: Cow<'a, str>,
    confirm: Cow<'a, str>,
    cancel: Cow<'a, str>,
    focus: DialogFocus,
    destructive: bool,
    width: u16,
}

impl<'a> Dialog<'a> {
    fn new(kind: DialogKind, title: impl Into<String>, message: impl Into<Cow<'a, str>>) -> Self {
        let body = Body {
            kind,
            message: message.into(),
            text: Cow::Borrowed(""),
            caret: 0,
            placeholder: Cow::Borrowed(""),
            confirm: Cow::Borrowed("OK"),
            cancel: Cow::Borrowed("Cancel"),
            focus: kind.first(),
            destructive: false,
            width: DEFAULT_WIDTH,
        };
        Self {
            popup: Popup::new(body).title(title).backdrop(0.55),
        }
    }

    /// An alert: `message` and an OK button.
    #[must_use]
    pub fn alert(title: impl Into<String>, message: impl Into<Cow<'a, str>>) -> Self {
        Self::new(DialogKind::Alert, title, message)
    }

    /// A confirmation: `message` with Cancel and OK buttons.
    #[must_use]
    pub fn confirm(title: impl Into<String>, message: impl Into<Cow<'a, str>>) -> Self {
        Self::new(DialogKind::Confirm, title, message)
    }

    /// A prompt: `message`, a field showing `text` with its caret at byte `caret`, and Cancel
    /// and OK buttons.
    #[must_use]
    pub fn prompt(
        title: impl Into<String>,
        message: impl Into<Cow<'a, str>>,
        text: impl Into<Cow<'a, str>>,
        caret: usize,
    ) -> Self {
        let mut dialog = Self::new(DialogKind::Prompt, title, message);
        let body = dialog.popup.content_mut();
        body.text = text.into();
        body.caret = caret;
        dialog
    }

    /// Dimmed text shown in an empty prompt field.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.popup.content_mut().placeholder = placeholder.into();
        self
    }

    /// The confirm button's label, `OK` by default.
    #[must_use]
    pub fn confirm_label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.popup.content_mut().confirm = label.into();
        self
    }

    /// The cancel button's label, `Cancel` by default.
    #[must_use]
    pub fn cancel_label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.popup.content_mut().cancel = label.into();
        self
    }

    /// Draws the confirm button in the theme's `fail` color, for actions that destroy work.
    #[must_use]
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.popup.content_mut().destructive = destructive;
        self
    }

    /// Panel width in columns, including the border.
    #[must_use]
    pub fn width(mut self, columns: u16) -> Self {
        self.popup.content_mut().width = columns.saturating_sub(2);
        self
    }

    /// Shows the focus kept in `state`, when its control is one this dialog has.
    #[must_use]
    pub fn state(mut self, state: &DialogState) -> Self {
        let body = self.popup.content_mut();
        if body.kind.order().contains(&state.focus()) {
            body.focus = state.focus();
        }
        self
    }

    /// Border line style.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.popup = self.popup.border_type(border_type);
        self
    }

    /// How far the screen behind fades, from `0.0` (not at all) to `1.0`; `0.55` by default.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32) -> Self {
        self.popup = self.popup.backdrop(alpha);
        self
    }

    /// The kind of dialog.
    #[must_use]
    pub fn kind(&self) -> DialogKind {
        self.popup.content().kind
    }

    /// The cells the panel covers inside `area`.
    #[must_use]
    pub fn area(&self, area: Rect) -> Rect {
        self.popup.area(area)
    }

    /// The control drawn at `(column, row)` inside `area`, for [`DialogState::click`].
    #[must_use]
    pub fn control_at(&self, area: Rect, column: u16, row: u16) -> Option<DialogFocus> {
        let inner = inside(self.area(area));
        let body = self.popup.content();
        let layout = body.layout(inner);
        if layout
            .input
            .is_some_and(|field| contains(field, column, row))
        {
            return Some(DialogFocus::Input);
        }
        body.buttons(layout.buttons)
            .find(|(_, cells)| contains(*cells, column, row))
            .map(|(focus, _)| focus)
    }
}

impl Widget for Dialog<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.popup.render(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Layout {
    message: Rect,
    input: Option<Rect>,
    buttons: Rect,
}

impl Body<'_> {
    fn message_rows(&self, columns: u16) -> u16 {
        let lines = Wrapped::new(&self.message, columns).count();
        u16::try_from(lines).unwrap_or(u16::MAX)
    }

    fn layout(&self, inner: Rect) -> Layout {
        let columns = inner.width.saturating_sub(2 * SIDE);
        let x = inner.x.saturating_add(SIDE.min(inner.width));
        let rows = self.message_rows(columns);
        let message = Rect::new(x, inner.y.saturating_add(1), columns, rows);
        let below = message.y.saturating_add(rows).saturating_add(1);
        let input =
            matches!(self.kind, DialogKind::Prompt).then(|| Rect::new(x, below, columns, 1));
        let buttons_y = match input {
            Some(field) => field.y.saturating_add(2),
            None => below,
        };
        let bottom = inner.y.saturating_add(inner.height);
        Layout {
            message: Rect::new(
                message.x,
                message.y,
                message.width,
                rows.min(bottom.saturating_sub(message.y)),
            ),
            input: input.filter(|field| field.y < bottom),
            buttons: Rect::new(x, buttons_y, columns, u16::from(buttons_y < bottom)),
        }
    }

    fn buttons(&self, row: Rect) -> impl Iterator<Item = (DialogFocus, Rect)> + '_ {
        let labels = match self.kind {
            DialogKind::Alert => &[DialogFocus::Confirm][..],
            DialogKind::Confirm | DialogKind::Prompt => {
                &[DialogFocus::Cancel, DialogFocus::Confirm]
            }
        };
        let label = move |focus: DialogFocus| match focus {
            DialogFocus::Cancel => self.cancel.as_ref(),
            DialogFocus::Input | DialogFocus::Confirm => self.confirm.as_ref(),
        };
        let size = move |focus: DialogFocus| {
            u16::try_from(width(label(focus)))
                .unwrap_or(u16::MAX)
                .saturating_add(2 * BUTTON_PAD)
        };
        let total = labels
            .iter()
            .map(|focus| size(*focus))
            .fold(0_u16, u16::saturating_add)
            .saturating_add(BUTTON_GAP * u16::try_from(labels.len() - 1).unwrap_or(0));
        let mut x = row
            .x
            .saturating_add(row.width.saturating_sub(total))
            .max(row.x);
        let end = row.x.saturating_add(row.width);
        labels.iter().map(move |focus| {
            let cells = size(*focus).min(end.saturating_sub(x));
            let at = Rect::new(x, row.y, cells, row.height);
            x = x.saturating_add(cells).saturating_add(BUTTON_GAP).min(end);
            (*focus, at)
        })
    }

    fn render_field(&self, field: Rect, cx: &mut RenderCx<'_>) {
        let focused = self.focus == DialogFocus::Input;
        let surface = cx.token(Token::Surface0);
        let line = Style::new().fg(cx.token(Token::TextPrimary)).bg(surface);
        cx.fill(field, line, ' ');
        let text = Rect::new(
            field.x.saturating_add(1.min(field.width)),
            field.y,
            field.width.saturating_sub(2),
            1,
        );
        let accent = if focused {
            cx.token(Token::FocusRing)
        } else {
            cx.token(Token::Accent)
        };
        cx.put_str(field.x, field.y, "▏", line.fg(accent));
        if self.text.is_empty() {
            let dim = line.with_attrs(Attrs::new().set_dim(true));
            cx.put_str(
                text.x,
                text.y,
                clip_display_prefix(&self.placeholder, text.width),
                dim,
            );
        } else {
            let caret = caret_index(&self.text, self.caret);
            let start = scroll_start(&self.text, caret, text.width);
            let shown = clip_display_prefix(&self.text[start..], text.width);
            cx.put_str(text.x, text.y, shown, line);
        }
        if focused && text.width > 0 {
            let caret = caret_index(&self.text, self.caret);
            let start = scroll_start(&self.text, caret, text.width);
            let offset = u16::try_from(width(&self.text[start..caret])).unwrap_or(u16::MAX);
            let at = text.x.saturating_add(offset);
            if at < text.x.saturating_add(text.width) {
                let shown = if self.text.is_empty() {
                    self.placeholder.as_ref()
                } else {
                    &self.text[caret..]
                };
                let under = shown.chars().next().unwrap_or(' ');
                let inverted = Style::new().fg(surface).bg(cx.token(Token::FocusRing));
                cx.put_str(at, text.y, under.encode_utf8(&mut [0; 4]), inverted);
            }
        }
    }

    fn render_button(&self, focus: DialogFocus, cells: Rect, cx: &mut RenderCx<'_>) {
        if cells.width == 0 || cells.height == 0 {
            return;
        }
        let danger = self.destructive && focus == DialogFocus::Confirm;
        let tone = if danger {
            cx.token(Token::Fail)
        } else {
            cx.token(Token::FocusRing)
        };
        let surface = cx.token(Token::Surface0);
        let style = if self.focus == focus {
            Style::new()
                .fg(surface)
                .bg(tone)
                .with_attrs(Attrs::new().set_bold(true))
        } else if danger {
            Style::new().fg(tone).bg(surface)
        } else {
            Style::new().fg(cx.token(Token::TextPrimary)).bg(surface)
        };
        cx.fill(cells, style, ' ');
        let label = match focus {
            DialogFocus::Cancel => self.cancel.as_ref(),
            DialogFocus::Input | DialogFocus::Confirm => self.confirm.as_ref(),
        };
        let room = cells.width.saturating_sub(2 * BUTTON_PAD);
        cx.put_str(
            cells.x.saturating_add(BUTTON_PAD.min(cells.width)),
            cells.y,
            clip_display_prefix(label, room),
            style,
        );
    }
}

impl Widget for Body<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let panel = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1));
        let layout = self.layout(area);
        let message = layout.message;
        for (row, line) in Wrapped::new(&self.message, message.width)
            .take(usize::from(message.height))
            .enumerate()
        {
            let y = message
                .y
                .saturating_add(u16::try_from(row).unwrap_or(u16::MAX));
            cx.put_str(
                message.x,
                y,
                clip_display_prefix(line, message.width),
                panel,
            );
        }
        if let Some(field) = layout.input {
            self.render_field(field, cx);
        }
        for (focus, cells) in self.buttons(layout.buttons) {
            self.render_button(focus, cells, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let columns = self.width.min(avail.width);
        let rows = self.message_rows(columns.saturating_sub(2 * SIDE));
        let field = if matches!(self.kind, DialogKind::Prompt) {
            2
        } else {
            0
        };
        Size::new(columns, rows.saturating_add(field).saturating_add(4))
    }
}

fn inside(panel: Rect) -> Rect {
    Rect::new(
        panel.x.saturating_add(1),
        panel.y.saturating_add(1),
        panel.width.saturating_sub(2),
        panel.height.saturating_sub(2),
    )
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column - area.x < area.width && row >= area.y && row - area.y < area.height
}

fn caret_index(text: &str, caret: usize) -> usize {
    let mut index = caret.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn scroll_start(text: &str, caret: usize, columns: u16) -> usize {
    let room = usize::from(columns.max(1)) - 1;
    let mut start = 0;
    let mut chars = text[..caret].char_indices();
    while width(&text[start..caret]) > room {
        match chars.next() {
            Some((_, ch)) => start += ch.len_utf8(),
            None => break,
        }
    }
    start
}

struct Wrapped<'t> {
    rest: &'t str,
    columns: u16,
    done: bool,
}

impl<'t> Wrapped<'t> {
    fn new(text: &'t str, columns: u16) -> Self {
        Self {
            rest: text,
            columns,
            done: columns == 0 || text.is_empty(),
        }
    }
}

impl<'t> Iterator for Wrapped<'t> {
    type Item = &'t str;

    fn next(&mut self) -> Option<&'t str> {
        if self.done {
            return None;
        }
        let (paragraph, after) = match self.rest.find('\n') {
            Some(at) => (&self.rest[..at], Some(&self.rest[at + 1..])),
            None => (self.rest, None),
        };
        let max = usize::from(self.columns);
        let mut end = 0;
        let mut taken = 0;
        for (at, word) in paragraph.split(' ').scan(0, |at, word| {
            let start = *at;
            *at += word.len() + 1;
            Some((start, word))
        }) {
            let word_end = at + word.len();
            let wanted = width(&paragraph[..word_end]);
            if wanted <= max {
                end = word_end;
                taken = at + word.len();
                continue;
            }
            if end == 0 {
                let cut = clip_display_prefix(paragraph, self.columns).len().max(
                    paragraph
                        .chars()
                        .next()
                        .map_or(paragraph.len(), char::len_utf8),
                );
                self.rest = &self.rest[cut..];
                return Some(&paragraph[..cut]);
            }
            let line = paragraph[..end].trim_end();
            let next = paragraph[taken..].trim_start_matches(' ');
            let skipped = paragraph.len() - next.len();
            self.rest = &self.rest[skipped..];
            return Some(line);
        }
        match after {
            Some(after) => self.rest = after,
            None => self.done = true,
        }
        Some(paragraph.trim_end())
    }
}
