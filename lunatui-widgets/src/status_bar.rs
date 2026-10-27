// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};

use crate::{RenderCx, Widget};

/// Single-line status strip (dashboard footer parity).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBar {
    text: String,
    style: Style,
    left: Vec<(String, Style)>,
    right: Vec<(String, Style)>,
}

impl StatusBar {
    /// Status line text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::new(),
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    /// Text style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Segments drawn from the left edge, each with its own style.
    ///
    /// With any segments set, the whole row is filled with [`StatusBar::style`]
    /// first, and the text from [`StatusBar::new`] follows the left segments.
    #[must_use]
    pub fn left<T: Into<String>>(mut self, segments: impl IntoIterator<Item = (T, Style)>) -> Self {
        self.left = segments
            .into_iter()
            .map(|(text, style)| (text.into(), style))
            .collect();
        self
    }

    /// Segments drawn so the last one ends at the right edge, each with its own style.
    #[must_use]
    pub fn right<T: Into<String>>(
        mut self,
        segments: impl IntoIterator<Item = (T, Style)>,
    ) -> Self {
        self.right = segments
            .into_iter()
            .map(|(text, style)| (text.into(), style))
            .collect();
        self
    }
}

impl Widget for StatusBar {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if self.left.is_empty() && self.right.is_empty() {
            let draw = clip_display_prefix(&self.text, area.width);
            cx.put_str(area.x, area.y, draw, self.style);
            return;
        }
        let end = area.x + area.width;
        cx.put_str(
            area.x,
            area.y,
            &" ".repeat(usize::from(area.width)),
            self.style,
        );
        let right_width = self
            .right
            .iter()
            .map(|(text, _)| u16::try_from(width(text)).unwrap_or(u16::MAX))
            .fold(0_u16, u16::saturating_add);
        let mut x = area.x;
        let text = (self.text.as_str(), self.style);
        for (segment, style) in self
            .left
            .iter()
            .map(|(text, style)| (text.as_str(), *style))
            .chain([text])
        {
            if x >= end {
                break;
            }
            let draw = clip_display_prefix(segment, end - x);
            cx.put_str(x, area.y, draw, style);
            x = x.saturating_add(u16::try_from(width(draw)).unwrap_or(0));
        }
        let mut x = end.saturating_sub(right_width).max(area.x);
        for (segment, style) in &self.right {
            if x >= end {
                break;
            }
            let draw = clip_display_prefix(segment, end - x);
            cx.put_str(x, area.y, draw, *style);
            x = x.saturating_add(u16::try_from(width(draw)).unwrap_or(0));
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}
