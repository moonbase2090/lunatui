// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::render_cx::patch_style;
use crate::{RenderCx, Widget};

/// Vertical bar chart: one column per value, bars rising from the bottom of the area.
///
/// Each bar is `width` cells wide with a gap between bars (one cell by default), and its height
/// is the value scaled to the largest value (or to `max`). Partial heights use eighth blocks, so
/// bar tops move smoothly; a level below one eighth of a row leaves that row blank. Bars use the
/// accent token unless a style is set. Rendering first blanks the whole area, keeping each
/// cell's background, so a second render leaves nothing behind.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{BarChart, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(5, 2);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// BarChart::new(&[0, 8]).render(Rect::new(0, 0, 5, 2), &mut cx);
/// assert_eq!(grid.row_display(1, &clusters), "  █  ");
/// ```
#[derive(Clone, Debug)]
pub struct BarChart<'a> {
    values: &'a [u64],
    max: Option<u64>,
    bar_width: u16,
    bar_gap: u16,
    style: Option<Style>,
    bar_styles: &'a [Style],
    track: Option<Style>,
    labels: &'a [&'a str],
    label_style: Style,
}

const EIGHTHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

impl<'a> BarChart<'a> {
    /// A chart over `values`, one bar each, one cell wide with a one-cell gap.
    #[must_use]
    pub fn new(values: &'a [u64]) -> Self {
        Self {
            values,
            max: None,
            bar_width: 1,
            bar_gap: 1,
            style: None,
            bar_styles: &[],
            track: None,
            labels: &[],
            label_style: Style::new(),
        }
    }

    /// Value that fills a whole column; defaults to the largest value.
    #[must_use]
    pub const fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Width of each bar in cells, at least 1.
    #[must_use]
    pub const fn bar_width(mut self, width: u16) -> Self {
        self.bar_width = if width == 0 { 1 } else { width };
        self
    }

    /// Blank cells between bars. The default is 1.
    #[must_use]
    pub const fn bar_gap(mut self, gap: u16) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Style of the bars, replacing the accent token.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Per-bar styles layered over the chart style, in bar order. Bars past the
    /// end of the slice use the chart style.
    #[must_use]
    pub const fn bar_styles(mut self, styles: &'a [Style]) -> Self {
        self.bar_styles = styles;
        self
    }

    /// Style for every cell of each bar's column, under the bar itself, so the
    /// empty part of a bar shows as a track.
    #[must_use]
    pub const fn track_style(mut self, style: Style) -> Self {
        self.track = Some(style);
        self
    }

    /// One label per bar, drawn left-aligned under it on the bottom row and
    /// clipped to the bar's width.
    #[must_use]
    pub const fn labels(mut self, labels: &'a [&'a str]) -> Self {
        self.labels = labels;
        self
    }

    /// Style for the labels.
    #[must_use]
    pub const fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }
}

fn eighths(value: u64, max: u64, rows: u16) -> u64 {
    if max == 0 {
        return 0;
    }
    let ticks = u128::from(value) * u128::from(rows) * 8 / u128::from(max);
    u64::try_from(ticks.min(u128::from(rows) * 8)).unwrap_or(0)
}

impl Widget for BarChart<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 || self.values.is_empty() {
            return;
        }
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                cx.put_char_patched(x, y, ' ', Style::new());
            }
        }
        let max = self
            .max
            .unwrap_or_else(|| self.values.iter().copied().max().unwrap_or(0));
        let base = self
            .style
            .unwrap_or_else(|| Style::new().fg(cx.token(Token::Accent)));
        let label_rows = u16::from(!self.labels.is_empty() && area.height > 1);
        let rows = area.height - label_rows;
        let stride = self.bar_width.saturating_add(self.bar_gap);
        let bottom = area.y + rows;
        let right = area.x + area.width;
        for (index, &value) in self.values.iter().enumerate() {
            let x = area.x.saturating_add(
                u16::try_from(index)
                    .unwrap_or(u16::MAX)
                    .saturating_mul(stride),
            );
            if x >= right {
                break;
            }
            let cells = self.bar_width.min(right - x);
            let bar = match self.bar_styles.get(index) {
                Some(style) => patch_style(base, *style),
                None => base,
            };
            let filled = match self.track {
                Some(track) => patch_style(track, bar),
                None => bar,
            };
            let ticks = eighths(value, max, rows);
            let whole = u16::try_from(ticks / 8).unwrap_or(0);
            let partial = usize::try_from(ticks % 8).unwrap_or(0);
            for row in 0..rows {
                let y = bottom - 1 - row;
                let glyph = if row < whole {
                    '█'
                } else if row == whole && partial > 0 {
                    EIGHTHS[partial]
                } else {
                    ' '
                };
                if glyph == ' ' && self.track.is_none() {
                    continue;
                }
                let style = if glyph == ' ' {
                    self.track.unwrap_or(bar)
                } else {
                    filled
                };
                let mut buffer = [0_u8; 4];
                let symbol = glyph.encode_utf8(&mut buffer);
                for column in 0..cells {
                    cx.put_str(x + column, y, symbol, style);
                }
            }
            if label_rows == 1
                && let Some(label) = self.labels.get(index)
            {
                let draw = clip_display_prefix(label, cells);
                if u16::try_from(width(draw)).unwrap_or(0) > 0 {
                    cx.put_str(x, bottom, draw, self.label_style);
                }
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, avail.height).clamp(avail)
    }
}
