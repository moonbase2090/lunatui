// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_layout::{Constraint, Flex, Layout};
use lunatui_text::source::{RowBuilder, RowSource};

use crate::render_cx::patch_style;
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
        if area.width == 0 || area.height == 0 || self.columns.is_empty() {
            return;
        }
        let gutter = self.highlight_symbol.map_or(0, |(symbol, _)| {
            u16::try_from(width(symbol))
                .unwrap_or(area.width)
                .min(area.width)
        });
        let body = Rect::new(area.x + gutter, area.y, area.width - gutter, area.height);
        let layout = Layout::horizontal(self.columns.iter().copied()).flex(Flex::Legacy);
        let solved = layout.split(body);
        let bottom = area.y + area.height;
        let mut y = area.y;
        let blank_row = " ".repeat(usize::from(area.width));
        if let Some(header) = &self.header {
            cx.put_str(area.x, y, &blank_row, self.style);
            for (column, segment) in solved.segments.iter().enumerate() {
                if segment.width == 0 {
                    continue;
                }
                if let Some((text, style)) = header.get(column) {
                    self.put_aligned(cx, *segment, column, y, text, *style);
                }
            }
            y += 1;
        }
        let mut row = RowBuilder::default();
        let mut index = self.offset;
        while y < bottom && index < self.source.len() {
            self.source.row(index, &mut row);
            let selected = self.selected == Some(index);
            let odd = !selected && index % 2 == 1 && self.odd_row.is_some();
            let band = if selected {
                Some(self.highlight)
            } else if odd {
                self.odd_row
                    .map(|odd_style| patch_style(self.style, odd_style))
            } else {
                None
            };
            cx.put_str(area.x, y, &blank_row, band.unwrap_or(self.style));
            if let (true, Some((symbol, symbol_style))) = (selected, self.highlight_symbol) {
                let draw = clip_display_prefix(symbol, gutter);
                cx.put_str(area.x, y, draw, patch_style(self.highlight, symbol_style));
            }
            for (column, segment) in solved.segments.iter().enumerate() {
                if segment.width == 0 {
                    continue;
                }
                let base = if selected {
                    self.highlight
                } else if let (true, Some(odd_style)) = (odd, self.odd_row) {
                    patch_style(self.column_style(column), odd_style)
                } else {
                    self.column_style(column)
                };
                let style = patch_style(base, self.cell_styles.style_for(index, column));
                let text = row.cells.get(column).map_or("", String::as_str);
                self.put_aligned(cx, *segment, column, y, text, style);
            }
            y = y.saturating_add(1);
            for _ in 0..self.row_gap {
                if y >= bottom {
                    break;
                }
                cx.put_str(area.x, y, &blank_row, self.style);
                y += 1;
            }
            index += 1;
        }
        while y < bottom {
            cx.put_str(area.x, y, &blank_row, self.style);
            y += 1;
        }
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
