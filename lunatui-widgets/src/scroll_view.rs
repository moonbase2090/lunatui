// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Grid, Rect, Size, Style};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Scrollbar, Widget, WidgetState};

/// When a [`ScrollView`] draws a scrollbar.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarVisibility {
    /// Only when the content does not fit on that axis.
    #[default]
    Automatic,
    /// Always, even when the content fits.
    Always,
    /// Never; the content can still scroll.
    Never,
}

/// Scroll position of a [`ScrollView`], kept by the app between frames.
///
/// [`ScrollView::sync`] records the content and page sizes, so paging and
/// [`scroll_to_bottom`](Self::scroll_to_bottom) know how far to go, and keeps the offset in range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollViewState {
    x: u16,
    y: u16,
    content: Option<Size>,
    page: Option<Size>,
}

impl ScrollViewState {
    /// Scrolled to the top left, with no sizes recorded yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            content: None,
            page: None,
        }
    }

    /// Scrolled so that content column `x` and row `y` are at the top left.
    #[must_use]
    pub const fn with_offset(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            content: None,
            page: None,
        }
    }

    /// The first visible content column and row.
    #[must_use]
    pub const fn offset(&self) -> (u16, u16) {
        (self.x, self.y)
    }

    /// Moves the top left to content column `x` and row `y`, kept in range once sizes are known.
    pub fn set_offset(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
        self.clamp();
    }

    /// The content size recorded by the last [`ScrollView::sync`].
    #[must_use]
    pub const fn content_size(&self) -> Option<Size> {
        self.content
    }

    /// The visible size recorded by the last [`ScrollView::sync`].
    #[must_use]
    pub const fn page_size(&self) -> Option<Size> {
        self.page
    }

    /// Scrolls up by `rows`.
    pub fn scroll_up(&mut self, rows: u16) {
        self.y = self.y.saturating_sub(rows);
    }

    /// Scrolls down by `rows`.
    pub fn scroll_down(&mut self, rows: u16) {
        self.y = self.y.saturating_add(rows);
        self.clamp();
    }

    /// Scrolls left by `columns`.
    pub fn scroll_left(&mut self, columns: u16) {
        self.x = self.x.saturating_sub(columns);
    }

    /// Scrolls right by `columns`.
    pub fn scroll_right(&mut self, columns: u16) {
        self.x = self.x.saturating_add(columns);
        self.clamp();
    }

    /// Scrolls down by one page, keeping the last visible row on screen.
    pub fn page_down(&mut self) {
        self.scroll_down(self.page_step());
    }

    /// Scrolls up by one page, keeping the first visible row on screen.
    pub fn page_up(&mut self) {
        self.scroll_up(self.page_step());
    }

    /// Scrolls to the top left.
    pub fn scroll_to_top(&mut self) {
        self.x = 0;
        self.y = 0;
    }

    /// Scrolls to the last page. Before the first sync it jumps as far as it can, and the next
    /// sync pulls it back to the last page.
    pub fn scroll_to_bottom(&mut self) {
        self.y = u16::MAX;
        self.clamp();
    }

    /// Whether the last content row is visible.
    #[must_use]
    pub fn is_at_bottom(&self) -> bool {
        match (self.content, self.page) {
            (Some(content), Some(page)) => self.y.saturating_add(page.height) >= content.height,
            _ => true,
        }
    }

    fn page_step(&self) -> u16 {
        self.page
            .map_or(1, |page| page.height.saturating_sub(1).max(1))
    }

    fn clamp(&mut self) {
        if let (Some(content), Some(page)) = (self.content, self.page) {
            self.x = self.x.min(content.width.saturating_sub(page.width));
            self.y = self.y.min(content.height.saturating_sub(page.height));
        }
    }
}

/// A viewport onto content larger than the screen, with optional scrollbars.
///
/// The content widget draws into an offscreen grid of `size` cells, and the visible part is
/// copied into the area. Cells the content leaves without a background show the background
/// under the view, and the part of the viewport past the content is cleared to blanks the same
/// way. Only the rows and columns up to the bottom right of the visible part are kept, so a tall
/// document costs memory for what has been scrolled past, not for all of it.
///
/// Scrollbars take the right column and the bottom row. Their colors come from theme tokens and
/// the [`WidgetState`]: the track is `surface.1`, and the thumb is `accent`, `focus.ring` when
/// focused, or dimmed text when disabled. They keep the background under the view.
#[derive(Clone, Debug)]
pub struct ScrollView<W> {
    content: W,
    size: Size,
    x: u16,
    y: u16,
    vertical: ScrollbarVisibility,
    horizontal: ScrollbarVisibility,
    state: WidgetState,
    track_style: Option<Style>,
    thumb_style: Option<Style>,
}

impl<W> ScrollView<W> {
    /// A view onto `content`, which is drawn at `size`.
    #[must_use]
    pub fn new(content: W, size: Size) -> Self {
        Self {
            content,
            size,
            x: 0,
            y: 0,
            vertical: ScrollbarVisibility::Automatic,
            horizontal: ScrollbarVisibility::Automatic,
            state: WidgetState::Normal,
            track_style: None,
            thumb_style: None,
        }
    }

    /// Scrolls to the offset in `scroll`.
    #[must_use]
    pub fn scroll(mut self, scroll: &ScrollViewState) -> Self {
        (self.x, self.y) = scroll.offset();
        self
    }

    /// Scrolls so that content column `x` and row `y` are at the top left.
    #[must_use]
    pub fn offset(mut self, x: u16, y: u16) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// When to draw both scrollbars.
    #[must_use]
    pub fn scrollbars(mut self, visibility: ScrollbarVisibility) -> Self {
        self.vertical = visibility;
        self.horizontal = visibility;
        self
    }

    /// When to draw the vertical scrollbar.
    #[must_use]
    pub fn vertical_scrollbar(mut self, visibility: ScrollbarVisibility) -> Self {
        self.vertical = visibility;
        self
    }

    /// When to draw the horizontal scrollbar.
    #[must_use]
    pub fn horizontal_scrollbar(mut self, visibility: ScrollbarVisibility) -> Self {
        self.horizontal = visibility;
        self
    }

    /// Interaction state for the scrollbar colors.
    #[must_use]
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Track style, in place of the theme's.
    #[must_use]
    pub fn track_style(mut self, style: Style) -> Self {
        self.track_style = Some(style);
        self
    }

    /// Thumb style, in place of the theme's.
    #[must_use]
    pub fn thumb_style(mut self, style: Style) -> Self {
        self.thumb_style = Some(style);
        self
    }

    /// The content size.
    #[must_use]
    pub fn content_size(&self) -> Size {
        self.size
    }

    /// The part of `area` that shows content, after the scrollbars take their row and column.
    #[must_use]
    pub fn viewport(&self, area: Rect) -> Rect {
        let area = clip_to_coordinates(area);
        let (horizontal, vertical) = self.bars(area);
        Rect::new(
            area.x,
            area.y,
            area.width.saturating_sub(u16::from(vertical)),
            area.height.saturating_sub(u16::from(horizontal)),
        )
    }

    /// Records the content and page sizes for `area` in `scroll` and keeps its offset in range.
    pub fn sync(&self, area: Rect, scroll: &mut ScrollViewState) {
        let viewport = self.viewport(area);
        scroll.content = Some(self.size);
        scroll.page = Some(Size::new(viewport.width, viewport.height));
        scroll.clamp();
    }

    fn bars(&self, area: Rect) -> (bool, bool) {
        let wide = self.size.width > area.width;
        let tall = self.size.height > area.height;
        let mut vertical = match self.vertical {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Automatic => tall,
        };
        let horizontal = match self.horizontal {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Automatic => {
                wide || (vertical && self.size.width > area.width.saturating_sub(1))
            }
        };
        if self.vertical == ScrollbarVisibility::Automatic && horizontal && !vertical {
            vertical = self.size.height > area.height.saturating_sub(1);
        }
        (horizontal, vertical)
    }

    fn bar_styles(&self, cx: &RenderCx<'_>, at: (u16, u16)) -> (Style, Style) {
        let under = Style::new().bg(cx.bg_at(at.0, at.1));
        let track = under.fg(cx.token(Token::Surface1));
        let thumb = match self.state {
            WidgetState::Focused => under.fg(cx.token(Token::FocusRing)),
            WidgetState::Disabled => under
                .fg(cx.token(Token::TextPrimary))
                .with_attrs(Attrs::new().set_dim(true)),
            _ => under.fg(cx.token(Token::Accent)),
        };
        (
            self.track_style.unwrap_or(track),
            self.thumb_style.unwrap_or(thumb),
        )
    }
}

impl<W: Widget> Widget for ScrollView<W> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let (horizontal, vertical) = self.bars(area);
        let viewport = self.viewport(area);
        let x = self.x.min(self.size.width.saturating_sub(viewport.width));
        let y = self.y.min(self.size.height.saturating_sub(viewport.height));
        if viewport.width > 0 && viewport.height > 0 {
            let mut canvas = Grid::new(
                x.saturating_add(viewport.width),
                y.saturating_add(viewport.height),
            );
            let full = Rect::new(0, 0, self.size.width, self.size.height);
            cx.offscreen(&mut canvas, |inner| self.content.render(full, inner));
            let shown = Rect::new(x, y, viewport.width, viewport.height);
            cx.blit(&canvas, shown, area.x, area.y);
        }
        let (track, thumb) = self.bar_styles(
            cx,
            (area.x.saturating_add(area.width.saturating_sub(1)), area.y),
        );
        if vertical && viewport.height > 0 {
            Scrollbar::vertical(
                usize::from(self.size.height),
                usize::from(viewport.height),
                usize::from(y),
            )
            .track_style(track)
            .thumb_style(thumb)
            .render(
                Rect::new(
                    area.x.saturating_add(viewport.width),
                    area.y,
                    1,
                    viewport.height,
                ),
                cx,
            );
        }
        if horizontal && viewport.width > 0 {
            Scrollbar::horizontal(
                usize::from(self.size.width),
                usize::from(viewport.width),
                usize::from(x),
            )
            .track_style(track)
            .thumb_style(thumb)
            .render(
                Rect::new(
                    area.x,
                    area.y.saturating_add(viewport.height),
                    viewport.width,
                    1,
                ),
                cx,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        self.size.clamp(avail)
    }
}
