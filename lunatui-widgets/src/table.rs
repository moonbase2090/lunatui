// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix};
use lunatui_layout::{Constraint, Flex, Layout};
use lunatui_text::source::{RowBuilder, RowSource};

use crate::{RenderCx, Widget};

/// Column layout for [`Table`].
#[derive(Clone, Debug)]
pub struct Table<S> {
    source: S,
    columns: Vec<Constraint>,
    style: Style,
    highlight: Style,
    offset: usize,
    selected: Option<usize>,
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
        }
    }

    /// Default cell style.
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

    /// Focused row.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }
}

impl<S: RowSource> Widget for Table<S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 || self.columns.is_empty() {
            return;
        }
        let layout = Layout::horizontal(self.columns.iter().copied()).flex(Flex::Legacy);
        let solved = layout.split(area);
        let mut row = RowBuilder::default();
        for y in 0..area.height {
            let index = self.offset + usize::from(y);
            if index >= self.source.len() {
                break;
            }
            self.source.row(index, &mut row);
            let style = if self.selected == Some(index) {
                self.highlight
            } else {
                self.style
            };
            for (col, segment) in solved.segments.iter().enumerate() {
                if segment.width == 0 {
                    continue;
                }
                let text = row.cells.get(col).map_or("", String::as_str);
                let draw = clip_display_prefix(text, segment.width);
                cx.put_str(segment.x, area.y + y, draw, style);
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
