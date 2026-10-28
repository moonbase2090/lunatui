// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};

use crate::{RenderCx, Widget};

const ELLIPSIS: &str = "…";

/// A path of clickable segments on one row, such as `Home › Projects › lunatui › src`.
///
/// When the path is wider than the area, the middle collapses to `…`: the first segment and as
/// many of the last ones as fit stay, and a last segment that still does not fit is cut with `…`.
/// [`Breadcrumbs::segment_areas`] reports where each shown segment sits, for hit testing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breadcrumbs {
    segments: Vec<String>,
    separator: String,
    style: Style,
    current: Style,
    separator_style: Style,
    hover: Style,
    hovered: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Segment(usize, String),
    Gap,
    Separator,
}

impl Breadcrumbs {
    /// Path segments in order, root first.
    #[must_use]
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
            separator: String::from(" › "),
            style: Style::new(),
            current: Style::new(),
            separator_style: Style::new(),
            hover: Style::new(),
            hovered: None,
        }
    }

    /// Text between segments. The default is ` › `.
    #[must_use]
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Style of every segment except the last, and of the collapsed `…`.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style of the last segment, the place the path ends.
    #[must_use]
    pub fn current_style(mut self, style: Style) -> Self {
        self.current = style;
        self
    }

    /// Style of the separators.
    #[must_use]
    pub fn separator_style(mut self, style: Style) -> Self {
        self.separator_style = style;
        self
    }

    /// Style laid over the segment at `index` while the pointer is on it. An index that is not a
    /// shown segment changes nothing.
    #[must_use]
    pub fn hover(mut self, index: Option<usize>, style: Style) -> Self {
        self.hovered = index;
        self.hover = style;
        self
    }

    fn pieces(&self, avail: u16) -> Vec<Piece> {
        let count = self.segments.len();
        let sep = u16::try_from(width(&self.separator)).unwrap_or(u16::MAX);
        let cells = |text: &str| u16::try_from(width(text)).unwrap_or(u16::MAX);
        let fits = |picked: &[Option<usize>]| {
            let mut total: u16 = 0;
            for (position, entry) in picked.iter().enumerate() {
                if position > 0 {
                    total = total.saturating_add(sep);
                }
                total = total.saturating_add(match entry {
                    Some(index) => cells(&self.segments[*index]),
                    None => cells(ELLIPSIS),
                });
            }
            total <= avail
        };
        let all: Vec<Option<usize>> = (0..count).map(Some).collect();
        if count == 0 || fits(&all) {
            return self.assemble(&all, avail);
        }
        for kept in (1..count.saturating_sub(1)).rev() {
            let mut picked = vec![Some(0), None];
            picked.extend((count - kept..count).map(Some));
            if fits(&picked) {
                return self.assemble(&picked, avail);
            }
        }
        let tail = [None, Some(count - 1)];
        if fits(&tail) {
            return self.assemble(&tail, avail);
        }
        self.assemble(&[Some(count - 1)], avail)
    }

    fn assemble(&self, picked: &[Option<usize>], avail: u16) -> Vec<Piece> {
        let mut pieces = Vec::new();
        for (position, entry) in picked.iter().enumerate() {
            if position > 0 {
                pieces.push(Piece::Separator);
            }
            match entry {
                Some(index) => {
                    let text = &self.segments[*index];
                    let room = usize::from(avail);
                    let shown = if picked.len() == 1 && width(text) > room {
                        let kept = clip_display_prefix(text, avail.saturating_sub(1));
                        format!("{kept}{ELLIPSIS}")
                    } else {
                        text.clone()
                    };
                    pieces.push(Piece::Segment(*index, shown));
                }
                None => pieces.push(Piece::Gap),
            }
        }
        pieces
    }

    /// Where each shown segment sits when rendered into `area`, as `(index, rect)` pairs in path
    /// order, for hit testing. Collapsed segments and the `…` that stands for them have no entry.
    /// A single remaining segment that had to be cut covers the cells that show.
    #[must_use]
    pub fn segment_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        let mut areas = Vec::new();
        if area.width == 0 || area.height == 0 {
            return areas;
        }
        let mut x = area.x;
        for piece in self.pieces(area.width) {
            let (text, index) = match &piece {
                Piece::Segment(index, text) => (text.as_str(), Some(*index)),
                Piece::Gap => (ELLIPSIS, None),
                Piece::Separator => (self.separator.as_str(), None),
            };
            let cells = u16::try_from(width(text)).unwrap_or(u16::MAX);
            if let Some(index) = index {
                areas.push((index, Rect::new(x, area.y, cells, 1)));
            }
            x = x.saturating_add(cells);
        }
        areas
    }
}

impl Widget for Breadcrumbs {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let blank = " ".repeat(usize::from(area.width));
        cx.put_str(area.x, area.y, &blank, self.style);
        let last = self.segments.len().saturating_sub(1);
        let mut x = area.x;
        for piece in self.pieces(area.width) {
            let (text, style) = match &piece {
                Piece::Segment(index, text) => {
                    let base = if *index == last {
                        self.current
                    } else {
                        self.style
                    };
                    let style = if self.hovered == Some(*index) {
                        patch(base, self.hover)
                    } else {
                        base
                    };
                    (text.as_str(), style)
                }
                Piece::Gap => (ELLIPSIS, self.style),
                Piece::Separator => (self.separator.as_str(), self.separator_style),
            };
            cx.put_str(x, area.y, text, style);
            x = x.saturating_add(u16::try_from(width(text)).unwrap_or(u16::MAX));
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}

fn patch(base: Style, over: Style) -> Style {
    crate::render_cx::patch_style(base, over)
}
