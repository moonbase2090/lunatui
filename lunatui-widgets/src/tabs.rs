// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};

use crate::{RenderCx, Widget};

/// Horizontal tab strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tabs {
    titles: Vec<String>,
    selected: usize,
    style: Style,
    highlight: Style,
}

impl Tabs {
    /// Tab labels in order.
    #[must_use]
    pub fn new(titles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            titles: titles.into_iter().map(Into::into).collect(),
            selected: 0,
            style: Style::new(),
            highlight: Style::new(),
        }
    }

    /// Selected tab index.
    #[must_use]
    pub fn select(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Unselected tab style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Selected tab style.
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = style;
        self
    }
}

impl Widget for Tabs {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut x = area.x;
        for (index, title) in self.titles.iter().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let remaining = area.x + area.width - x;
            let draw = clip_display_prefix(title, remaining);
            let w = u16::try_from(width(draw)).unwrap_or(remaining);
            let style = if index == self.selected {
                self.highlight
            } else {
                self.style
            };
            cx.put_str(x, area.y, draw, style);
            x = x.saturating_add(w);
            if index + 1 < self.titles.len() && x < area.x + area.width {
                cx.put_str(x, area.y, " ", self.style);
                x = x.saturating_add(1);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}
