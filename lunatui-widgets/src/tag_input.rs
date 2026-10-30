// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const CHIP_CHROME: u16 = 4;
const MIN_FIELD: u16 = 8;
const REMOVE: &str = "×";

#[derive(Clone, Copy)]
struct Slot {
    index: Option<usize>,
    x: u16,
    row: u16,
    width: u16,
}

/// Tags drawn as chips that wrap across rows, followed by a text field for the next one.
///
/// The app owns the tags, the text being typed, the caret, and which chip is
/// [`selected`](TagInput::selected) for removal. Each chip shows its label and a `×`;
/// [`tag_at`](TagInput::tag_at), [`remove_at`](TagInput::remove_at), and
/// [`field_area`](TagInput::field_area) are for hit testing. When the chips need more rows than
/// the area has, the last rows are shown so the field stays in view; [`measure`](Widget::measure)
/// reports the rows needed. [`commit`](TagInput::commit) turns the typed text into a tag and
/// [`step`](TagInput::step) moves the selection between the chips and the field.
#[derive(Clone, Debug, PartialEq)]
pub struct TagInput<'a, S = &'a str> {
    tags: &'a [S],
    text: Cow<'a, str>,
    caret: usize,
    selected: Option<usize>,
    placeholder: Cow<'a, str>,
    state: WidgetState,
}

impl<'a, S: AsRef<str>> TagInput<'a, S> {
    /// Chips for `tags` and a field showing `text` with the caret at byte index `caret` (moved
    /// back to a character boundary, and to the end when past it).
    #[must_use]
    pub fn new(tags: &'a [S], text: impl Into<Cow<'a, str>>, caret: usize) -> Self {
        Self {
            tags,
            text: text.into(),
            caret,
            selected: None,
            placeholder: Cow::Borrowed(""),
            state: WidgetState::Normal,
        }
    }

    /// Dim text shown while there are no tags and the field is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// The chip highlighted for removal. With none, the caret shows in the field instead.
    #[must_use]
    pub const fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Interaction state: focused draws the caret or the selected chip in the accent color,
    /// disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Moves the selection one step: `back` goes toward the first chip (from the field, to the
    /// last chip), otherwise toward the field (from the last chip, to the field, shown as `None`).
    /// It stops at the first chip.
    #[must_use]
    pub fn step(selected: Option<usize>, count: usize, back: bool) -> Option<usize> {
        match (selected, back) {
            (None, true) => count.checked_sub(1),
            (None, false) => None,
            (Some(index), true) => Some(index.saturating_sub(1)),
            (Some(index), false) => Some(index + 1).filter(|next| *next < count),
        }
    }

    /// Turns the typed text into a tag: trims it, and adds it unless it is empty or already a tag.
    /// Clears `text` either way and returns whether a tag was added.
    pub fn commit(tags: &mut Vec<String>, text: &mut String) -> bool {
        let label = text.trim().to_owned();
        text.clear();
        if label.is_empty() || tags.contains(&label) {
            return false;
        }
        tags.push(label);
        true
    }

    fn chip_width(label: &str, total: u16) -> u16 {
        let wanted = u16::try_from(width(label))
            .unwrap_or(u16::MAX)
            .saturating_add(CHIP_CHROME);
        wanted.min(total)
    }

    fn layout(&self, total: u16, mut visit: impl FnMut(Slot)) -> u16 {
        let mut x = 0_u16;
        let mut row = 0_u16;
        for (index, tag) in self.tags.iter().enumerate() {
            let chip = Self::chip_width(tag.as_ref(), total);
            if x > 0 && x.saturating_add(chip) > total {
                row += 1;
                x = 0;
            }
            visit(Slot {
                index: Some(index),
                x,
                row,
                width: chip,
            });
            x = x.saturating_add(chip).saturating_add(1);
        }
        if x > 0 && x.saturating_add(MIN_FIELD) > total {
            row += 1;
            x = 0;
        }
        visit(Slot {
            index: None,
            x,
            row,
            width: total.saturating_sub(x),
        });
        row + 1
    }

    /// The rows the chips and the field need in a width of `columns`.
    #[must_use]
    pub fn rows(&self, columns: u16) -> u16 {
        if columns == 0 {
            return 0;
        }
        self.layout(columns, |_| {})
    }

    fn hidden_rows(&self, area: Rect) -> u16 {
        self.rows(area.width).saturating_sub(area.height)
    }

    fn cell(&self, area: Rect, slot: Slot) -> Option<Rect> {
        let row = slot.row.checked_sub(self.hidden_rows(area))?;
        (row < area.height).then(|| Rect::new(area.x + slot.x, area.y + row, slot.width, 1))
    }

    /// The cells of the text field in `area`, or an empty rectangle when it is scrolled out.
    #[must_use]
    pub fn field_area(&self, area: Rect) -> Rect {
        let mut found = Rect::new(area.x, area.y, 0, 0);
        if area.width == 0 || area.height == 0 {
            return found;
        }
        self.layout(area.width, |slot| {
            if slot.index.is_none() {
                found = self.cell(area, slot).unwrap_or(found);
            }
        });
        found
    }

    fn chip_at(&self, area: Rect, x: u16, y: u16) -> Option<(usize, Rect)> {
        if area.width == 0 || area.height == 0 {
            return None;
        }
        let mut hit = None;
        self.layout(area.width, |slot| {
            let (Some(index), Some(cell)) = (slot.index, self.cell(area, slot)) else {
                return;
            };
            if y == cell.y && x >= cell.x && x - cell.x < cell.width {
                hit = Some((index, cell));
            }
        });
        hit
    }

    /// The chip at (`x`, `y`), if any.
    #[must_use]
    pub fn tag_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        self.chip_at(area, x, y).map(|(index, _)| index)
    }

    /// The chip whose `×` is at (`x`, `y`), if any.
    #[must_use]
    pub fn remove_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let (index, cell) = self.chip_at(area, x, y)?;
        (cell.width >= CHIP_CHROME && x == cell.x + cell.width - 2).then_some(index)
    }

    fn caret_index(&self) -> usize {
        let mut index = self.caret.min(self.text.len());
        while !self.text.is_char_boundary(index) {
            index -= 1;
        }
        index
    }

    fn render_chip(&self, label: &str, index: usize, cell: Rect, cx: &mut RenderCx<'_>) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let surface = cx.token(Token::Surface1);
        let current = self.selected == Some(index);
        let style = if current {
            Style::new().fg(surface).bg(cx.token(Token::Accent))
        } else {
            Style::new().fg(cx.token(Token::TextPrimary)).bg(surface)
        }
        .with_attrs(Attrs::new().set_dim(disabled));
        cx.fill(cell, style, ' ');
        let room = cell.width.saturating_sub(CHIP_CHROME);
        cx.put_str(cell.x + 1, cell.y, clip_display_prefix(label, room), style);
        if cell.width >= CHIP_CHROME {
            let cross = style.with_attrs(Attrs::new().set_dim(true));
            cx.put_str(cell.x + cell.width - 2, cell.y, REMOVE, cross);
        }
    }

    fn render_field(&self, field: Rect, cx: &mut RenderCx<'_>) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused) && self.selected.is_none();
        let surface = cx.token(Token::Surface0);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        cx.fill(field, base, ' ');
        if self.text.is_empty() {
            if self.tags.is_empty() {
                let dim = base.with_attrs(Attrs::new().set_dim(true));
                let shown = clip_display_prefix(&self.placeholder, field.width);
                cx.put_str(field.x, field.y, shown, dim);
            }
        } else {
            cx.put_str(
                field.x,
                field.y,
                clip_display_prefix(&self.text, field.width),
                base,
            );
        }
        if focused {
            let caret = self.caret_index();
            let offset = u16::try_from(width(&self.text[..caret])).unwrap_or(u16::MAX);
            let at = field.x.saturating_add(offset);
            if at < field.x + field.width {
                let under = self.text[caret..].chars().next().unwrap_or(' ');
                let inverted = Style::new().fg(surface).bg(cx.token(Token::Accent));
                cx.put_str(at, field.y, under.encode_utf8(&mut [0; 4]), inverted);
            }
        }
    }
}

impl<S: AsRef<str>> Widget for TagInput<'_, S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let surface = cx.token(Token::Surface0);
        let blank = Style::new().fg(cx.token(Token::TextPrimary)).bg(surface);
        cx.fill(area, blank, ' ');
        self.layout(area.width, |slot| {
            let Some(cell) = self.cell(area, slot) else {
                return;
            };
            match slot.index {
                Some(index) => {
                    let label = self.tags[index].as_ref();
                    self.render_chip(label, index, cell, cx);
                }
                None => self.render_field(cell, cx),
            }
        });
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, self.rows(avail.width).min(avail.height))
    }
}
