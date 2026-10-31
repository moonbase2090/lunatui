// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::cell::RefCell;
use std::ops::RangeInclusive;

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_layout::{Constraint, Flex, Layout};
use lunatui_text::source::{RowBuilder, RowSource};

use crate::render_cx::{clip_to_coordinates, patch_style};
use crate::table_state::{SortDirection, TableState};
use crate::{Alignment, RenderCx, Widget};

/// Style for one table cell, layered over its row and column style.
pub trait CellStyle {
    /// Style for the cell at source row `row` and column `column`.
    /// [`Color::Default`](lunatui_core::Color::Default) leaves that channel alone.
    fn style_for(&self, row: usize, column: usize) -> Style;
}

impl CellStyle for () {
    fn style_for(&self, _row: usize, _column: usize) -> Style {
        Style::new()
    }
}

impl<F: Fn(usize, usize) -> Style> CellStyle for F {
    fn style_for(&self, row: usize, column: usize) -> Style {
        self(row, column)
    }
}

impl CellStyle for Vec<Vec<Style>> {
    fn style_for(&self, row: usize, column: usize) -> Style {
        self.get(row)
            .and_then(|cells| cells.get(column))
            .copied()
            .unwrap_or_default()
    }
}

/// Column layout for [`Table`].
///
/// Every row of the area is cleared across its full width, including rows
/// below the last data row, so a second render leaves nothing behind. The selected row's highlight style
/// covers the whole row, gaps included.
///
/// [`state`](Self::state) applies a [`TableState`]: its scroll offset, cursor, selected range
/// (drawn in [`selection_style`](Self::selection_style)), sort marker (`▲` or `▼` after the
/// sorted column's title) and column widths. Cells are read through [`RowSource::cell`] when the
/// source answers it, and the solved columns are kept between renders of the same area, so a
/// built table over such a source draws without allocating.
#[derive(Clone, Debug)]
pub struct Table<S, C = ()> {
    source: S,
    columns: Vec<Constraint>,
    style: Style,
    highlight: Style,
    offset: usize,
    selected: Option<usize>,
    header: Option<Vec<(String, Style)>>,
    column_styles: Vec<Style>,
    alignments: Vec<Alignment>,
    cell_styles: C,
    highlight_symbol: Option<(&'static str, Style)>,
    odd_row: Option<Style>,
    row_gap: u16,
    selection: Option<(usize, usize)>,
    selection_style: Style,
    sort: Option<(usize, SortDirection)>,
    widths: Vec<Option<u16>>,
    solved: RefCell<Option<(Rect, Vec<Rect>)>>,
    buffer: RefCell<RowBuilder>,
}

impl<S: RowSource> Table<S> {
    /// Builds a table with `columns` resolved by the layout engine.
    #[must_use]
    pub fn new(source: S, columns: Vec<Constraint>) -> Self {
        Self {
            source,
            columns,
            style: Style::new(),
            highlight: Style::new(),
            offset: 0,
            selected: None,
            header: None,
            column_styles: Vec::new(),
            alignments: Vec::new(),
            cell_styles: (),
            highlight_symbol: None,
            odd_row: None,
            row_gap: 0,
            selection: None,
            selection_style: Style::new(),
            sort: None,
            widths: Vec::new(),
            solved: RefCell::new(None),
            buffer: RefCell::new(RowBuilder::default()),
        }
    }
}

impl<S: RowSource, C: CellStyle> Table<S, C> {
    /// Default cell style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style for the focused row. It replaces the row's other styles; cell
    /// styles are still layered on top.
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = style;
        self
    }

    /// First visible row index.
    #[must_use]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Focused row.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Header row drawn above the data, one title and style per column.
    #[must_use]
    pub fn header<T: Into<String>>(mut self, cells: impl IntoIterator<Item = (T, Style)>) -> Self {
        self.header = Some(
            cells
                .into_iter()
                .map(|(text, style)| (text.into(), style))
                .collect(),
        );
        self
    }

    /// Per-column styles layered over [`Table::style`]. Missing columns use
    /// the table style alone.
    #[must_use]
    pub fn column_styles(mut self, styles: Vec<Style>) -> Self {
        self.column_styles = styles;
        self
    }

    /// Per-column text alignment inside each column. Missing columns are left
    /// aligned.
    #[must_use]
    pub fn alignments(mut self, alignments: Vec<Alignment>) -> Self {
        self.alignments = alignments;
        self
    }

    /// Per-cell styles layered over the row and column styles.
    #[must_use]
    pub fn cell_styles<D: CellStyle>(self, cell_styles: D) -> Table<S, D> {
        Table {
            source: self.source,
            columns: self.columns,
            style: self.style,
            highlight: self.highlight,
            offset: self.offset,
            selected: self.selected,
            header: self.header,
            column_styles: self.column_styles,
            alignments: self.alignments,
            cell_styles,
            highlight_symbol: self.highlight_symbol,
            odd_row: self.odd_row,
            row_gap: self.row_gap,
            selection: self.selection,
            selection_style: self.selection_style,
            sort: self.sort,
            widths: self.widths,
            solved: self.solved,
            buffer: self.buffer,
        }
    }

    /// Marker drawn at the left edge of the focused row. Every row reserves
    /// its width, so the columns stay aligned.
    #[must_use]
    pub fn highlight_symbol(mut self, symbol: &'static str, style: Style) -> Self {
        self.highlight_symbol = Some((symbol, style));
        self
    }

    /// Style layered over every odd row (1, 3, 5, ...) across its full width,
    /// except the focused row.
    #[must_use]
    pub fn odd_row_style(mut self, style: Style) -> Self {
        self.odd_row = Some(style);
        self
    }

    /// Blank rows left between data rows.
    #[must_use]
    pub fn row_gap(mut self, rows: u16) -> Self {
        self.row_gap = rows;
        self
    }

    /// Rows drawn in [`selection_style`](Self::selection_style), besides the focused row.
    #[must_use]
    pub fn selection(mut self, rows: Option<RangeInclusive<usize>>) -> Self {
        self.selection = rows.map(|rows| (*rows.start(), *rows.end()));
        self
    }

    /// Style layered over the selected rows other than the focused one.
    #[must_use]
    pub fn selection_style(mut self, style: Style) -> Self {
        self.selection_style = style;
        self
    }

    /// Marks `column`'s header with `▲` or `▼` for its sort direction.
    #[must_use]
    pub fn sort_indicator(mut self, sort: Option<(usize, SortDirection)>) -> Self {
        self.sort = sort;
        self
    }

    /// Applies `state`'s offset, cursor, selection, sort marker and column widths.
    #[must_use]
    pub fn state(mut self, state: &TableState) -> Self {
        self.offset = state.offset();
        self.selected = state.cursor();
        self.selection = state.selection().map(|rows| (*rows.start(), *rows.end()));
        self.sort = state.sorted();
        self.widths.clear();
        self.widths.extend_from_slice(state.widths());
        self
    }

    /// Records the row count and the data rows that fit in `area` in `state`, keeping its
    /// cursor, selection and offset in range.
    ///
    /// A table reading [`TableState::view`] borrows the state, so measure first and fit after:
    /// `let rows = table.data_rows(area);` then [`TableState::fit`] once the table is dropped.
    pub fn sync(&self, area: Rect, state: &mut TableState) {
        state.fit(self.source.len(), self.data_rows(area));
    }

    /// How many data rows fit in `area` below the header, counting row gaps.
    #[must_use]
    pub fn data_rows(&self, area: Rect) -> usize {
        let area = clip_to_coordinates(area);
        let header = u16::from(self.header.is_some());
        let rows = area.height.saturating_sub(header);
        let pitch = self.row_gap.saturating_add(1);
        usize::from(rows.div_ceil(pitch))
    }

    /// The cells each column is drawn in inside `area`, header row included.
    #[must_use]
    pub fn column_areas(&self, area: Rect) -> Vec<Rect> {
        let area = clip_to_coordinates(area);
        self.with_segments(area, |segments| {
            segments
                .iter()
                .map(|segment| Rect::new(segment.x, area.y, segment.width, area.height))
                .collect()
        })
    }

    /// The column under `(x, y)` inside `area`.
    #[must_use]
    pub fn column_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let area = clip_to_coordinates(area);
        if y < area.y || y - area.y >= area.height {
            return None;
        }
        self.with_segments(area, |segments| {
            segments
                .iter()
                .position(|segment| x >= segment.x && x - segment.x < segment.width)
        })
    }

    /// The column whose right edge is under `(x, y)` on the header row: its last cell or the
    /// cell after it. Dragging there resizes the column.
    #[must_use]
    pub fn divider_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let area = clip_to_coordinates(area);
        if self.header.is_none() || y != area.y || area.height == 0 {
            return None;
        }
        self.with_segments(area, |segments| {
            segments.iter().position(|segment| {
                let edge = segment.x.saturating_add(segment.width);
                segment.width > 0 && (x == edge || x.saturating_add(1) == edge)
            })
        })
    }

    fn with_segments<R>(&self, body: Rect, use_segments: impl FnOnce(&[Rect]) -> R) -> R {
        let gutter = self.gutter(body.width);
        let body = Rect::new(body.x + gutter, body.y, body.width - gutter, body.height);
        let mut solved = self.solved.borrow_mut();
        let fresh = solved.as_ref().is_none_or(|(at, _)| *at != body);
        if fresh {
            let columns = self.columns.iter().enumerate().map(|(column, constraint)| {
                self.widths
                    .get(column)
                    .copied()
                    .flatten()
                    .map_or(*constraint, Constraint::Length)
            });
            let layout = Layout::horizontal(columns).flex(Flex::Legacy);
            *solved = Some((body, layout.split(body).segments));
        }
        use_segments(solved.as_ref().map_or(&[], |(_, segments)| segments))
    }

    fn gutter(&self, width_available: u16) -> u16 {
        self.highlight_symbol.map_or(0, |(symbol, _)| {
            u16::try_from(width(symbol))
                .unwrap_or(width_available)
                .min(width_available)
        })
    }

    fn put_header(
        &self,
        cx: &mut RenderCx<'_>,
        segment: Rect,
        column: usize,
        y: u16,
        text: &str,
        style: Style,
    ) {
        let Some(direction) = self
            .sort
            .filter(|(sorted, _)| *sorted == column)
            .map(|(_, direction)| direction)
        else {
            self.put_aligned(cx, segment, column, y, text, style);
            return;
        };
        if segment.width < 2 {
            cx.put_str(
                segment.x,
                y,
                clip_display_prefix(direction.marker(), segment.width),
                style,
            );
            return;
        }
        let title = clip_display_prefix(text, segment.width - 2);
        let used = u16::try_from(width(title)).unwrap_or(0) + 2;
        let spare = segment.width.saturating_sub(used);
        let x = match self.alignments.get(column).copied().unwrap_or_default() {
            Alignment::Left => segment.x,
            Alignment::Center => segment.x + spare / 2,
            Alignment::Right => segment.x + spare,
        };
        if !title.is_empty() {
            cx.put_str(x, y, title, style);
        }
        let mark = x + used - 1;
        cx.put_str(mark, y, direction.marker(), style);
    }

    fn blank(cx: &mut RenderCx<'_>, area: Rect, y: u16, style: Style) {
        let row = Rect::new(area.x, y, area.width, 1);
        cx.fill(row, style, ' ');
        cx.set_style(row, style);
    }

    fn column_style(&self, column: usize) -> Style {
        match self.column_styles.get(column) {
            Some(column_style) => patch_style(self.style, *column_style),
            None => self.style,
        }
    }

    fn put_aligned(
        &self,
        cx: &mut RenderCx<'_>,
        segment: Rect,
        column: usize,
        y: u16,
        text: &str,
        style: Style,
    ) {
        let draw = clip_display_prefix(text, segment.width);
        let used = u16::try_from(width(draw)).unwrap_or(segment.width);
        let spare = segment.width.saturating_sub(used);
        let x = match self.alignments.get(column).copied().unwrap_or_default() {
            Alignment::Left => segment.x,
            Alignment::Center => segment.x + spare / 2,
            Alignment::Right => segment.x + spare,
        };
        if !draw.is_empty() {
            cx.put_str(x, y, draw, style);
        }
    }
}

impl<S: RowSource, C: CellStyle> Widget for Table<S, C> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 || self.columns.is_empty() {
            return;
        }
        let gutter = self.gutter(area.width);
        self.with_segments(area, |segments| {
            let bottom = area.y + area.height;
            let mut y = area.y;
            if let Some(header) = &self.header {
                Self::blank(cx, area, y, self.style);
                for (column, segment) in segments.iter().enumerate() {
                    if segment.width == 0 {
                        continue;
                    }
                    if let Some((text, style)) = header.get(column) {
                        let segment = Rect::new(segment.x, y, segment.width, 1);
                        self.put_header(cx, segment, column, y, text, *style);
                    }
                }
                y += 1;
            }
            let mut buffer = self.buffer.borrow_mut();
            let mut index = self.offset;
            while y < bottom && index < self.source.len() {
                let mut filled = false;
                let selected = self.selected == Some(index);
                let in_range = !selected
                    && self
                        .selection
                        .is_some_and(|(first, last)| (first..=last).contains(&index));
                let odd = !selected && !in_range && index % 2 == 1 && self.odd_row.is_some();
                let band = if selected {
                    Some(self.highlight)
                } else if in_range {
                    Some(patch_style(self.style, self.selection_style))
                } else if odd {
                    self.odd_row
                        .map(|odd_style| patch_style(self.style, odd_style))
                } else {
                    None
                };
                Self::blank(cx, area, y, band.unwrap_or(self.style));
                if let (true, Some((symbol, symbol_style))) = (selected, self.highlight_symbol) {
                    let draw = clip_display_prefix(symbol, gutter);
                    cx.put_str(area.x, y, draw, patch_style(self.highlight, symbol_style));
                }
                for (column, segment) in segments.iter().enumerate() {
                    if segment.width == 0 {
                        continue;
                    }
                    let base = if selected {
                        self.highlight
                    } else if in_range {
                        patch_style(self.column_style(column), self.selection_style)
                    } else if let (true, Some(odd_style)) = (odd, self.odd_row) {
                        patch_style(self.column_style(column), odd_style)
                    } else {
                        self.column_style(column)
                    };
                    let style = patch_style(base, self.cell_styles.style_for(index, column));
                    let text = if let Some(text) = self.source.cell(index, column) {
                        text
                    } else {
                        if !filled {
                            self.source.row(index, &mut buffer);
                            filled = true;
                        }
                        buffer.cells.get(column).map_or("", String::as_str)
                    };
                    self.put_aligned(cx, *segment, column, y, text, style);
                }
                y = y.saturating_add(1);
                for _ in 0..self.row_gap {
                    if y >= bottom {
                        break;
                    }
                    Self::blank(cx, area, y, self.style);
                    y += 1;
                }
                index += 1;
            }
            while y < bottom {
                Self::blank(cx, area, y, self.style);
                y += 1;
            }
        });
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = u16::try_from(self.source.len()).unwrap_or(u16::MAX);
        let gaps = rows.saturating_sub(1).saturating_mul(self.row_gap);
        let header = u16::from(self.header.is_some());
        Size::new(
            avail.width,
            rows.saturating_add(gaps)
                .saturating_add(header)
                .min(avail.height),
        )
    }
}
