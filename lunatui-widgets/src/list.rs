// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix};
use lunatui_text::source::{RowBuilder, RowSource};

use crate::{RenderCx, Widget};

/// Single-column list backed by a [`RowSource`].
#[derive(Clone, Debug)]
pub struct List<S> {
    source: S,
    style: Style,
    highlight: Style,
    /// Row index shown at the top of the viewport (vertical scroll).
    offset: usize,
    /// Focused row, highlighted when visible.
    selected: Option<usize>,
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

    /// Focused row (selection until runtime focus wiring lands).
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }
}

impl<S: RowSource> Widget for List<S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut row = RowBuilder::default();
        for y in 0..area.height {
            let index = self.offset + usize::from(y);
            if index >= self.source.len() {
                break;
            }
            self.source.row(index, &mut row);
            let text = row.cells.first().map_or("", String::as_str);
            let draw = clip_display_prefix(text, area.width);
            let style = if self.selected == Some(index) {
                self.highlight
            } else {
                self.style
            };
            cx.put_str(area.x, area.y + y, draw, style);
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
