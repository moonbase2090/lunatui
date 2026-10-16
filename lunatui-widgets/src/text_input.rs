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
}

impl Widget for TextInput {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let inner = if let Some(block) = &self.block {
            block.render(area, cx);
            block.inner(area)
        } else {
            area
        };
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let y = inner.y + inner.height / 2;
        let visible = clip_display_prefix(&self.value, inner.width);
        cx.put_str(inner.x, y, visible, self.style);
        let cursor_col = display_col_at_byte(&self.value, self.cursor);
        let col = u16::try_from(cursor_col).unwrap_or(0);
        if col < inner.width {
            cx.put_str(inner.x + col, y, "│", self.cursor_style);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width.max(8), avail.height.min(3))
    }
}

fn display_col_at_byte(text: &str, byte: usize) -> usize {
    let prefix = text.get(..byte.min(text.len())).unwrap_or("");
    width(prefix)
}
