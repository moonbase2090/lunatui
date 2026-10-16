// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};

use crate::{RenderCx, Widget};

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Braille animation frame (ratatui dashboard spinner parity).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spinner {
    frame: u32,
    style: Style,
}

impl Spinner {
    /// Spinner at animation frame `frame`.
    #[must_use]
    pub fn new(frame: u32) -> Self {
        Self {
            frame,
            style: Style::new(),
        }
    }

    /// Glyph color.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Spinner {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let index = usize::try_from(self.frame % 10).unwrap_or(0);
        cx.put_str(area.x, area.y, FRAMES[index], self.style);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(1, 1).clamp(avail)
    }
}
