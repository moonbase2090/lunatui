// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix};

use crate::{Block, RenderCx, Widget};

/// Horizontal progress bar in `[0.0, 1.0]`.
#[derive(Clone, Debug)]
pub struct Gauge {
    ratio: f32,
    block: Option<Block>,
    fill_style: Style,
}

impl Gauge {
    /// Gauge at `ratio` (clamped to 0..1).
    #[must_use]
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            block: None,
            fill_style: Style::new(),
        }
    }

    /// Optional surrounding block.
    #[must_use]
    pub fn block(mut self, block: Block) -> Self {
        self.block = Some(block);
        self
    }

    /// Fill style.
    #[must_use]
    pub fn gauge_style(mut self, style: Style) -> Self {
        self.fill_style = style;
        self
    }
}

fn filled_columns(width: u16, ratio: f32) -> u16 {
    if width == 0 {
        return 0;
    }
    let clamped = ratio.clamp(0.0, 1.0);
    let mut filled = 0_u16;
    for col in 1..=width {
        let edge = f32::from(col) / f32::from(width);
        if edge <= clamped {
            filled = col;
        }
    }
    filled
}

impl Widget for Gauge {
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
        let filled = filled_columns(inner.width, self.ratio);
        let bar = "█".repeat(usize::from(filled));
        let draw = clip_display_prefix(&bar, inner.width);
        let y = inner.y + inner.height / 2;
        cx.put_str(inner.x, y, draw, self.fill_style);
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, avail.height.min(3))
    }
}
