// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::Field;
use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_runtime::{HitId, RovingFocus};
use lunatui_ui_tokens::Role;
use lunatui_widgets::{RenderCx, Widget};
use std::borrow::Cow;

const MARKER: &str = " *";
const COLUMN_GAP: u16 = 2;

/// Where one field's parts sit in a [`Form::layout`].
///
/// Downstream code cannot list every field without `..`, because fields can be added:
///
/// ```compile_fail,E0638
/// use lunatui_field::FieldArea;
///
/// fn destructures(value: FieldArea) {
///     let FieldArea { index: _, row: _, label: _, input: _, message: _ } = value;
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
/// Each field's [`Field::body`] is rendered into its input rectangle. Callers
/// can instead draw their own inputs into the rectangles from [`Form::layout`].
/// [`FormState`](crate::FormState) moves focus through the fields with Tab and Shift+Tab, and
/// [`Form::step`] is the same move as a pure function; both wrap and skip disabled fields.
///
/// # Panics
///
/// [`Form::new`] panics if a field's selected accessible name is empty or
/// whitespace, even when the form will be drawn without a hit map. Set a
/// nonblank [`Field::name`] for blank labels before constructing the form.
/// Rendering with a hit map also requires each field to have an ID, supplied by
/// [`Field::id`] or [`Form::first_id`]. Rendering panics if the base ID plus a
/// field index overflows `u32`.
#[derive(Clone)]
pub struct Form<'a> {
    fields: Vec<Field<'a>>,
    first_id: Option<HitId>,
    focus: Option<usize>,
    gap: u16,
    label_width: Option<u16>,
}

impl<'a> Form<'a> {
    /// A form with these fields.
    ///
    /// # Panics
    ///
    /// Panics if a field's selected accessible name is empty or whitespace.
    /// An explicit [`Field::name`] overrides the label. This validation happens
    /// at construction in every build profile, including forms that will be
    /// drawn without an attached hit map.
    #[must_use]
    pub fn new(fields: impl IntoIterator<Item = Field<'a>>) -> Self {
        let fields: Vec<_> = fields.into_iter().collect();
        for field in &fields {
            let _ = field.accessible_name();
        }
        Self {
            fields,
            first_id: None,
            focus: None,
            gap: 1,
            label_width: None,
        }
    }

    /// Base identity for fields that do not supply their own ID. Each field's
    /// index is added to this value, independent of clipping and disabled state.
    #[must_use]
    pub const fn first_id(mut self, first: HitId) -> Self {
        self.first_id = Some(first);
        self
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
                field.error = Some(Cow::Borrowed("Required"));
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
        RovingFocus::step(from, self.fields.len(), forward, true, |index| {
            !self.fields[index].is_disabled()
        })
    }

    pub(crate) fn enabled(&self) -> impl Iterator<Item = bool> + '_ {
        self.fields.iter().map(|field| !field.is_disabled())
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
        let input_x = area
            .x
            .saturating_add(label_w)
            .saturating_add(COLUMN_GAP)
            .min(area.x.saturating_add(area.width.saturating_sub(1)));
        let input_w = area
            .x
            .saturating_add(area.width)
            .saturating_sub(input_x)
            .max(1);
        let mut y = area.y;
        for (index, field) in self.fields.iter().enumerate() {
            let input_rows = field.input_rows;
            let rows = input_rows.saturating_add(1);
            if area.width == 0 || y.saturating_add(rows) > end_y {
                break;
            }
            areas.push(FieldArea {
                index,
                row: Rect::new(area.x, y, area.width, rows),
                label: Rect::new(area.x, y, label_w, 1),
                input: Rect::new(input_x, y, input_w, input_rows),
                message: Rect::new(input_x, y + input_rows, input_w, 1),
            });
            y = y.saturating_add(rows).saturating_add(self.gap);
        }
        areas
    }

    fn field_styles(&self, cx: &RenderCx<'_>, index: usize) -> (Style, Style) {
        let field = &self.fields[index];
        let focused = self.focus == Some(index) && !field.is_disabled();
        let attrs = Attrs::new().set_dim(field.is_disabled()).set_bold(focused);
        let color = if focused {
            cx.token(Role::Accent.token())
        } else {
            cx.token(Role::Foreground.token())
        };
        (
            Style::new().fg(color).with_attrs(attrs),
            Style::new()
                .fg(cx.token(Role::Destructive.token()))
                .with_attrs(Attrs::new().set_dim(field.is_disabled())),
        )
    }
}

impl Widget for Form<'_> {
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
            let fallback = self.first_id.and_then(|first| {
                u32::try_from(field_area.index)
                    .ok()
                    .and_then(|offset| first.0.checked_add(offset))
                    .map(HitId)
            });
            assert!(
                self.first_id.is_none() || fallback.is_some(),
                "form control IDs overflow"
            );
            field.register(field_area.input, cx, fallback);
            field.render_body(field_area.input, cx);
            let message = field_area.message;
            let (text, style) = match (&field.error, field.help.as_ref()) {
                (Some(error), _) => (format!("✖ {error}"), error_style),
                (None, hint) if !hint.is_empty() => (
                    hint.to_owned(),
                    Style::new()
                        .fg(cx.token(Role::Foreground.token()))
                        .with_attrs(Attrs::new().set_dim(true)),
                ),
                (None, _) => continue,
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
