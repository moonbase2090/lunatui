// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix};
use lunatui_text::wrap::{RowIndex, Wrap, WrapCache, wrap_rows};

use crate::{RenderCx, Widget};

/// How to wrap overflowing lines (ratatui `Wrap` parity).
pub use lunatui_text::wrap::Wrap as TextWrap;

/// Scroll offset as `(vertical_rows, horizontal_columns)`.
pub type Scroll = (u16, u16);

/// Static paragraph: re-wraps each [`render`](Widget::render) (fine for small text).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paragraph {
    text: String,
    style: Style,
    wrap: Option<Wrap>,
    scroll: Scroll,
}

impl Paragraph {
    /// Creates a paragraph from plain text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::new(),
            wrap: None,
            scroll: (0, 0),
        }
    }

    /// Enables word wrapping.
    #[must_use]
    pub fn wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = Some(wrap);
        self
    }

    /// Vertical and horizontal scroll offset in rows/columns.
    #[must_use]
    pub fn scroll(mut self, offset: Scroll) -> Self {
        self.scroll = offset;
        self
    }

    /// Style applied to every glyph.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Borrowed text body.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    fn row_index(&self, width: u16) -> RowIndex {
        if let Some(wrap) = self.wrap {
            wrap_rows(&self.text, width, wrap)
        } else {
            unwrap_rows(&self.text)
        }
    }
}

impl Widget for Paragraph {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let index = self.row_index(area.width);
        paint_rows(
            &index,
            &self.text,
            area,
            self.scroll,
            self.wrap.is_some(),
            self.style,
            cx,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = self.row_index(avail.width).len();
        Size::new(
            avail.width,
            u16::try_from(rows).unwrap_or(u16::MAX).min(avail.height),
        )
    }
}

/// Paragraph with a persistent [`WrapCache`] for large logs (call [`render`](Self::render)).
#[derive(Clone, Debug, Default)]
pub struct TextView {
    paragraph: Paragraph,
    cache: WrapCache,
}

impl TextView {
    /// Creates a text view from plain text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            paragraph: Paragraph::new(text),
            cache: WrapCache::new(),
        }
    }

    /// Enables word wrapping.
    #[must_use]
    pub fn wrap(mut self, wrap: Wrap) -> Self {
        self.paragraph = self.paragraph.wrap(wrap);
        self
    }

    /// Scroll offset.
    #[must_use]
    pub fn scroll(mut self, offset: Scroll) -> Self {
        self.paragraph = self.paragraph.scroll(offset);
        self
    }

    /// Style applied to every glyph.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.paragraph = self.paragraph.style(style);
        self
    }

    /// Mutable access to the wrap cache (for benchmarks).
    #[must_use]
    pub fn cache_mut(&mut self) -> &mut WrapCache {
        &mut self.cache
    }

    /// Paints using the cached wrap index.
    pub fn render(&mut self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if let Some(wrap) = self.paragraph.wrap {
            self.cache.sync(self.paragraph.text(), area.width, wrap);
            paint_rows(
                self.cache.index(),
                self.paragraph.text(),
                area,
                self.paragraph.scroll,
                true,
                self.paragraph.style,
                cx,
            );
        } else {
            let index = unwrap_rows(self.paragraph.text());
            paint_rows(
                &index,
                self.paragraph.text(),
                area,
                self.paragraph.scroll,
                false,
                self.paragraph.style,
                cx,
            );
        }
    }
}

fn unwrap_rows(text: &str) -> RowIndex {
    let mut row_starts = vec![0];
    for (index, _) in text.match_indices('\n') {
        row_starts.push(index + 1);
    }
    RowIndex { row_starts }
}

fn paint_rows(
    index: &RowIndex,
    text: &str,
    area: Rect,
    scroll: Scroll,
    _wrapped: bool,
    style: Style,
    cx: &mut RenderCx<'_>,
) {
    let first_row = usize::from(scroll.0);
    let h_scroll = scroll.1;
    for row in 0..area.height {
        let visual = first_row + usize::from(row);
        let line = index.row_text(text, visual).unwrap_or("");
        let line = skip_columns(line, h_scroll);
        let draw = clip_display_prefix(line, area.width);
        cx.put_str(area.x, area.y + row, draw, style);
    }
}

fn skip_columns(text: &str, skip: u16) -> &str {
    if skip == 0 {
        return text;
    }
    let mut cols = 0u16;
    let mut end = 0usize;
    for grapheme in unicode_segmentation::UnicodeSegmentation::graphemes(text, true) {
        let w = u16::try_from(lunatui_core::cluster_display_width(grapheme)).unwrap_or(1);
        if cols + w > skip {
            break;
        }
        cols += w;
        end += grapheme.len();
    }
    text.get(end..).unwrap_or("")
}
