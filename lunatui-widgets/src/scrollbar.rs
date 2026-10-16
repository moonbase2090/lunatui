// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};

use crate::{RenderCx, Widget};

/// Vertical scrollbar for a virtualized viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scrollbar {
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    track_style: Style,
    thumb_style: Style,
}

impl Scrollbar {
    /// Scrollbar for `content_len` rows, `viewport_len` visible, scrolled to `offset`.
    #[must_use]
    pub fn vertical(content_len: usize, viewport_len: usize, offset: usize) -> Self {
        Self {
            content_len,
            viewport_len,
            offset,
            track_style: Style::new(),
            thumb_style: Style::new(),
        }
    }

    /// Track glyph style.
    #[must_use]
    pub fn track_style(mut self, style: Style) -> Self {
        self.track_style = style;
        self
    }

    /// Thumb glyph style.
    #[must_use]
    pub fn thumb_style(mut self, style: Style) -> Self {
        self.thumb_style = style;
        self
    }
}

impl Widget for Scrollbar {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 || self.content_len == 0 {
            return;
        }
        let track_h = area.height;
        let thumb_h = thumb_size(self.content_len, usize::from(track_h), self.viewport_len);
        let thumb_y = thumb_position(
            self.offset,
            self.content_len,
            self.viewport_len,
            usize::from(track_h),
            thumb_h,
        );
        for row in 0..track_h {
            let y = area.y + row;
            let thumb = row >= thumb_y && row < thumb_y.saturating_add(thumb_h);
            if thumb {
                cx.put_str(area.x, y, "█", self.thumb_style);
            } else {
                cx.put_str(area.x, y, "│", self.track_style);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(1.min(avail.width), avail.height)
    }
}

fn thumb_size(content: usize, track: usize, viewport: usize) -> u16 {
    if content == 0 {
        return 1;
    }
    let max_thumb = track.max(1);
    let min_thumb = 1usize;
    let proportional = track
        .saturating_mul(viewport)
        .div_ceil(content)
        .max(min_thumb);
    u16::try_from(proportional.min(max_thumb)).unwrap_or(1)
}

fn thumb_position(offset: usize, content: usize, viewport: usize, track: usize, thumb: u16) -> u16 {
    if content <= viewport {
        return 0;
    }
    let scrollable = content.saturating_sub(viewport);
    let max_y = track.saturating_sub(usize::from(thumb));
    if scrollable == 0 {
        return 0;
    }
    let offset = offset.min(scrollable);
    let y = offset.saturating_mul(max_y) / scrollable;
    u16::try_from(y).unwrap_or(0)
}
