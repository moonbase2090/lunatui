// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix};

use crate::{RenderCx, Widget};

/// Single-line status strip (dashboard footer parity).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBar {
    text: String,
    style: Style,
}

impl StatusBar {
    /// Status line text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::new(),
        }
    }

    /// Text style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for StatusBar {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let draw = clip_display_prefix(&self.text, area.width);
        cx.put_str(area.x, area.y, draw, self.style);
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}
