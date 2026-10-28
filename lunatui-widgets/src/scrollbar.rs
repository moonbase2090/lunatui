// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget};

/// Scrollbar for a virtualized viewport, vertical by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scrollbar {
    horizontal: bool,
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
            horizontal: false,
            content_len,
            viewport_len,
            offset,
            track_style: Style::new(),
            thumb_style: Style::new(),
        }
    }

    /// Horizontal scrollbar for `content_len` columns, `viewport_len` visible, scrolled to
    /// `offset`. It draws along the top row of its area, with `─` for the track and `━` for the
    /// thumb.
    #[cfg(feature = "scroll-view")]
    #[must_use]
    pub fn horizontal(content_len: usize, viewport_len: usize, offset: usize) -> Self {
        Self {
            horizontal: true,
            ..Self::vertical(content_len, viewport_len, offset)
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
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 || self.content_len == 0 {
            return;
        }
        let (track_len, track, thumb) = if self.horizontal {
            (area.width, "─", "━")
        } else {
            (area.height, "│", "█")
        };
        let thumb_len = thumb_size(self.content_len, usize::from(track_len), self.viewport_len);
        let thumb_at = thumb_position(
            self.offset,
            self.content_len,
            self.viewport_len,
            usize::from(track_len),
            thumb_len,
        );
        for step in 0..track_len {
            let (x, y) = if self.horizontal {
                (area.x.saturating_add(step), area.y)
            } else {
                (area.x, area.y.saturating_add(step))
            };
            if step >= thumb_at && step < thumb_at.saturating_add(thumb_len) {
                cx.put_str(x, y, thumb, self.thumb_style);
            } else {
                cx.put_str(x, y, track, self.track_style);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        if self.horizontal {
            Size::new(avail.width, 1.min(avail.height))
        } else {
            Size::new(1.min(avail.width), avail.height)
        }
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
    if offset >= scrollable {
        return u16::try_from(max_y).unwrap_or(0);
    }
    let y = (offset as u128) * (max_y as u128) / (scrollable as u128);
    u16::try_from(usize::try_from(y).unwrap_or(max_y)).unwrap_or(0)
}
