// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use unicode_segmentation::UnicodeSegmentation;

use crate::cell::{Cell, CellWidth, Meta, word_width};
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

const STYLE_BITS: u128 = (0xFFFF_FFFF_u128 << 32) | (0xFFFF_FFFF_u128 << 64) | (0xFFF_u128 << 96);

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
        let bytes = text.as_bytes();
        if x < self.width
            && !bytes.is_empty()
            && bytes.iter().all(|byte| (0x20..0x7F).contains(byte))
        {
            let count = bytes.len().min(usize::from(self.width - x));
            if self.put_ascii(x, y, style, &bytes[..count]) {
                return;
            }
        }
        self.put_graphemes(x, y, style, text, clusters);
    }

    /// Places `ch` at `(x, y)` with `style`, including text attributes.
    ///
    /// A width-1 character replaces that cell directly. Any other character goes
    /// through [`put_str`](Self::put_str), which does not store attributes, and
    /// then [`set_style`](Self::set_style) writes them.
    pub fn put_char(
        &mut self,
        x: u16,
        y: u16,
        style: Style,
        ch: char,
        clusters: &mut ClusterTable,
    ) {
        if y >= self.height || x >= self.width {
            return;
        }
        if char_width(ch) == 1
            && let Some(slot) = GlyphSlot::inline_scalar(ch)
            && self
                .get(x, y)
                .is_none_or(|cell| cell.meta().width == CellWidth::One)
        {
            self.place(
                x,
                y,
                Cell::from_parts(
                    slot,
                    style,
                    Meta {
                        width: CellWidth::One,
                        attrs: style.attrs,
                        ..Meta::default()
                    },
                ),
            );
            return;
        }
        let mut encoded = [0; 4];
        self.put_str(x, y, style, ch.encode_utf8(&mut encoded), clusters);
        if style.attrs.to_bits() != 0 {
            self.set_style(Rect::new(x, y, 1, 1), style);
        }
    }

    fn put_ascii(&mut self, x: u16, y: u16, style: Style, bytes: &[u8]) -> bool {
        let start = usize::from(y) * usize::from(self.width) + usize::from(x);
        let end = start + bytes.len();
        if self.cells[start..end]
            .iter()
            .any(|word| word_width(*word) != Some(CellWidth::One))
        {
            return false;
        }
        let meta = Meta {
            width: CellWidth::One,
            ..Meta::default()
        };
        let blank = Cell::from_parts(GlyphSlot::continuation(), style, meta).to_bits();
        let template = blank & !u128::from(u32::MAX);
        for (word, byte) in self.cells[start..end].iter_mut().zip(bytes) {
            *word = template | u128::from(*byte);
        }
        let columns = u16::try_from(bytes.len()).unwrap_or(u16::MAX);
        self.mark_dirty(y, x, x + columns);
        true
    }

    fn put_graphemes(
        &mut self,
        x: u16,
        y: u16,
        style: Style,
        text: &str,
        clusters: &mut ClusterTable,
    ) {
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
        let end_y = area.y.saturating_add(area.height).min(self.height);
        let row_end = end_x.min(self.width);
        if area.x >= row_end || area.y >= end_y {
            return;
        }
        let display = char_width(ch);
        let word = (display == 1).then(|| {
            let meta = Meta {
                attrs: style.attrs,
                width: CellWidth::One,
                ..Meta::default()
            };
            Cell::from_parts(glyph_for_char(ch, clusters), style, meta).to_bits()
        });
        for row in area.y..end_y {
            if let Some(word) = word
                && !self.cuts_wide_glyph(area.x, row_end, row)
            {
                let base = usize::from(row) * usize::from(self.width);
                self.cells[base + usize::from(area.x)..base + usize::from(row_end)].fill(word);
                self.mark_dirty(row, area.x, row_end);
                continue;
            }
            let span = Rect::new(area.x, row, row_end - area.x, 1);
            self.fill_row_cells(span, style, ch, clusters);
        }
    }

    fn cuts_wide_glyph(&self, start: u16, end: u16, row: u16) -> bool {
        let continuation_at_start = start > 0
            && self
                .get(start, row)
                .is_some_and(|cell| cell.meta().width == CellWidth::Continuation);
        let lead_at_end = self
            .get(end - 1, row)
            .is_some_and(|cell| cell.meta().width == CellWidth::Two);
        continuation_at_start || lead_at_end
    }

    fn fill_row_cells(&mut self, span: Rect, style: Style, ch: char, clusters: &mut ClusterTable) {
        let (row, start) = (span.y, span.x);
        let row_end = span.x + span.width;
        let display = char_width(ch);
        let mut col = start;
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
                            attrs: style.attrs,
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
                            attrs: style.attrs,
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
                            attrs: style.attrs,
                            width: CellWidth::One,
                            ..Meta::default()
                        },
                    ),
                );
                col += 1;
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
        let meta = Meta {
            attrs: style.attrs,
            ..Meta::default()
        };
        let restyle =
            Cell::from_parts(GlyphSlot::continuation(), style, meta).to_bits() & STYLE_BITS;
        for row in area.y..end_y.min(self.height) {
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
            let base = usize::from(row) * usize::from(self.width);
            for word in &mut self.cells[base + usize::from(start)..base + usize::from(end)] {
                *word = (*word & !STYLE_BITS) | restyle;
            }
            self.mark_dirty(row, start, end);
        }
    }

    /// Copies the cells of `from` in `src` to `(x, y)` in this grid, clipped to both grids.
    ///
    /// Both grids must share one [`ClusterTable`], because cluster glyphs are copied by id. A
    /// source cell with a [`Color::Default`](crate::Color::Default) background keeps the
    /// background already under it, so content drawn without a background shows the surface it
    /// is copied onto. A wide grapheme cut by either edge of the copy, in the source or in this
    /// grid, becomes a blank in its own style.
    #[cfg(feature = "blit")]
    pub fn blit(&mut self, src: &Grid, from: Rect, x: u16, y: u16) {
        const BG_BITS: u128 = 0xFFFF_FFFF_u128 << 64;
        let src_end_x = from.x.saturating_add(from.width).min(src.width);
        let src_end_y = from.y.saturating_add(from.height).min(src.height);
        if from.x >= src_end_x || from.y >= src_end_y || x >= self.width || y >= self.height {
            return;
        }
        let width = (src_end_x - from.x).min(self.width - x);
        let height = (src_end_y - from.y).min(self.height - y);
        let end = x + width;
        for row in 0..height {
            let dst_row = y + row;
            let src_row = from.y + row;
            if x > 0
                && let Some(cell) = self.get(x, dst_row)
                && cell.meta().width == CellWidth::Continuation
            {
                self.place_blank(x - 1, dst_row, cell.style());
            }
            if end < self.width
                && let Some(cell) = self.get(end - 1, dst_row)
                && cell.meta().width == CellWidth::Two
            {
                self.place_blank(end, dst_row, cell.style());
            }
            let src_base = usize::from(src_row) * usize::from(src.width) + usize::from(from.x);
            let dst_base = usize::from(dst_row) * usize::from(self.width) + usize::from(x);
            let words = &src.cells[src_base..src_base + usize::from(width)];
            for (dst, &word) in self.cells[dst_base..dst_base + usize::from(width)]
                .iter_mut()
                .zip(words)
            {
                *dst = if word & BG_BITS == 0 {
                    word | (*dst & BG_BITS)
                } else {
                    word
                };
            }
            for col in [x, end - 1] {
                let Some(cell) = self.get(col, dst_row) else {
                    continue;
                };
                let cut = match cell.meta().width {
                    CellWidth::Continuation => col == x,
                    CellWidth::Two => col == end - 1,
                    CellWidth::One => false,
                };
                if cut {
                    self.place_blank(col, dst_row, cell.style());
                }
            }
            self.mark_dirty(dst_row, x, end);
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
                attrs: style.attrs,
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

#[cfg(test)]
mod put_str_tests;

#[cfg(test)]
mod tests {
    use super::{Cell, CellWidth, ClusterTable, Grid, Rect, Style};
    use crate::{Attrs, Color};

    impl Grid {
        fn fill_reference(
            &mut self,
            area: Rect,
            style: Style,
            ch: char,
            clusters: &mut ClusterTable,
        ) {
            let end_x = area.x.saturating_add(area.width);
            let end_y = area.y.saturating_add(area.height);
            for row in area.y..end_y.min(self.height) {
                let row_end = end_x.min(self.width);
                let span = Rect::new(area.x, row, row_end.saturating_sub(area.x), 1);
                self.fill_row_cells(span, style, ch, clusters);
            }
        }

        fn set_style_reference(&mut self, area: Rect, style: Style) {
            let end_x = area.x.saturating_add(area.width).min(self.width);
            let end_y = area.y.saturating_add(area.height);
            if area.x >= end_x {
                return;
            }
            for row in area.y..end_y.min(self.height) {
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
                        let meta = super::Meta {
                            attrs: style.attrs,
                            width: cell.meta().width,
                            flags: cell.meta().flags,
                            ext: cell.meta().ext,
                        };
                        let updated = super::Cell::from_parts(cell.glyph(), style, meta);
                        self.place(col, row, updated);
                    }
                }
            }
        }
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }

        fn small(&mut self, bound: u64) -> u16 {
            u16::try_from(self.below(bound)).unwrap_or(0)
        }
    }

    fn style(rng: &mut Rng) -> Style {
        let colors = [
            Color::Default,
            Color::Indexed(3),
            Color::Rgb(9, 8, 7),
            Color::Rgb(200, 100, 50),
        ];
        let pick = |index: u64| colors[usize::try_from(index % 4).unwrap_or(0)];
        let bits = rng.next();
        let attrs = Attrs::default()
            .set_bold(bits & 1 != 0)
            .set_dim(bits & 2 != 0);
        Style::new()
            .fg(pick(bits >> 8))
            .bg(pick(bits >> 16))
            .with_attrs(attrs)
    }

    fn area(rng: &mut Rng) -> Rect {
        Rect::new(rng.small(14), rng.small(5), rng.small(9), rng.small(4))
    }

    #[test]
    fn put_char_stores_a_width_one_glyph_and_its_attributes() {
        let mut clusters = ClusterTable::new();
        let mut grid = Grid::new(2, 1);
        let style = Style::new()
            .fg(Color::Indexed(2))
            .with_attrs(Attrs::default().set_bold(true));
        grid.put_char(0, 0, style, '█', &mut clusters);
        assert_eq!(grid.row_display(0, &clusters), "█ ");
        assert_eq!(
            grid.get(0, 0)
                .map(Cell::style)
                .map(|style| (style.fg, style.attrs.bold())),
            Some((Color::Indexed(2), true))
        );
    }

    #[test]
    fn fill_keeps_attributes_for_scalar_wide_and_clipped_cells() {
        let style = Style::new().with_attrs(Attrs::new().set_bold(true).set_dim(true));
        for glyph in ['a', '░', '界'] {
            let mut grid = Grid::new(3, 1);
            let mut clusters = ClusterTable::new();
            grid.fill(Rect::new(0, 0, 3, 1), style, glyph, &mut clusters);
            let expected = match glyph {
                'a' => "aaa",
                '░' => "░░░",
                _ => "界 ",
            };
            assert_eq!(grid.row_display(0, &clusters), expected);
            for column in 0..3 {
                let attrs = grid.get(column, 0).map(|cell| cell.style().attrs);
                assert_eq!(attrs.map(Attrs::bold), Some(true));
                assert_eq!(attrs.map(Attrs::dim), Some(true));
            }
        }
    }

    #[test]
    fn fill_and_set_style_match_the_per_cell_reference() {
        const TEXTS: [&str; 7] = ["a", "日", "é", "日本", "x", "│", "ab"];
        const CHARS: [char; 7] = ['x', '日', ' ', '·', '\u{301}', '🙂', '│'];
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        for _ in 0..400 {
            let mut fast = Grid::new(12, 4);
            let mut fast_clusters = ClusterTable::new();
            let mut slow = fast.clone();
            let mut slow_clusters = ClusterTable::new();
            for _ in 0..12 {
                let text = TEXTS[usize::try_from(rng.below(7)).unwrap_or(0)];
                let (x, y, painted) = (rng.small(12), rng.small(4), style(&mut rng));
                fast.put_str(x, y, painted, text, &mut fast_clusters);
                slow.put_str(x, y, painted, text, &mut slow_clusters);
            }
            for _ in 0..10 {
                let rect = area(&mut rng);
                let painted = style(&mut rng);
                if rng.below(2) == 0 {
                    let ch = CHARS[usize::try_from(rng.below(7)).unwrap_or(0)];
                    fast.fill(rect, painted, ch, &mut fast_clusters);
                    slow.fill_reference(rect, painted, ch, &mut slow_clusters);
                } else {
                    fast.set_style(rect, painted);
                    slow.set_style_reference(rect, painted);
                }
                assert_eq!(fast, slow);
            }
        }
    }
}
