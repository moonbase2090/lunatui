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
        match name {
            Some(name) => format!("{name} {}x{}", rect.width, rect.height),
            None => format!("{}x{}", rect.width, rect.height),
        }
    }
}

fn plot(cx: &mut RenderCx<'_>, area: Rect, x: u16, y: u16, glyph: &str, style: Style) {
    let inside = x >= area.x
        && y >= area.y
        && u32::from(x) < u32::from(area.x) + u32::from(area.width)
        && u32::from(y) < u32::from(area.y) + u32::from(area.height);
    if inside {
        let style = style.bg(cx.bg_at(x, y));
        cx.put_str(x, y, glyph, style);
    }
}

impl Widget for LayoutDebug {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if !self.enabled {
            return;
        }
        for (position, (rect, name)) in self.boxes.iter().enumerate() {
            if rect.width == 0 || rect.height == 0 {
                continue;
            }
            let color = cx.token(CYCLE[position % CYCLE.len()]);
            let style = Style::new().fg(color);
            let left = rect.x;
            let top = rect.y;
            let right = rect.x.saturating_add(rect.width - 1);
            let bottom = rect.y.saturating_add(rect.height - 1);
            for y in top..=bottom {
                for x in left..=right {
                    let on_x = x == left || x == right;
                    let on_y = y == top || y == bottom;
                    if !on_x && !on_y {
                        continue;
                    }
                    let glyph = match (on_x, on_y, x == left, y == top) {
                        (true, true, _, _) if rect.width == 1 && rect.height == 1 => "▪",
                        (true, true, _, _) if rect.height == 1 => "─",
                        (true, true, _, _) if rect.width == 1 => "│",
                        (true, true, true, true) => "┌",
                        (true, true, false, true) => "┐",
                        (true, true, true, false) => "└",
                        (true, true, false, false) => "┘",
                        (true, false, _, _) => "│",
                        _ => "─",
                    };
                    plot(cx, area, x, y, glyph, style);
                }
            }
            let label = Self::label(*rect, name.as_deref());
            let room = usize::from(rect.width.saturating_sub(2));
            if rect.width >= 3 && rect.height >= 2 && label.chars().count() <= room {
                let bold = style.with_attrs(Attrs::new().set_bold(true));
                for (offset, ch) in label.chars().enumerate() {
                    let x = left.saturating_add(1 + u16::try_from(offset).unwrap_or(u16::MAX));
                    plot(cx, area, x, top, ch.encode_utf8(&mut [0; 4]), bold);
                }
            }
        }
    }

    fn measure(&self, _avail: Size) -> Size {
        Size::new(0, 0)
    }
}
