// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};

use crate::{RenderCx, Widget};

/// One-row progress line: `━` for the filled part and `─` for the rest.
///
/// The filled part rounds up to a whole cell, so any progress above zero shows.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{LineGauge, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(8, 1);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// LineGauge::new(0.5).render(Rect::new(0, 0, 8, 1), &mut cx);
/// assert_eq!(grid.row_display(0, &clusters), "━━━━────");
/// ```
#[derive(Clone, Debug)]
pub struct LineGauge {
    ratio: f32,
    filled: Style,
    unfilled: Style,
}

impl LineGauge {
    /// Line at `ratio`, clamped to 0..1.
    #[must_use]
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            filled: Style::new(),
            unfilled: Style::new(),
        }
    }

    /// Style of the filled part.
    #[must_use]
    pub fn filled_style(mut self, style: Style) -> Self {
        self.filled = style;
        self
    }

    /// Style of the unfilled part.
    #[must_use]
    pub fn unfilled_style(mut self, style: Style) -> Self {
        self.unfilled = style;
        self
    }
}

fn filled_cells(width: u16, ratio: f32) -> u16 {
    (0..width)
        .take_while(|&cell| f32::from(cell) < ratio * f32::from(width))
        .fold(0, |count, _| count + 1)
}

impl Widget for LineGauge {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let filled = filled_cells(area.width, self.ratio);
        let y = area.y + area.height / 2;
        cx.put_str(area.x, y, &"━".repeat(usize::from(filled)), self.filled);
        cx.put_str(
            area.x + filled,
            y,
            &"─".repeat(usize::from(area.width - filled)),
            self.unfilled,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}
