// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget};

const MARKER: &str = " *";
const COLUMN_GAP: u16 = 2;

/// One labelled row of a [`Form`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormField {
    label: String,
    required: bool,
    disabled: bool,
    hint: Option<String>,
    error: Option<String>,
    input_rows: u16,
}

impl FormField {
    /// A field with this label, one input row, and no hint or error.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            required: false,
            disabled: false,
            hint: None,
            error: None,
            input_rows: 1,
        }
    }

    /// Marks the field as required: its label gets a `*`.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Dims the field and takes it out of the tab order.
    #[must_use]
    pub const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Dim help text under the input, shown while there is no error.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// A validation message under the input, in the fail color. It replaces the hint.
    #[must_use]
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Rows the input needs, at least one.
    #[must_use]
    pub fn input_rows(mut self, rows: u16) -> Self {
        self.input_rows = rows.max(1);
        self
    }

    /// The validation message, if there is one.
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Where one field's parts sit in a [`FormLayout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldArea {
    /// Index of the field.
    pub index: usize,
    /// The whole row group: label, input, and message.
    pub row: Rect,
    /// The label column cells.
    pub label: Rect,
    /// The cells for the caller's input widget.
    pub input: Rect,
    /// The one-row message under the input.
    pub message: Rect,
}

/// Labelled fields with a tab order, required markers, hints, and validation messages.
///
/// The form lays the fields out and draws everything except the inputs: each field's label, its
/// `*` when required, and the hint or error under it. The caller draws its own widgets (a text
/// input, a toggle, a radio group) into the [`FieldArea::input`] rectangles from
/// [`Form::layout`]. [`Form::step`] moves focus through the fields in tab order, wrapping around
/// and skipping disabled fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    fields: Vec<FormField>,
    focus: Option<usize>,
    gap: u16,
    label_width: Option<u16>,
}

impl Form {
    /// A form with these fields.
    #[must_use]
    pub fn new(fields: impl IntoIterator<Item = FormField>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
            focus: None,
            gap: 1,
            label_width: None,
        }
    }

    /// The field that has input focus.
    #[must_use]
    pub const fn focus(mut self, index: Option<usize>) -> Self {
        self.focus = index;
        self
    }

    /// Blank rows between fields. The default is one.
    #[must_use]
    pub const fn gap(mut self, rows: u16) -> Self {
        self.gap = rows;
        self
    }

    /// Width of the label column. By default it fits the longest label and its marker.
    #[must_use]
    pub const fn label_width(mut self, cells: u16) -> Self {
        self.label_width = Some(cells);
        self
    }

    /// Number of fields.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether the form has no fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Sets "Required" on every required field whose value is empty or only blanks, keeping any
    /// error already set. `values` has one entry per field, in order.
    #[must_use]
    pub fn check_required(mut self, values: &[&str]) -> Self {
        for (field, value) in self.fields.iter_mut().zip(values) {
            if field.required && value.trim().is_empty() && field.error.is_none() {
                field.error = Some(String::from("Required"));
            }
        }
        self
    }

    /// Whether no field has a validation message.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.fields.iter().all(|field| field.error.is_none())
    }

    /// The first field with a validation message, for moving focus to it.
    #[must_use]
    pub fn first_error(&self) -> Option<usize> {
        self.fields.iter().position(|field| field.error.is_some())
    }

    /// The field after (or before) `from` in tab order, wrapping around and skipping disabled
    /// fields. From `None`, forward gives the first enabled field and backward the last. `None`
    /// when no field can take focus.
    #[must_use]
    pub fn step(&self, from: Option<usize>, forward: bool) -> Option<usize> {
        let count = self.fields.len();
        if count == 0 {
            return None;
        }
        let start = match (from.filter(|index| *index < count), forward) {
            (Some(index), true) => index + 1,
            (Some(index), false) => index + count - 1,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        (0..count)
            .map(|offset| {
                if forward {
                    (start + offset) % count
                } else {
                    (start + count - offset) % count
                }
            })
            .find(|index| !self.fields[*index].disabled)
    }

    fn label_column(&self) -> u16 {
        self.label_width.unwrap_or_else(|| {
            self.fields
                .iter()
                .map(|field| {
                    let marker = if field.required { width(MARKER) } else { 0 };
                    u16::try_from(width(&field.label) + marker).unwrap_or(u16::MAX)
                })
                .max()
                .unwrap_or(0)
        })
    }

    /// Where each field's parts sit when the form fills `area`, in order. Fields that do not fit
    /// in the height are left out, and the input column keeps at least one cell.
    #[must_use]
    pub fn layout(&self, area: Rect) -> Vec<FieldArea> {
        let mut areas = Vec::new();
        let end_y = area.y.saturating_add(area.height);
        let label_w = self.label_column().min(area.width.saturating_sub(1));
        let input_x = area.x.saturating_add(label_w).saturating_add(COLUMN_GAP);
        let input_w = area
            .x
            .saturating_add(area.width)
            .saturating_sub(input_x)
            .max(1);
        let mut y = area.y;
        for (index, field) in self.fields.iter().enumerate() {
            let rows = field.input_rows.saturating_add(1);
            if area.width == 0 || y.saturating_add(rows) > end_y {
                break;
            }
            areas.push(FieldArea {
                index,
                row: Rect::new(area.x, y, area.width, rows),
                label: Rect::new(area.x, y, label_w, 1),
                input: Rect::new(input_x, y, input_w, field.input_rows),
                message: Rect::new(input_x, y + field.input_rows, input_w, 1),
            });
            y = y.saturating_add(rows).saturating_add(self.gap);
        }
        areas
    }

    fn field_styles(&self, cx: &RenderCx<'_>, index: usize) -> (Style, Style) {
        let field = &self.fields[index];
        let focused = self.focus == Some(index) && !field.disabled;
        let attrs = Attrs::new().set_dim(field.disabled).set_bold(focused);
        let color = if focused {
            cx.token(Token::Accent)
        } else {
            cx.token(Token::TextPrimary)
        };
        (
            Style::new().fg(color).with_attrs(attrs),
            Style::new()
                .fg(cx.token(Token::Fail))
                .with_attrs(Attrs::new().set_dim(field.disabled)),
        )
    }
}

impl Widget for Form {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        for field_area in self.layout(area) {
            let field = &self.fields[field_area.index];
            let (label_style, error_style) = self.field_styles(cx, field_area.index);
            let label_room = field_area.label.width;
            cx.put_str(
                field_area.label.x,
                field_area.label.y,
                clip_display_prefix(&field.label, label_room),
                label_style,
            );
            if field.required {
                let at = field_area.label.x
                    + u16::try_from(width(&field.label))
                        .unwrap_or(u16::MAX)
                        .min(label_room);
                let room = (field_area.label.x + label_room).saturating_sub(at);
                cx.put_str(
                    at,
                    field_area.label.y,
                    clip_display_prefix(MARKER, room),
                    error_style,
                );
            }
            let message = field_area.message;
            let (text, style) = match (&field.error, &field.hint) {
                (Some(error), _) => (format!("✖ {error}"), error_style),
                (None, Some(hint)) => (
                    hint.clone(),
                    Style::new()
                        .fg(cx.token(Token::TextPrimary))
                        .with_attrs(Attrs::new().set_dim(true)),
                ),
                (None, None) => continue,
            };
            cx.put_str(
                message.x,
                message.y,
                clip_display_prefix(&text, message.width),
                style,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let areas = self.layout(Rect::new(0, 0, avail.width, avail.height));
        let bottom = areas
            .last()
            .map_or(0, |field| field.row.y + field.row.height);
        let width = if areas.is_empty() { 0 } else { avail.width };
        Size::new(width, bottom)
    }
}
