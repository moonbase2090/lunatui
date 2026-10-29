// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_text::source::{RowBuilder, RowSource};

use crate::render_cx::patch_style;
use crate::{RenderCx, Widget};

/// Single-column list backed by a [`RowSource`].
///
/// Every row of the area is cleared across its full width, and the selected
/// row's highlight style covers the whole row.
#[derive(Clone, Debug)]
pub struct List<S> {
    source: S,
    style: Style,
    highlight: Style,
    offset: usize,
    selected: Option<usize>,
    highlight_symbol: Option<(&'static str, Style)>,
    icons: Vec<(String, Style)>,
    marks: Vec<Option<(String, Style)>>,
    mark_padding: u16,
}

impl<S: RowSource> List<S> {
    /// Wraps `source`.
    #[must_use]
    pub fn new(source: S) -> Self {
        Self {
            source,
            style: Style::new(),
            highlight: Style::new(),
            offset: 0,
            selected: None,
            highlight_symbol: None,
            icons: Vec::new(),
            marks: Vec::new(),
            mark_padding: 0,
        }
    }

    /// Default row style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style for the focused row.
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

    /// Where each visible row sits when the list is rendered into `area`, as `(index, rect)`
    /// pairs from the first visible row down, for hit testing. Each row is the full width of the
    /// area and one cell tall; rows past the end of the source are left out.
    #[must_use]
    pub fn row_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        let mut areas = Vec::new();
        if area.width == 0 {
            return areas;
        }
        for y in 0..area.height {
            let index = self.offset + usize::from(y);
            if index >= self.source.len() {
                break;
            }
            areas.push((index, Rect::new(area.x, area.y + y, area.width, 1)));
        }
        areas
    }

    /// Focused row (selection until runtime focus wiring lands).
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Marker drawn at the left edge of the focused row. Every row reserves its
    /// width, so the items stay aligned.
    #[must_use]
    pub fn highlight_symbol(mut self, symbol: &'static str, style: Style) -> Self {
        self.highlight_symbol = Some((symbol, style));
        self
    }

    /// One icon per row, drawn before the item text with one blank cell after it.
    /// It takes the row's background and keeps its own color and attributes.
    #[must_use]
    pub fn icons(mut self, icons: Vec<(String, Style)>) -> Self {
        self.icons = icons;
        self
    }

    /// An optional mark per row, right-aligned `padding` cells in from the right
    /// edge. It takes the row's background and keeps its own color and attributes.
    /// On a row with a mark, the item text is clipped so it ends before the mark.
    #[must_use]
    pub fn marks(mut self, marks: Vec<Option<(String, Style)>>, padding: u16) -> Self {
        self.marks = marks;
        self.mark_padding = padding;
        self
    }
}

impl<S: RowSource> Widget for List<S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let end = area.x + area.width;
        let gutter = self
            .highlight_symbol
            .map_or(0, |(symbol, _)| cell_width(symbol).min(area.width));
        let mut row = RowBuilder::default();
        for y in 0..area.height {
            let line = area.y + y;
            let index = self.offset + usize::from(y);
            if index >= self.source.len() {
                cx.fill(Rect::new(area.x, line, area.width, 1), self.style, ' ');
                continue;
            }
            let selected = self.selected == Some(index);
            let style = if selected { self.highlight } else { self.style };
            cx.fill(Rect::new(area.x, line, area.width, 1), style, ' ');
            self.source.row(index, &mut row);
            if let (true, Some((symbol, symbol_style))) = (selected, self.highlight_symbol) {
                let draw = clip_display_prefix(symbol, gutter);
                cx.put_str(area.x, line, draw, decoration(style, symbol_style));
            }
            let mut x = area.x + gutter;
            if let Some((icon, icon_style)) = self.icons.get(index) {
                let draw = clip_display_prefix(icon, end.saturating_sub(x));
                cx.put_str(x, line, draw, decoration(style, *icon_style));
                x = x
                    .saturating_add(cell_width(draw))
                    .saturating_add(1)
                    .min(end);
            }
            let mark = self.marks.get(index).and_then(Option::as_ref).map(|mark| {
                let mark_x = end
                    .saturating_sub(self.mark_padding)
                    .saturating_sub(cell_width(&mark.0))
                    .max(area.x);
                (mark_x, mark)
            });
            let text_end = mark.map_or(end, |(mark_x, _)| mark_x);
            let text = row.cells.first().map_or("", String::as_str);
            let draw = clip_display_prefix(text, text_end.saturating_sub(x));
            cx.put_str(x, line, draw, style);
            if let Some((mark_x, (mark, mark_style))) = mark {
                let draw = clip_display_prefix(mark, end.saturating_sub(mark_x));
                cx.put_str(mark_x, line, draw, decoration(style, *mark_style));
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = u16::try_from(self.source.len()).unwrap_or(u16::MAX);
        Size::new(avail.width, rows.min(avail.height))
    }
}

fn cell_width(text: &str) -> u16 {
    u16::try_from(width(text)).unwrap_or(u16::MAX)
}

fn decoration(row: Style, own: Style) -> Style {
    patch_style(Style::new().bg(row.bg), own)
}
