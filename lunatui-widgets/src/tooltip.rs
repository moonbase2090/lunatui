// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use core::time::Duration;

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::anchor::{AnchorSide, place};
use crate::{RenderCx, Widget};

const DEFAULT_MAX_WIDTH: u16 = 40;
const MIN_WIDTH: u16 = 3;
const DEFAULT_DELAY: Duration = Duration::from_millis(500);

/// A short hint drawn beside the cells it explains, its anchor.
///
/// The tooltip never covers its anchor. It opens on its [`side`](Self::side) and flips to the
/// opposite side, then to the other two, when the side has no room inside the area; when no side
/// has room it is not drawn at all. Along the side it starts aligned with the anchor and slides to
/// stay inside the area.
///
/// The text wraps at spaces so the tooltip, with one cell of padding on each side, is no wider
/// than [`max_width`](Self::max_width); a word too long for a row is broken, and `\n` starts a new
/// row. The tooltip is as wide as its widest row. It is drawn in `text.primary` on `surface.1`
/// unless [`style`](Self::style) is set. Rendering does not allocate.
///
/// [`area`](Self::area) gives the cells it covers, for hit testing, and [`TooltipState`] decides
/// when it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tooltip {
    text: String,
    anchor: Rect,
    side: AnchorSide,
    max_width: u16,
    style: Option<Style>,
}

impl Tooltip {
    /// A tooltip showing `text` for the cells in `anchor`, opening below it.
    #[must_use]
    pub fn new(text: impl Into<String>, anchor: Rect) -> Self {
        Self {
            text: text.into(),
            anchor,
            side: AnchorSide::Below,
            max_width: DEFAULT_MAX_WIDTH,
            style: None,
        }
    }

    /// The side tried first.
    #[must_use]
    pub const fn side(mut self, side: AnchorSide) -> Self {
        self.side = side;
        self
    }

    /// Widest the tooltip may be, padding included; at least three cells. The default is 40.
    #[must_use]
    pub fn max_width(mut self, cells: u16) -> Self {
        self.max_width = cells.max(MIN_WIDTH);
        self
    }

    /// Draws the tooltip in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// The anchor the tooltip explains.
    #[must_use]
    pub const fn anchor(&self) -> Rect {
        self.anchor
    }

    /// The cells the tooltip covers inside `area`, or `None` when no side of the anchor has room.
    #[must_use]
    pub fn area(&self, area: Rect) -> Option<Rect> {
        let size = self.size(area.width.min(self.max_width), area.height)?;
        place(self.anchor, size, area, self.side)
    }

    fn size(&self, width_limit: u16, height_limit: u16) -> Option<Size> {
        let inner = width_limit.checked_sub(2).filter(|inner| *inner > 0)?;
        let (mut widest, mut rows) = (0_u16, 0_u16);
        for row in Rows::new(&self.text, inner) {
            widest = widest.max(u16::try_from(width(row)).unwrap_or(u16::MAX));
            rows = rows.saturating_add(1);
        }
        (rows > 0 && rows <= height_limit).then(|| Size::new(widest.saturating_add(2), rows))
    }
}

impl Widget for Tooltip {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let Some(bubble) = self.area(area) else {
            return;
        };
        let style = self.style.unwrap_or_else(|| {
            Style::new()
                .fg(cx.token(Token::TextPrimary))
                .bg(cx.token(Token::Surface1))
        });
        cx.fill(bubble, style, ' ');
        let rows = Rows::new(&self.text, bubble.width - 2);
        for (offset, row) in (0..bubble.height).zip(rows) {
            cx.put_str(bubble.x + 1, bubble.y + offset, row, style);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        self.size(avail.width.min(self.max_width), avail.height)
            .unwrap_or(Size::new(0, 0))
    }
}

struct Rows<'t> {
    rest: Option<&'t str>,
    width: u16,
}

impl<'t> Rows<'t> {
    const fn new(text: &'t str, width: u16) -> Self {
        Self {
            rest: if text.is_empty() { None } else { Some(text) },
            width,
        }
    }
}

impl<'t> Iterator for Rows<'t> {
    type Item = &'t str;

    fn next(&mut self) -> Option<&'t str> {
        let rest = self.rest?;
        let limit = usize::from(self.width);
        let (line, after) = match rest.split_once('\n') {
            Some((line, after)) => (line, Some(after).filter(|after| !after.is_empty())),
            None => (rest, None),
        };
        if width(line) <= limit {
            self.rest = after;
            return Some(line.trim_end());
        }
        let mut cut = None;
        for (index, ch) in line.char_indices() {
            if ch != ' ' {
                continue;
            }
            if width(&line[..index]) > limit {
                break;
            }
            cut = Some(index);
        }
        let row = if let Some(index) = cut.filter(|index| *index > 0) {
            &line[..index]
        } else {
            let prefix = clip_display_prefix(line, self.width);
            if prefix.is_empty() {
                line.chars()
                    .next()
                    .map_or(line, |ch| &line[..ch.len_utf8()])
            } else {
                prefix
            }
        };
        let left = rest[row.len()..].trim_start_matches(' ');
        self.rest = Some(left).filter(|left| !left.is_empty());
        Some(row.trim_end())
    }
}

/// When a [`Tooltip`] shows, with time passed in by the caller.
///
/// It shows once the pointer has rested on the anchor for the delay (half a second by default)
/// and hides as soon as it leaves. [`dismiss`](Self::dismiss), for Escape or a click, hides it
/// until the pointer leaves the anchor and comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooltipState {
    delay: Duration,
    since: Option<Duration>,
    dismissed: bool,
}

impl Default for TooltipState {
    fn default() -> Self {
        Self::new()
    }
}

impl TooltipState {
    /// Shows after the default delay of half a second.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_delay(DEFAULT_DELAY)
    }

    /// Shows after the pointer has rested on the anchor for `delay`.
    #[must_use]
    pub const fn with_delay(delay: Duration) -> Self {
        Self {
            delay,
            since: None,
            dismissed: false,
        }
    }

    /// Records whether the pointer is `over` the anchor at time `now`.
    pub fn hover(&mut self, over: bool, now: Duration) {
        if over {
            if self.since.is_none() {
                self.since = Some(now);
            }
        } else {
            self.since = None;
            self.dismissed = false;
        }
    }

    /// Hides the tooltip until the pointer leaves the anchor and comes back.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Whether the tooltip shows at time `now`.
    #[must_use]
    pub fn is_shown(&self, now: Duration) -> bool {
        self.shows_at().is_some_and(|at| now >= at)
    }

    /// When the tooltip shows or showed, while the pointer rests on the anchor and it has not
    /// been dismissed; a caller can schedule its next frame for then.
    #[must_use]
    pub fn shows_at(&self) -> Option<Duration> {
        if self.dismissed {
            return None;
        }
        self.since.map(|since| since.saturating_add(self.delay))
    }
}
