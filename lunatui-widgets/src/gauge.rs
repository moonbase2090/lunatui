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
    track_style: Option<Style>,
    sub_cell: bool,
}

impl Gauge {
    /// Gauge at `ratio` (clamped to 0..1).
    #[must_use]
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            block: None,
            fill_style: Style::new(),
            track_style: None,
            sub_cell: false,
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

    /// Style for the cells the fill has not reached. Without it, those cells are
    /// reset to blank default-style cells.
    #[must_use]
    pub fn track_style(mut self, style: Style) -> Self {
        self.track_style = Some(style);
        self
    }

    /// Draws the fractional end of the fill as a left eighth block (`▏` to `▉`).
    /// Any fraction past the last whole cell shows at least `▏`.
    #[must_use]
    pub fn sub_cell(mut self, enabled: bool) -> Self {
        self.sub_cell = enabled;
        self
    }
}

const LEFT_EIGHTHS: [(f32, &str); 7] = [
    (0.0, "▏"),
    (0.25, "▎"),
    (0.375, "▍"),
    (0.5, "▌"),
    (0.625, "▋"),
    (0.75, "▊"),
    (0.875, "▉"),
];

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
        let mut next = filled;
        if self.sub_cell && filled < inner.width {
            let fraction = self.ratio * f32::from(inner.width) - f32::from(filled);
            if fraction > 0.0 {
                let eighths = LEFT_EIGHTHS
                    .iter()
                    .take_while(|(threshold, _)| fraction >= *threshold)
                    .last()
                    .map_or(LEFT_EIGHTHS[0].1, |(_, glyph)| *glyph);
                cx.put_str(inner.x + filled, y, eighths, self.fill_style);
                next = filled + 1;
            }
        }
        if next < inner.width {
            let rest = " ".repeat(usize::from(inner.width - next));
            cx.put_str(
                inner.x + next,
                y,
                &rest,
                self.track_style.unwrap_or_default(),
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, avail.height.min(3))
    }
}
