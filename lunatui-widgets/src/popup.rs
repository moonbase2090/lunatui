// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Rect, Size, Style};
use lunatui_theme::Token;

use crate::overlay::FocusTrap;
use crate::render_cx::clip_to_coordinates;
use crate::{Block, BorderType, Overlay, RenderCx, Widget, WidgetState};

/// Where a [`Popup`] opens inside its area.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    /// Centred, with an odd leftover cell after the popup (right or below).
    #[default]
    Center,
    /// Top left corner at column `x`, row `y`.
    At(u16, u16),
    /// Under the anchor, left edges aligned; above it when there is no room below.
    Below(Rect),
    /// Over the anchor, left edges aligned; below it when there is no room above.
    Above(Rect),
    /// Right of the anchor, top edges aligned; left of it when there is no room to the right.
    Right(Rect),
    /// Left of the anchor, top edges aligned; right of it when there is no room to the left.
    Left(Rect),
}

/// Where a dragged or moved [`Popup`] sits, kept by the app between frames.
///
/// A popup opens at its [`Placement`]. [`Popup::sync`] records where it is drawn. After
/// [`press`](Self::press) and [`drag`](Self::drag), or [`move_by`](Self::move_by), the popup
/// stays where it was put until [`reset`](Self::reset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PopupState {
    area: Option<Rect>,
    moved: Option<(u16, u16)>,
    grab: Option<(u16, u16)>,
}

impl PopupState {
    /// A popup at its placement, not being dragged.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            area: None,
            moved: None,
            grab: None,
        }
    }

    /// Where the popup was drawn at the last [`Popup::sync`].
    #[must_use]
    pub const fn area(&self) -> Option<Rect> {
        self.area
    }

    /// The top left corner the popup was moved to, if it was moved.
    #[must_use]
    pub const fn moved_to(&self) -> Option<(u16, u16)> {
        self.moved
    }

    /// Whether a drag is in progress.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        self.grab.is_some()
    }

    /// Starts a drag when `(column, row)` is on the popup. Returns whether it was.
    pub fn press(&mut self, column: u16, row: u16) -> bool {
        let Some(area) = self.area else {
            return false;
        };
        let inside = column >= area.x
            && column - area.x < area.width
            && row >= area.y
            && row - area.y < area.height;
        if inside {
            self.grab = Some((column - area.x, row - area.y));
        }
        inside
    }

    /// Moves a popup being dragged so the grabbed cell follows `(column, row)`.
    pub fn drag(&mut self, column: u16, row: u16) {
        if let Some((dx, dy)) = self.grab {
            self.place(column.saturating_sub(dx), row.saturating_sub(dy));
        }
    }

    /// Ends a drag.
    pub fn release(&mut self) {
        self.grab = None;
    }

    /// Moves the popup by `columns` and `rows`, which may be negative.
    pub fn move_by(&mut self, columns: i32, rows: i32) {
        if let Some(area) = self.area {
            let shift =
                |at: u16, by: i32| u16::try_from((i32::from(at) + by).max(0)).unwrap_or(u16::MAX);
            self.place(shift(area.x, columns), shift(area.y, rows));
        }
    }

    /// Moves the popup's top left corner to column `x`, row `y`.
    pub fn move_to(&mut self, x: u16, y: u16) {
        self.place(x, y);
    }

    /// Returns the popup to its placement and ends any drag.
    pub fn reset(&mut self) {
        self.moved = None;
        self.grab = None;
    }

    fn place(&mut self, x: u16, y: u16) {
        self.moved = Some((x, y));
        if let Some(area) = &mut self.area {
            area.x = x;
            area.y = y;
        }
    }
}

/// A titled, bordered panel drawn over the rest of the screen.
///
/// The popup is an [`Overlay`]: the area behind it fades toward `surface.0`, and the panel is
/// cleared and drawn on top, so nothing behind shows through. Colors come from theme tokens and
/// the [`WidgetState`]: the panel is `surface.1` with `text.primary`; the border and title are
/// `accent`, `focus.ring` when focused, and dimmed when disabled. Explicit styles win. Content
/// cells drawn without a foreground or background take the panel's.
///
/// The content's size is [`size`](Self::size) when set, and otherwise what the content measures
/// in the area inside the border. The popup is kept inside its area.
#[derive(Clone, Debug)]
pub struct Popup<W> {
    content: W,
    title: Option<String>,
    size: Option<Size>,
    placement: Placement,
    moved: Option<(u16, u16)>,
    state: WidgetState,
    border_type: BorderType,
    backdrop: f32,
    trap: FocusTrap,
    style: Option<Style>,
    border_style: Option<Style>,
    title_style: Option<Style>,
}

impl<W> Popup<W> {
    /// A popup showing `content`, centred, with a rounded border.
    #[must_use]
    pub fn new(content: W) -> Self {
        Self {
            content,
            title: None,
            size: None,
            placement: Placement::Center,
            moved: None,
            state: WidgetState::Normal,
            border_type: BorderType::Rounded,
            backdrop: 0.5,
            trap: FocusTrap::enabled(),
            style: None,
            border_style: None,
            title_style: None,
        }
    }

    /// Title on the top border, drawn as given.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Content size inside the border.
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Where the popup opens.
    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Draws the popup where `popup` says it was dragged or moved, if it was.
    #[must_use]
    pub fn position(mut self, popup: &PopupState) -> Self {
        self.moved = popup.moved;
        self
    }

    /// Interaction state for the border and title colors.
    #[must_use]
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Border line style.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    /// How far the area behind the popup fades, from `0.0` (not at all) to `1.0`.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32) -> Self {
        self.backdrop = alpha.clamp(0.0, 1.0);
        self
    }

    /// Focus confinement while the popup is open.
    #[must_use]
    pub fn focus_trap(mut self, trap: FocusTrap) -> Self {
        self.trap = trap;
        self
    }

    /// Whether focus should stay inside the popup.
    #[must_use]
    pub fn traps_focus(&self) -> bool {
        self.trap.is_active()
    }

    /// Panel style, in place of the theme's.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Border style, in place of the theme's.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = Some(style);
        self
    }

    /// Title style, in place of the theme's.
    #[must_use]
    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = Some(style);
        self
    }

    fn styles(&self, cx: &RenderCx<'_>) -> (Style, Style, Style) {
        let panel = self.style.unwrap_or_else(|| {
            Style::new()
                .fg(cx.token(Token::TextPrimary))
                .bg(cx.token(Token::Surface1))
        });
        let line = match self.state {
            WidgetState::Focused => panel.fg(cx.token(Token::FocusRing)),
            WidgetState::Disabled => panel.with_attrs(Attrs::new().set_dim(true)),
            _ => panel.fg(cx.token(Token::Accent)),
        };
        let title = line.with_attrs(match self.state {
            WidgetState::Disabled => Attrs::new().set_dim(true),
            _ => Attrs::new().set_bold(true),
        });
        (
            panel,
            self.border_style.unwrap_or(line),
            self.title_style.unwrap_or(title),
        )
    }
}

impl<W: Widget> Popup<W> {
    /// Where the popup is drawn inside `area`.
    #[must_use]
    pub fn area(&self, area: Rect) -> Rect {
        let area = clip_to_coordinates(area);
        let inner = Size::new(area.width.saturating_sub(2), area.height.saturating_sub(2));
        let wanted = self.size.unwrap_or_else(|| self.content.measure(inner));
        let size = Size::new(
            wanted.width.saturating_add(2),
            wanted.height.saturating_add(2),
        )
        .clamp(Size::new(area.width, area.height));
        let (x, y) = match self.moved {
            Some(at) => at,
            None => place(self.placement, area, size),
        };
        let max_x = area.x.saturating_add(area.width - size.width);
        let max_y = area.y.saturating_add(area.height - size.height);
        Rect::new(
            x.clamp(area.x, max_x),
            y.clamp(area.y, max_y),
            size.width,
            size.height,
        )
    }

    /// Records where the popup is drawn inside `area` in `popup`, for dragging and moving.
    pub fn sync(&self, area: Rect, popup: &mut PopupState) {
        let drawn = self.area(area);
        popup.area = Some(drawn);
        if popup.moved.is_some() {
            popup.moved = Some((drawn.x, drawn.y));
        }
    }
}

fn place(placement: Placement, area: Rect, size: Size) -> (u16, u16) {
    let right = area.x.saturating_add(area.width);
    let bottom = area.y.saturating_add(area.height);
    match placement {
        Placement::Center => (
            area.x
                .saturating_add(area.width.saturating_sub(size.width) / 2),
            area.y
                .saturating_add(area.height.saturating_sub(size.height) / 2),
        ),
        Placement::At(x, y) => (x, y),
        Placement::Below(anchor) | Placement::Above(anchor) => {
            let below = anchor.y.saturating_add(anchor.height);
            let above = anchor.y.checked_sub(size.height).filter(|y| *y >= area.y);
            let fits_below = below.saturating_add(size.height) <= bottom;
            let prefer_above = matches!(placement, Placement::Above(_));
            let y = match above {
                Some(y) if prefer_above || !fits_below => y,
                _ => below,
            };
            (anchor.x, y)
        }
        Placement::Right(anchor) | Placement::Left(anchor) => {
            let after = anchor.x.saturating_add(anchor.width);
            let before = anchor.x.checked_sub(size.width).filter(|x| *x >= area.x);
            let fits_after = after.saturating_add(size.width) <= right;
            let prefer_before = matches!(placement, Placement::Left(_));
            let x = match before {
                Some(x) if prefer_before || !fits_after => x,
                _ => after,
            };
            (x, anchor.y)
        }
    }
}

struct Panel<'p, W> {
    popup: &'p Popup<W>,
    styles: (Style, Style, Style),
}

impl<W: Widget> Widget for Panel<'_, W> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let (panel, border, title) = self.styles;
        cx.fill(area, panel, ' ');
        let mut block = Block::bordered()
            .border_type(self.popup.border_type)
            .style(panel)
            .border_style(border)
            .title_style(title);
        if let Some(text) = &self.popup.title {
            block = block.title(text.clone());
        }
        block.render(area, cx);
        let inner = block.inner(area);
        if inner.width > 0 && inner.height > 0 {
            self.popup.content.render(inner, cx);
            cx.inherit(inner, panel);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

impl<W: Widget> Widget for Popup<W> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let modal = self.area(area);
        let panel = Panel {
            popup: self,
            styles: self.styles(cx),
        };
        if self.backdrop > 0.0 {
            let toward: Color = cx.token(Token::Surface0);
            cx.fade_fg(area, self.backdrop, toward);
            Overlay::new(panel, modal)
                .backdrop(self.backdrop, toward)
                .focus_trap(self.trap)
                .render(area, cx);
        } else {
            panel.render(modal, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}
