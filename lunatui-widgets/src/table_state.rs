// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::cmp::Ordering;
use std::ops::RangeInclusive;

use lunatui_text::source::{RowBuilder, RowSource};

/// Which way a sorted [`Table`](crate::Table) column runs.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    /// Smallest first, marked `▲`.
    #[default]
    Ascending,
    /// Largest first, marked `▼`.
    Descending,
}

impl SortDirection {
    /// The other direction.
    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub(crate) const fn marker(self) -> &'static str {
        match self {
            Self::Ascending => "▲",
            Self::Descending => "▼",
        }
    }
}

/// A [`RowSource`] that shows another source's rows in a [`TableState`]'s sorted order.
///
/// Built by [`TableState::view`]. Row `i` of the view is the source row at position `i` of the
/// order, or row `i` itself while the state is unsorted.
#[derive(Clone, Copy, Debug)]
pub struct TableView<'a, S> {
    source: &'a S,
    order: &'a [usize],
}

impl<S> TableView<'_, S> {
    fn source_index(&self, index: usize) -> usize {
        self.order.get(index).copied().unwrap_or(index)
    }
}

impl<S: RowSource> RowSource for TableView<'_, S> {
    fn len(&self) -> usize {
        self.source.len()
    }

    fn row(&self, index: usize, out: &mut RowBuilder) {
        self.source.row(self.source_index(index), out);
    }

    fn cell(&self, index: usize, column: usize) -> Option<&str> {
        self.source.cell(self.source_index(index), column)
    }
}

/// Sorting, selection, scrolling and column widths for a [`Table`](crate::Table), kept by the
/// app between frames.
///
/// Rows are counted in the order shown. [`sort`](Self::sort) orders them by one column's text,
/// comparing numbers by value and other text without regard to case, and keeps the cursor on
/// the same source row; [`view`](Self::view) shows a source in that order and
/// [`source_row`](Self::source_row) maps back. The cursor moves with [`down`](Self::down) and
/// [`up`](Self::up); moving with `extend` set selects the rows between the anchor and the
/// cursor. [`Table::sync`](crate::Table::sync) gives the state the row count and the visible
/// rows so moves keep the cursor on screen. Column widths set here replace the table's
/// constraints for those columns, and the drag methods resize a column from its header divider.
///
/// Draw with both: `Table::new(state.view(&source), columns).state(&state)`. The view supplies
/// the rows in order and [`Table::state`](crate::Table::state) the cursor, selection, sort
/// marker and widths.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableState {
    order: Vec<usize>,
    cursor: Option<usize>,
    anchor: Option<usize>,
    offset: usize,
    sort: Option<(usize, SortDirection)>,
    widths: Vec<Option<u16>>,
    drag: Option<(usize, u16, u16)>,
    len: usize,
    rows: usize,
}

impl TableState {
    /// Unsorted, scrolled to the top, without a cursor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            order: Vec::new(),
            cursor: None,
            anchor: None,
            offset: 0,
            sort: None,
            widths: Vec::new(),
            drag: None,
            len: 0,
            rows: 0,
        }
    }

    /// Shows `source` in this state's order.
    #[must_use]
    pub fn view<'a, S>(&'a self, source: &'a S) -> TableView<'a, S> {
        TableView {
            source,
            order: &self.order,
        }
    }

    /// The source row shown at `row`.
    #[must_use]
    pub fn source_row(&self, row: usize) -> usize {
        self.order.get(row).copied().unwrap_or(row)
    }

    /// The row with the cursor.
    #[must_use]
    pub const fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// The first row shown.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// The selected rows: from the anchor to the cursor, or the cursor alone.
    #[must_use]
    pub fn selection(&self) -> Option<RangeInclusive<usize>> {
        let cursor = self.cursor?;
        let anchor = self.anchor.unwrap_or(cursor);
        Some(anchor.min(cursor)..=anchor.max(cursor))
    }

    /// Whether row `row` is selected.
    #[must_use]
    pub fn is_selected(&self, row: usize) -> bool {
        self.selection().is_some_and(|range| range.contains(&row))
    }

    /// The source rows of the selection, in the order shown.
    pub fn selected_source_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.selection()
            .into_iter()
            .flatten()
            .map(|row| self.source_row(row))
    }

    /// Puts the cursor on `row`, or removes it, and drops any range.
    pub fn set_cursor(&mut self, row: Option<usize>) {
        self.cursor = row.map(|row| row.min(self.len.saturating_sub(1)));
        self.anchor = None;
        self.reveal();
    }

    /// Moves the cursor down by `rows`, starting at the first row when there is none. With
    /// `extend`, the rows between the anchor and the cursor are selected.
    pub fn down(&mut self, rows: usize, extend: bool) {
        let target = self.cursor.map_or(0, |cursor| cursor.saturating_add(rows));
        self.move_to(target, extend);
    }

    /// Moves the cursor up by `rows`, starting at the first row when there is none.
    pub fn up(&mut self, rows: usize, extend: bool) {
        let target = self.cursor.map_or(0, |cursor| cursor.saturating_sub(rows));
        self.move_to(target, extend);
    }

    /// Moves down by one screen of rows.
    pub fn page_down(&mut self, extend: bool) {
        self.down(self.rows.max(1), extend);
    }

    /// Moves up by one screen of rows.
    pub fn page_up(&mut self, extend: bool) {
        self.up(self.rows.max(1), extend);
    }

    /// Selects every row, with the cursor on the last.
    pub fn select_all(&mut self) {
        if self.len > 0 {
            self.anchor = Some(0);
            self.cursor = Some(self.len - 1);
            self.reveal();
        }
    }

    /// Keeps the cursor and drops the rest of the selection.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// The sorted column and direction.
    #[must_use]
    pub const fn sorted(&self) -> Option<(usize, SortDirection)> {
        self.sort
    }

    /// Orders the rows of `source` by the text in `column`, keeping equal rows in source order
    /// and the cursor on the same source row. The range selection is dropped.
    pub fn sort<S: RowSource>(&mut self, source: &S, column: usize, direction: SortDirection) {
        let focused = self.cursor.map(|cursor| self.source_row(cursor));
        let mut buffer = RowBuilder::default();
        let keys: Vec<String> = (0..source.len())
            .map(|row| {
                if let Some(text) = source.cell(row, column) {
                    text.to_owned()
                } else {
                    source.row(row, &mut buffer);
                    buffer.cells.get(column).cloned().unwrap_or_default()
                }
            })
            .collect();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        order.sort_by(|left, right| {
            let ordering = natural_cmp(&keys[*left], &keys[*right]);
            match direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });
        self.order = order;
        self.len = keys.len();
        self.sort = Some((column, direction));
        self.anchor = None;
        self.cursor = focused.and_then(|row| self.order.iter().position(|at| *at == row));
        self.reveal();
    }

    /// Sorts by `column` ascending, or reverses it when it is already the sorted column.
    pub fn toggle_sort<S: RowSource>(&mut self, source: &S, column: usize) {
        let direction = match self.sort {
            Some((sorted, direction)) if sorted == column => direction.reversed(),
            _ => SortDirection::Ascending,
        };
        self.sort(source, column, direction);
    }

    /// Returns to source order, keeping the cursor on the same source row.
    pub fn clear_sort(&mut self) {
        let focused = self.cursor.map(|cursor| self.source_row(cursor));
        self.order.clear();
        self.sort = None;
        self.anchor = None;
        self.cursor = focused;
        self.reveal();
    }

    /// The width set for `column`, if any.
    #[must_use]
    pub fn column_width(&self, column: usize) -> Option<u16> {
        self.widths.get(column).copied().flatten()
    }

    /// Sets `column` to `width` cells (at least one), or back to the table's constraint.
    pub fn set_column_width(&mut self, column: usize, width: Option<u16>) {
        if self.widths.len() <= column {
            self.widths.resize(column + 1, None);
        }
        self.widths[column] = width.map(|width| width.max(1));
    }

    /// Starts resizing `column`, `width` cells wide now, from a press at column `x`.
    pub fn start_resize(&mut self, column: usize, width: u16, x: u16) {
        self.drag = Some((column, width, x));
    }

    /// Resizes the column being dragged so its right edge follows the pointer at column `x`.
    pub fn drag_resize(&mut self, x: u16) {
        if let Some((column, width, start)) = self.drag {
            let moved = i32::from(x) - i32::from(start);
            let wanted = (i32::from(width) + moved).clamp(1, i32::from(u16::MAX));
            self.set_column_width(column, u16::try_from(wanted).ok());
        }
    }

    /// Ends a resize.
    pub fn end_resize(&mut self) {
        self.drag = None;
    }

    /// The column being resized.
    #[must_use]
    pub fn resizing(&self) -> Option<usize> {
        self.drag.map(|(column, _, _)| column)
    }

    pub(crate) fn widths(&self) -> &[Option<u16>] {
        &self.widths
    }

    /// Records that there are `len` rows and `rows` of them show at once, keeping the cursor,
    /// selection and offset in range. A sort made for a different row count is dropped, since
    /// the rows it ordered have changed. [`Table::sync`](crate::Table::sync) calls this.
    pub fn fit(&mut self, len: usize, rows: usize) {
        self.len = len;
        self.rows = rows;
        if self.order.len() != len {
            self.order.clear();
            self.sort = None;
        }
        self.cursor = self
            .cursor
            .filter(|_| len > 0)
            .map(|cursor| cursor.min(len - 1));
        self.anchor = self
            .anchor
            .filter(|_| len > 0)
            .map(|anchor| anchor.min(len - 1));
        self.reveal();
    }

    fn move_to(&mut self, row: usize, extend: bool) {
        if self.len == 0 {
            return;
        }
        let row = row.min(self.len - 1);
        if extend {
            self.anchor = self.anchor.or(self.cursor).or(Some(row));
        } else {
            self.anchor = None;
        }
        self.cursor = Some(row);
        self.reveal();
    }

    fn reveal(&mut self) {
        if let Some(cursor) = self.cursor {
            if cursor < self.offset {
                self.offset = cursor;
            } else if self.rows > 0 && cursor >= self.offset + self.rows {
                self.offset = cursor + 1 - self.rows;
            }
        }
        self.offset = self.offset.min(self.len.saturating_sub(self.rows.max(1)));
    }
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    match (number(left), number(right)) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left
            .chars()
            .flat_map(char::to_lowercase)
            .cmp(right.chars().flat_map(char::to_lowercase)),
    }
}

fn number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let digits = trimmed
        .strip_suffix('%')
        .unwrap_or(trimmed)
        .trim_end_matches(|ch: char| ch.is_ascii_alphabetic() || ch == ' ');
    let cleaned: String = digits.chars().filter(|ch| *ch != ',').collect();
    cleaned
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}
