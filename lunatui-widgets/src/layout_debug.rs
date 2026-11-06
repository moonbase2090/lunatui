// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style};
use lunatui_layout::Solved;
use lunatui_theme::Token;

use crate::{RenderCx, Widget};

const CYCLE: [Token; 4] = [Token::Accent, Token::Ok, Token::Warn, Token::Fail];

/// Outlines layout boxes with their sizes, to spot alignment bugs.
///
/// Add the rectangles a layout produced, then render the widget last over the finished screen.
/// Each box gets a one-cell outline and a `WxH` label in its top edge; a name added with
/// [`named`](Self::named) goes in front of the size. Outlines only replace the cells they sit
/// on and keep the background underneath. Boxes take their color from a four-color cycle of
/// theme tokens, so neighbors are easy to tell apart. Nothing is drawn outside the render area.
///
/// [`enabled`](Self::enabled) turns the overlay on or off without changing the call site, so an
/// app can bind it to a key.
///
/// # Example
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{LayoutDebug, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(12, 4);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// LayoutDebug::new()
///     .rect(Rect::new(0, 0, 12, 4))
///     .render(Rect::new(0, 0, 12, 4), &mut cx);
/// assert_eq!(grid.row_display(0, &clusters).trim_end(), "┌12x4──────┐");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDebug {
    boxes: Vec<(Rect, Option<String>)>,
    enabled: bool,
}

impl Default for LayoutDebug {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutDebug {
    /// An enabled overlay with no boxes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            boxes: Vec::new(),
            enabled: true,
        }
    }

    /// Adds one box.
    #[must_use]
    pub fn rect(mut self, rect: Rect) -> Self {
        self.boxes.push((rect, None));
        self
    }

    /// Adds one box whose label starts with `name`.
    #[must_use]
    pub fn named(mut self, name: &str, rect: Rect) -> Self {
        self.boxes.push((rect, Some(name.to_owned())));
        self
    }

    /// Adds every rectangle of `rects`, in order.
    #[must_use]
    pub fn rects(mut self, rects: impl IntoIterator<Item = Rect>) -> Self {
        self.boxes
            .extend(rects.into_iter().map(|rect| (rect, None)));
        self
    }

    /// Adds the segments of a solved split. Spacers are left out.
    #[must_use]
    pub fn solved(self, solved: &Solved) -> Self {
        self.rects(solved.segments.iter().copied())
    }

    /// Turns the overlay on or off. A disabled overlay draws nothing.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn label(rect: Rect, name: Option<&str>) -> String {
        let size = format!("{}x{}", rect.width, rect.height);
        match name {
            Some(name) => {
                let name: String = name.chars().filter(|&ch| cell_width(ch) > 0).collect();
                format!("{name} {size}")
            }
            None => size,
        }
    }
}

fn cell_width(ch: char) -> u16 {
    let mut buf = [0u8; 4];
    u16::try_from(lunatui_core::width(ch.encode_utf8(&mut buf))).unwrap_or(0)
}

fn visible(start: u32, len: u32, area_start: u16, area_len: u16) -> core::ops::Range<u32> {
    let lo = start.max(u32::from(area_start));
    let hi = (start + len).min(u32::from(area_start) + u32::from(area_len));
    lo..hi.max(lo)
}

fn edge_glyph(rect: Rect, x: u32, y: u32) -> &'static str {
    let left = x == u32::from(rect.x);
    let top = y == u32::from(rect.y);
    let right = x + 1 == u32::from(rect.x) + u32::from(rect.width);
    let bottom = y + 1 == u32::from(rect.y) + u32::from(rect.height);
    match (rect.width, rect.height) {
        (1, 1) => "▪",
        (_, 1) => "─",
        (1, _) => "│",
        _ => match (left || right, top || bottom, left, top) {
            (true, true, true, true) => "┌",
            (true, true, false, true) => "┐",
            (true, true, true, false) => "└",
            (true, true, false, false) => "┘",
            (true, false, _, _) => "│",
            _ => "─",
        },
    }
}

fn plot(cx: &mut RenderCx<'_>, x: u32, y: u32, glyph: &str, style: Style) {
    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
        return;
    };
    let style = style.bg(cx.bg_at(x, y));
    cx.put_str(x, y, glyph, style);
}

impl Widget for LayoutDebug {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if !self.enabled {
            return;
        }
        for (position, (rect, name)) in self.boxes.iter().enumerate() {
            let color = cx.token(CYCLE[position % CYCLE.len()]);
            let style = Style::new().fg(color);
            let (left, top) = (u32::from(rect.x), u32::from(rect.y));
            let (width, height) = (u32::from(rect.width), u32::from(rect.height));
            let columns = visible(left, width, area.x, area.width);
            let rows = visible(top, height, area.y, area.height);
            let horizontal = [top, (top + height).saturating_sub(1)];
            for &y in &horizontal[..if height == 1 { 1 } else { 2 }] {
                if rows.contains(&y) {
                    for x in columns.clone() {
                        plot(cx, x, y, edge_glyph(*rect, x, y), style);
                    }
                }
            }
            let vertical = [left, (left + width).saturating_sub(1)];
            for &x in &vertical[..if width == 1 { 1 } else { 2 }] {
                if columns.contains(&x) {
                    for y in rows.clone() {
                        plot(cx, x, y, edge_glyph(*rect, x, y), style);
                    }
                }
            }
            if rect.height < 2 || !rows.contains(&top) {
                continue;
            }
            let label = Self::label(*rect, name.as_deref());
            if lunatui_core::width(&label) > usize::from(rect.width.saturating_sub(2)) {
                continue;
            }
            let bold = style.with_attrs(Attrs::new().set_bold(true));
            let mut x = left + 1;
            for ch in label.chars() {
                let cells = u32::from(cell_width(ch));
                let inside = x >= columns.start && x + cells <= columns.end;
                if inside {
                    plot(cx, x, top, ch.encode_utf8(&mut [0; 4]), bold);
                }
                x += cells;
            }
        }
    }

    fn measure(&self, _avail: Size) -> Size {
        Size::new(0, 0)
    }
}
