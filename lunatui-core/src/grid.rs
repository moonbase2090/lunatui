// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use unicode_segmentation::UnicodeSegmentation;

use crate::cell::{Cell, CellWidth, Meta};
use crate::grapheme::{ClusterTable, DecodedGlyph, GlyphSlot, cluster_display_width};
use crate::style::Style;
use crate::width::char_width;

/// A rectangle on the cell grid, in column/row coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Left column (inclusive).
    pub x: u16,
    /// Top row (inclusive).
    pub y: u16,
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

impl Rect {
    /// A rectangle at `(x, y)` with the given size.
    #[must_use]
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Per-row damage: whether the row changed since the last [`Grid::clear_dirty`], and the
/// column span `[min_x, max_x)` that was touched (PRD §2.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RowDirty {
    /// Whether this row has changes since the last clear.
    pub dirty: bool,
    /// First changed column (inclusive).
    pub min_x: u16,
    /// One past the last changed column (exclusive).
    pub max_x: u16,
}

/// A terminal cell grid stored as `[u128]` words (ADR 0001 §4.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    width: u16,
    height: u16,
    cells: Vec<u128>,
    rows: Vec<RowDirty>,
}

impl Grid {
    /// A grid of `width`×`height` cells, filled with [`Cell::BLANK`].
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        let len = usize::from(width) * usize::from(height);
        let blank = Cell::BLANK.to_bits();
        Self {
            width,
            height,
            cells: alloc::vec![blank; len],
            rows: alloc::vec![RowDirty::default(); usize::from(height)],
        }
    }

    /// Grid width in columns.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Grid height in rows.
    #[must_use]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Returns the cell at `(x, y)`, or `None` when out of bounds.
    #[must_use]
    pub fn get(&self, x: u16, y: u16) -> Option<Cell> {
        self.index(x, y)
            .and_then(|idx| Cell::from_bits(self.cells[idx]))
    }

    /// Packed cell word at `(x, y)`, or [`Cell::BLANK`] when out of bounds.
    #[must_use]
    pub fn cell_bits(&self, x: u16, y: u16) -> u128 {
        self.get(x, y).map_or(Cell::BLANK.to_bits(), Cell::to_bits)
    }

    /// Packed cell words of `row` (see [`Cell::to_bits`]); empty when `row` is out of bounds.
    #[must_use]
    pub fn row_bits(&self, row: u16) -> &[u128] {
        self.row_word_slice(row)
    }

    #[must_use]
    pub(crate) fn row_word_slice(&self, row: u16) -> &[u128] {
        if row >= self.height {
            return &[];
        }
        let width = usize::from(self.width);
        let start = usize::from(row) * width;
        &self.cells[start..start + width]
    }

    /// Clears all row dirty flags and spans.
    pub fn clear_dirty(&mut self) {
        for row in &mut self.rows {
            *row = RowDirty::default();
        }
    }

    /// Dirty metadata for `row`.
    #[must_use]
    pub fn row_dirty(&self, row: u16) -> Option<RowDirty> {
        self.rows.get(usize::from(row)).copied()
    }

    /// Writes `text` at `(x, y)` using `style` and `clusters` for multi-codepoint graphemes.
    ///
    /// Wide graphemes that do not fit at the right edge are replaced by a blank (ADR 0001 §4.7).
    pub fn put_str(
        &mut self,
        x: u16,
        y: u16,
        style: Style,
        text: &str,
        clusters: &mut ClusterTable,
    ) {
        if y >= self.height {
            return;
        }
        let mut col = x;
        let mut buf = [0u8; 4];
        for grapheme in UnicodeSegmentation::graphemes(text, true) {
            if col >= self.width {
                break;
            }
            let display = cluster_display_width(grapheme);
            if display <= 2 {
                col = self.put_cluster(col, y, style, grapheme, display, clusters);
                continue;
            }
            let mut anchor_col: Option<u16> = None;
            for ch in grapheme.chars() {
                if col >= self.width {
                    break;
                }
                let width = char_width(ch);
                if width == 0 {
                    if let Some(lead) = anchor_col {
                        self.attach_combining(lead, y, ch, clusters);
                    }
                    continue;
                }
                let scalar = ch.encode_utf8(&mut buf);
                let lead = col;
                col = self.put_cluster(col, y, style, scalar, width, clusters);
                anchor_col = Some(lead);
            }
        }
    }

    fn attach_combining(&mut self, col: u16, y: u16, mark: char, clusters: &mut ClusterTable) {
        let Some(cell) = self.get(col, y) else {
            return;
        };
        if cell.meta().width == CellWidth::Continuation {
            return;
        }
        let mut text = glyph_text(cell.glyph(), clusters);
        text.push(mark);
        let glyph = clusters.glyph_for(&text);
        let updated = Cell::from_parts(glyph, cell.style(), cell.meta());
        self.place(col, y, updated);
    }

    fn put_cluster(
        &mut self,
        col: u16,
        y: u16,
        style: Style,
        grapheme: &str,
        display: usize,
        clusters: &mut ClusterTable,
    ) -> u16 {
        if display == 0 {
            return col;
        }
        let span = u16::try_from(display).unwrap_or(1);
        self.erase_span(col, y, span, style);
        let glyph = clusters.glyph_for(grapheme);
        if display == 2 {
            if col + 1 >= self.width {
                self.place_blank(col, y, style);
                return col + 1;
            }
            let lead = Cell::from_parts(
                glyph,
                style,
                Meta {
                    width: CellWidth::Two,
                    ..Meta::default()
                },
            );
            let tail = Cell::from_parts(
                GlyphSlot::continuation(),
                style,
                Meta {
                    width: CellWidth::Continuation,
                    ..Meta::default()
                },
            );
            self.place(col, y, lead);
            self.place(col + 1, y, tail);
            col + 2
        } else {
            let cell = Cell::from_parts(
                glyph,
                style,
                Meta {
                    width: CellWidth::One,
                    ..Meta::default()
                },
            );
            self.place(col, y, cell);
            col + 1
        }
    }

    /// Fills `area` with `ch` and `style`.
    pub fn fill(&mut self, area: Rect, style: Style, ch: char, clusters: &mut ClusterTable) {
        let end_x = area.x.saturating_add(area.width);
        let end_y = area.y.saturating_add(area.height);
        let display = char_width(ch);
        for row in area.y..end_y.min(self.height) {
            let mut col = area.x;
            let row_end = end_x.min(self.width);
            while col < row_end {
                if display == 2 && col + 1 < row_end {
                    let glyph = glyph_for_char(ch, clusters);
                    self.erase_span(col, row, 2, style);
                    self.place(
                        col,
                        row,
                        Cell::from_parts(
                            glyph,
                            style,
                            Meta {
                                width: CellWidth::Two,
                                ..Meta::default()
                            },
                        ),
                    );
                    self.place(
                        col + 1,
                        row,
                        Cell::from_parts(
                            GlyphSlot::continuation(),
                            style,
                            Meta {
                                width: CellWidth::Continuation,
                                ..Meta::default()
                            },
                        ),
                    );
                    col += 2;
                } else if display == 2 {
                    self.erase_span(col, row, 1, style);
                    self.place_blank(col, row, style);
                    col += 1;
                } else {
                    let glyph = glyph_for_char(ch, clusters);
                    self.erase_span(col, row, 1, style);
                    self.place(
                        col,
                        row,
                        Cell::from_parts(
                            glyph,
                            style,
                            Meta {
                                width: CellWidth::One,
                                ..Meta::default()
                            },
                        ),
                    );
                    col += 1;
                }
            }
        }
    }

    /// Sets foreground, background and attributes in `area`, keeping each cell's glyph and width.
    pub fn set_style(&mut self, area: Rect, style: Style) {
        let end_x = area.x.saturating_add(area.width).min(self.width);
        let end_y = area.y.saturating_add(area.height);
        if area.x >= end_x {
            return;
        }
        for row in area.y..end_y.min(self.height) {
            // A wide glyph cut by an edge of `area` is restyled whole: extend by its other half.
            let mut start = area.x;
            if start > 0
                && self
                    .get(start, row)
                    .is_some_and(|cell| cell.meta().width == CellWidth::Continuation)
            {
                start -= 1;
            }
            let mut end = end_x;
            if end < self.width
                && self
                    .get(end - 1, row)
                    .is_some_and(|cell| cell.meta().width == CellWidth::Two)
            {
                end += 1;
            }
            for col in start..end {
                if let Some(cell) = self.get(col, row) {
                    let meta = Meta {
                        attrs: style.attrs,
                        width: cell.meta().width,
                        flags: cell.meta().flags,
                        ext: cell.meta().ext,
                    };
                    self.place(col, row, Cell::from_parts(cell.glyph(), style, meta));
                }
            }
        }
    }

    /// One row of display text (wide graphemes appear once; continuation cells are skipped).
    #[must_use]
    pub fn row_display(&self, row: u16, clusters: &ClusterTable) -> String {
        self.row_text(row, clusters)
    }

    /// All rows joined with newlines, suitable for printing a full frame.
    #[must_use]
    pub fn display_text(&self, clusters: &ClusterTable) -> String {
        let mut out = String::new();
        for row in 0..self.height {
            out.push_str(&self.row_text(row, clusters));
            out.push('\n');
        }
        out
    }

    fn row_text(&self, row: u16, clusters: &ClusterTable) -> String {
        let mut line = String::new();
        let mut col = 0u16;
        while col < self.width {
            let Some(cell) = self.get(col, row) else {
                break;
            };
            match cell.meta().width {
                CellWidth::Continuation => col += 1,
                CellWidth::Two => {
                    line.push_str(&glyph_text(cell.glyph(), clusters));
                    col += 2;
                }
                CellWidth::One => {
                    line.push_str(&glyph_text(cell.glyph(), clusters));
                    col += 1;
                }
            }
        }
        line
    }

    fn erase_span(&mut self, col: u16, row: u16, span: u16, style: Style) {
        for offset in 0..span {
            let column = col.saturating_add(offset);
            if column < self.width {
                self.erase_touching(column, row, style);
            }
        }
    }

    fn erase_touching(&mut self, col: u16, row: u16, style: Style) {
        if col > 0
            && let Some(lead) = self.get(col - 1, row)
            && lead.meta().width == CellWidth::Two
        {
            self.place_blank(col - 1, row, style);
            self.place_blank(col, row, style);
            return;
        }
        if let Some(here) = self.get(col, row) {
            match here.meta().width {
                CellWidth::Continuation => {
                    if col > 0 {
                        self.place_blank(col - 1, row, style);
                    }
                    self.place_blank(col, row, style);
                }
                CellWidth::Two => {
                    self.place_blank(col, row, style);
                    if col + 1 < self.width {
                        self.place_blank(col + 1, row, style);
                    }
                }
                CellWidth::One => {}
            }
        }
    }

    fn place_blank(&mut self, col: u16, row: u16, style: Style) {
        let blank = Cell::from_parts(
            GlyphSlot::inline_scalar(' ').unwrap_or(GlyphSlot::continuation()),
            style,
            Meta {
                width: CellWidth::One,
                ..Meta::default()
            },
        );
        self.place(col, row, blank);
    }

    fn place(&mut self, col: u16, row: u16, cell: Cell) {
        if let Some(idx) = self.index(col, row) {
            self.cells[idx] = cell.to_bits();
            let end = col + column_span(cell);
            self.mark_dirty(row, col, end);
        }
    }

    fn mark_dirty(&mut self, row: u16, start: u16, end: u16) {
        if start >= end || row >= self.height {
            return;
        }
        let slot = &mut self.rows[usize::from(row)];
        if slot.dirty {
            slot.min_x = slot.min_x.min(start);
            slot.max_x = slot.max_x.max(end);
        } else {
            slot.dirty = true;
            slot.min_x = start;
            slot.max_x = end;
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(usize::from(y) * usize::from(self.width) + usize::from(x))
        }
    }
}

fn column_span(cell: Cell) -> u16 {
    match cell.meta().width {
        CellWidth::Two => 2,
        CellWidth::Continuation | CellWidth::One => 1,
    }
}

fn glyph_for_char(ch: char, clusters: &mut ClusterTable) -> GlyphSlot {
    if let Some(slot) = GlyphSlot::inline_scalar(ch) {
        slot
    } else {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        clusters.glyph_for(s)
    }
}

fn glyph_text(glyph: GlyphSlot, clusters: &ClusterTable) -> String {
    match glyph.decode() {
        DecodedGlyph::Inline(scalar) => {
            let mut s = String::new();
            s.push(scalar);
            s
        }
        DecodedGlyph::Cluster(id) => clusters
            .cluster_text(id)
            .map_or_else(|| "\u{FFFD}".to_owned(), str::to_owned),
        DecodedGlyph::Continuation => String::new(),
    }
}
