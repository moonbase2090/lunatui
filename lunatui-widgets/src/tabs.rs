// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};

use crate::{RenderCx, Widget};

/// Horizontal tab strip.
///
/// By default the strip is one row of titles. [`Tabs::boxed`] switches to a
/// three-row strip where the selected tab sits in a rounded box that opens
/// into a divider line below the titles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tabs {
    titles: Vec<String>,
    selected: usize,
    style: Style,
    highlight: Style,
    indent: u16,
    boxed: Option<BoxedStyle>,
    join_parent_border: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoxedStyle {
    frame: Style,
    divider: Style,
}

impl Tabs {
    /// Tab labels in order.
    #[must_use]
    pub fn new(titles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            titles: titles.into_iter().map(Into::into).collect(),
            selected: 0,
            style: Style::new(),
            highlight: Style::new(),
            indent: 0,
            boxed: None,
            join_parent_border: false,
        }
    }

    /// Selected tab index.
    #[must_use]
    pub fn select(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Unselected tab style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Selected tab style.
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = style;
        self
    }

    /// Blank cells before the first tab.
    #[must_use]
    pub fn indent(mut self, cells: u16) -> Self {
        self.indent = cells;
        self
    }

    /// Draws a three-row strip: a rounded box around the selected title, the
    /// titles, and a divider line that the box opens into.
    ///
    /// `frame` styles the box and `divider` styles the line. Each title gets
    /// one blank cell of padding on each side, and tabs are one cell apart.
    #[must_use]
    pub fn boxed(mut self, frame: Style, divider: Style) -> Self {
        self.boxed = Some(BoxedStyle { frame, divider });
        self
    }

    /// Treats the area's first and last columns as a parent block's side
    /// borders. In boxed mode the divider ends in `├` and `┤` there, and the
    /// rows above it leave those columns untouched.
    #[must_use]
    pub fn join_parent_border(mut self, join: bool) -> Self {
        self.join_parent_border = join;
        self
    }

    /// Where each visible tab sits when the strip is rendered into `area`, as `(index, rect)`
    /// pairs in tab order, for hit testing.
    ///
    /// A plain strip gives each title's own cells in the title row. A boxed strip gives each tab's
    /// slot (title, padding, and box edges) over the box row and the title row. Tabs that do not
    /// fit are left out, and the last tab that is cut off by the edge gets only the cells that show.
    #[must_use]
    pub fn tab_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        if area.width == 0 || area.height == 0 {
            return Vec::new();
        }
        match self.boxed {
            Some(_) if area.height >= 3 => self.boxed_areas(area),
            _ => self.plain_areas(area),
        }
    }

    fn plain_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        let end = area.x + area.width;
        let mut x = area.x.saturating_add(self.indent).min(end);
        let mut areas = Vec::new();
        for (index, title) in self.titles.iter().enumerate() {
            let remaining = end - x;
            let draw = clip_display_prefix(title, remaining);
            let w = u16::try_from(width(draw)).unwrap_or(remaining);
            if w > 0 {
                areas.push((index, Rect::new(x, area.y, w, 1)));
            }
            x = x.saturating_add(w).saturating_add(1).min(end);
        }
        areas
    }

    fn boxed_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        let edge = u16::from(self.join_parent_border && area.width >= 2);
        let span_start = area.x + edge;
        let span_end = area.x + area.width - edge;
        let mut areas = Vec::new();
        if span_start >= span_end {
            return areas;
        }
        let mut x = span_start.saturating_add(self.indent);
        for (index, title) in self.titles.iter().enumerate() {
            if x >= span_end {
                break;
            }
            let title_width = u16::try_from(width(title)).unwrap_or(u16::MAX);
            let slot = title_width.saturating_add(4);
            let room = span_end - x;
            if slot > room {
                areas.push((index, Rect::new(x, area.y, room, 2)));
                break;
            }
            areas.push((index, Rect::new(x, area.y, slot, 2)));
            x = (x + slot - 1).saturating_add(2);
        }
        areas
    }

    fn render_plain(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let end = area.x + area.width;
        let mut x = area.x.saturating_add(self.indent).min(end);
        for (index, title) in self.titles.iter().enumerate() {
            if x >= end {
                break;
            }
            let remaining = end - x;
            let draw = clip_display_prefix(title, remaining);
            let w = u16::try_from(width(draw)).unwrap_or(remaining);
            let style = if index == self.selected {
                self.highlight
            } else {
                self.style
            };
            cx.put_str(x, area.y, draw, style);
            x = x.saturating_add(w);
            if index + 1 < self.titles.len() && x < end {
                cx.put_str(x, area.y, " ", self.style);
                x = x.saturating_add(1);
            }
        }
    }

    fn render_boxed(&self, boxed: BoxedStyle, area: Rect, cx: &mut RenderCx<'_>) {
        let (top, labels, divider) = (area.y, area.y + 1, area.y + 2);
        let edge = u16::from(self.join_parent_border && area.width >= 2);
        let span_start = area.x + edge;
        let span_end = area.x + area.width - edge;
        if span_start >= span_end {
            return;
        }
        let span = usize::from(span_end - span_start);
        let blank = " ".repeat(span);
        cx.put_str(span_start, top, &blank, self.style);
        cx.put_str(span_start, labels, &blank, self.style);
        cx.put_str(span_start, divider, &"─".repeat(span), boxed.divider);
        if edge == 1 {
            cx.put_str(area.x, divider, "├", boxed.divider);
            cx.put_str(span_end, divider, "┤", boxed.divider);
        }
        let mut x = span_start.saturating_add(self.indent);
        for (index, title) in self.titles.iter().enumerate() {
            if x >= span_end {
                break;
            }
            let title_width = u16::try_from(width(title)).unwrap_or(u16::MAX);
            let slot = title_width.saturating_add(4);
            let selected = index == self.selected;
            let room = span_end - x;
            if slot > room {
                self.render_clipped_slot(boxed, index, (x, area.y, room), cx);
                break;
            }
            let right = x + slot - 1;
            let label = format!(" {title} ");
            if selected {
                let bar = "─".repeat(usize::from(slot - 2));
                cx.put_str(x, top, &format!("╭{bar}╮"), boxed.frame);
                cx.put_str(x, labels, "│", boxed.frame);
                cx.put_str(x + 1, labels, &label, self.highlight);
                cx.put_str(right, labels, "│", boxed.frame);
                cx.put_str(x, divider, "╯", boxed.frame);
                cx.put_str(
                    x + 1,
                    divider,
                    &" ".repeat(usize::from(slot - 2)),
                    self.style,
                );
                cx.put_str(right, divider, "╰", boxed.frame);
            } else {
                cx.put_str(x + 1, labels, &label, self.style);
            }
            x = right.saturating_add(2);
        }
    }

    fn render_clipped_slot(
        &self,
        boxed: BoxedStyle,
        index: usize,
        at: (u16, u16, u16),
        cx: &mut RenderCx<'_>,
    ) {
        let (x, top, room) = at;
        let label = format!(" {} ", self.titles[index]);
        let shown = clip_display_prefix(&label, room.saturating_sub(1));
        if index != self.selected {
            cx.put_str(x + 1, top + 1, shown, self.style);
            return;
        }
        let inner = usize::from(room - 1);
        cx.put_str(x, top, &format!("╭{}", "─".repeat(inner)), boxed.frame);
        cx.put_str(x, top + 1, "│", boxed.frame);
        cx.put_str(x + 1, top + 1, shown, self.highlight);
        cx.put_str(x, top + 2, "╯", boxed.frame);
        cx.put_str(x + 1, top + 2, &" ".repeat(inner), self.style);
    }
}

impl Widget for Tabs {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        match self.boxed {
            Some(boxed) if area.height >= 3 => self.render_boxed(boxed, area, cx),
            _ => self.render_plain(area, cx),
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = if self.boxed.is_some() { 3 } else { 1 };
        Size::new(avail.width, avail.height.min(rows))
    }
}

/// A finished reorder: the tab at `from` moved to position `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabMove {
    /// Index the tab had before the move.
    pub from: usize,
    /// Index the tab has after the move.
    pub to: usize,
}

impl TabMove {
    /// Moves the item at `from` to `to` in `items`, shifting the ones between. Does nothing when
    /// either index is out of range.
    pub fn apply<T>(self, items: &mut Vec<T>) {
        if self.from < items.len() && self.to < items.len() {
            let item = items.remove(self.from);
            items.insert(self.to, item);
        }
    }

    /// Where the selected tab `selected` ends up after this move, so the selection stays on the
    /// same tab: the dragged tab keeps it, and tabs it passed shift by one.
    #[must_use]
    pub fn selection(self, selected: usize) -> usize {
        if selected == self.from {
            self.to
        } else if self.from < selected && selected <= self.to {
            selected - 1
        } else if self.to <= selected && selected < self.from {
            selected + 1
        } else {
            selected
        }
    }
}

/// Pointer-driven reordering of a [`Tabs`] strip.
///
/// Feed it the regions from [`Tabs::tab_areas`] and the pointer events: [`TabsDrag::press`] on
/// button down, [`TabsDrag::drag`] while the pointer moves, then [`TabsDrag::release`] on button up,
/// which returns the [`TabMove`] to apply. A press that never leaves its own tab is a click and
/// reorders nothing. Moving off the strip's rows, or [`TabsDrag::cancel`], abandons the drag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TabsDrag {
    from: Option<usize>,
    target: Option<usize>,
}

impl TabsDrag {
    /// Starts idle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a drag is in progress.
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.from.is_some()
    }

    /// Index of the tab being dragged.
    #[must_use]
    pub fn from(&self) -> Option<usize> {
        self.from
    }

    /// Index the dragged tab would take if released now, or `None` while the pointer is off the
    /// strip.
    #[must_use]
    pub fn target(&self) -> Option<usize> {
        self.target
    }

    /// Begins a drag when `(column, row)` is on a tab. Returns whether one began.
    pub fn press(&mut self, areas: &[(usize, Rect)], column: u16, row: u16) -> bool {
        self.from = areas
            .iter()
            .find(|(_, rect)| contains(*rect, column, row))
            .map(|(index, _)| *index);
        self.target = self.from;
        self.from.is_some()
    }

    /// Updates the drop target for the pointer at `(column, row)`. Left of the first tab picks the
    /// first, right of the last picks the last, the gap between two tabs keeps the current target,
    /// and a row outside the strip picks none.
    pub fn drag(&mut self, areas: &[(usize, Rect)], column: u16, row: u16) {
        if self.from.is_none() {
            return;
        }
        if !areas.iter().any(|(_, rect)| row_in(*rect, row)) {
            self.target = None;
            return;
        }
        let under = |rect: &Rect| column >= rect.x && column < rect.x + rect.width;
        if let Some((index, _)) = areas.first().filter(|(_, rect)| column < rect.x) {
            self.target = Some(*index);
        } else if let Some((index, _)) = areas
            .last()
            .filter(|(_, rect)| column >= rect.x + rect.width)
        {
            self.target = Some(*index);
        } else if let Some((index, _)) = areas.iter().find(|(_, rect)| under(rect)) {
            self.target = Some(*index);
        } else if self.target.is_none() {
            self.target = self.from;
        }
    }

    /// Ends the drag and returns the reorder to apply, if the tab was dropped on another tab.
    pub fn release(&mut self) -> Option<TabMove> {
        let from = self.from.take()?;
        let to = self.target.take()?;
        (from != to).then_some(TabMove { from, to })
    }

    /// Abandons the drag without a reorder.
    pub fn cancel(&mut self) {
        self.from = None;
        self.target = None;
    }
}

fn row_in(rect: Rect, row: u16) -> bool {
    row >= rect.y && row < rect.y + rect.height
}

fn contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x && column < rect.x + rect.width && row_in(rect, row)
}
