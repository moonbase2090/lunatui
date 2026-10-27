// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};

use crate::{Block, RenderCx, Widget};

/// One-line text field with optional block chrome.
#[derive(Clone, Debug)]
pub struct TextInput {
    value: String,
    cursor: usize,
    block: Option<Block>,
    style: Style,
    cursor_style: Style,
    cursor_glyph: &'static str,
    field_style: Option<Style>,
    padding: u16,
    show_cursor: bool,
}

impl TextInput {
    /// Field contents and byte cursor index into `value`.
    #[must_use]
    pub fn new(value: impl Into<String>, cursor: usize) -> Self {
        Self {
            value: value.into(),
            cursor,
            block: None,
            style: Style::new(),
            cursor_style: Style::new(),
            cursor_glyph: "│",
            field_style: None,
            padding: 0,
            show_cursor: true,
        }
    }

    /// Optional surrounding block.
    #[must_use]
    pub fn block(mut self, block: Block) -> Self {
        self.block = Some(block);
        self
    }

    /// Text style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Cursor glyph style.
    #[must_use]
    pub fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }

    /// Cursor glyph, clipped to the space remaining in the field's inner area. The default is the
    /// one-cell vertical bar `│`; a multi-cell grapheme is omitted if it cannot fit fully.
    #[must_use]
    pub fn cursor_glyph(mut self, glyph: &'static str) -> Self {
        self.cursor_glyph = glyph;
        self
    }

    /// Style for every cell inside the block, drawn before the text.
    ///
    /// Without it, cells the text does not cover keep what was there.
    #[must_use]
    pub fn field_style(mut self, style: Style) -> Self {
        self.field_style = Some(style);
        self
    }

    /// Blank cells between the field edge and the text, on each side.
    #[must_use]
    pub fn padding(mut self, cells: u16) -> Self {
        self.padding = cells;
        self
    }

    /// Whether the cursor glyph is drawn. Pass `false` for an unfocused field.
    #[must_use]
    pub fn show_cursor(mut self, show: bool) -> Self {
        self.show_cursor = show;
        self
    }
}

impl Widget for TextInput {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let field = if let Some(block) = &self.block {
            block.render(area, cx);
            block.inner(area)
        } else {
            area
        };
        if field.width == 0 || field.height == 0 {
            return;
        }
        if let Some(style) = self.field_style {
            let blank = " ".repeat(usize::from(field.width));
            for row in field.y..field.y + field.height {
                cx.put_str(field.x, row, &blank, style);
            }
        }
        let pad = self.padding.min(field.width / 2);
        let inner = Rect::new(field.x + pad, field.y, field.width - 2 * pad, field.height);
        if inner.width == 0 {
            return;
        }
        let y = inner.y + inner.height / 2;
        let cursor_col = display_col_at_byte(&self.value, self.cursor);
        let start_col = window_start_col(cursor_col, inner.width);
        let start_byte = byte_at_display_col(&self.value, start_col);
        let window_col = display_col_at_byte(&self.value, start_byte);
        let window = self.value.get(start_byte..).unwrap_or("");
        let visible = clip_display_prefix(window, inner.width);
        cx.put_str(inner.x, y, visible, self.style);
        let rel = cursor_col.saturating_sub(window_col);
        let col = u16::try_from(rel).unwrap_or(0);
        if self.show_cursor && col < inner.width {
            let available = inner.width.saturating_sub(col);
            let cursor = clip_display_prefix(self.cursor_glyph, available);
            if !cursor.is_empty() {
                cx.put_str(inner.x + col, y, cursor, self.cursor_style);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, avail.height.min(3))
    }
}

fn display_col_at_byte(text: &str, byte: usize) -> usize {
    let prefix = text.get(..byte.min(text.len())).unwrap_or("");
    width(prefix)
}

fn window_start_col(cursor_col: usize, viewport_width: u16) -> usize {
    let viewport = usize::from(viewport_width);
    if viewport == 0 || cursor_col < viewport {
        return 0;
    }
    cursor_col.saturating_sub(viewport - 1)
}

fn byte_at_display_col(text: &str, target_col: usize) -> usize {
    if target_col == 0 {
        return 0;
    }
    let mut col = 0usize;
    let mut end = 0usize;
    for grapheme in unicode_segmentation::UnicodeSegmentation::graphemes(text, true) {
        let w = lunatui_core::cluster_display_width(grapheme);
        if col >= target_col {
            break;
        }
        col += w;
        end += grapheme.len();
    }
    end
}
