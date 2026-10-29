// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};

use crate::anchor::{AnchorSide, place};
use crate::overlay::FocusTrap;
use crate::render_cx::clip_to_coordinates;
use crate::{BorderType, Popup, RenderCx, Widget, WidgetState};

/// A titled, bordered panel that opens beside the cells it belongs to, its anchor, such as the
/// options behind a toolbar button.
///
/// The popover never covers its anchor. It opens on its [`side`](Self::side) (below by default)
/// and flips to the opposite side, then to the other two, when the side has no room inside the
/// area; along the side it starts aligned with the anchor and slides to stay inside. When no side
/// has room, or the area leaves no cell inside the border, it is not drawn. The panel looks like a [`Popup`] (theme tokens, the
/// [`WidgetState`], an optional title) but leaves the rest of the screen as it is: there is no
/// backdrop and focus is not trapped.
///
/// The content's size is [`size`](Self::size) when set, and otherwise what the content measures.
/// [`area`](Self::area) gives the cells the panel covers, for hit testing, and [`PopoverState`]
/// opens and closes it.
#[derive(Clone, Debug)]
pub struct Popover<W> {
    popup: Popup<W>,
    anchor: Rect,
    side: AnchorSide,
}

impl<W> Popover<W> {
    /// A popover showing `content` for the cells in `anchor`, opening below it.
    #[must_use]
    pub fn new(content: W, anchor: Rect) -> Self {
        Self {
            popup: Popup::new(content)
                .backdrop(0.0)
                .focus_trap(FocusTrap::disabled()),
            anchor,
            side: AnchorSide::Below,
        }
    }

    /// The side tried first.
    #[must_use]
    pub const fn side(mut self, side: AnchorSide) -> Self {
        self.side = side;
        self
    }

    /// Title on the top border, drawn as given.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.popup = self.popup.title(title);
        self
    }

    /// Content size inside the border.
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.popup = self.popup.size(size);
        self
    }

    /// Interaction state for the border and title colors.
    #[must_use]
    pub fn state(mut self, state: WidgetState) -> Self {
        self.popup = self.popup.state(state);
        self
    }

    /// Border line style.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.popup = self.popup.border_type(border_type);
        self
    }

    /// Panel style, in place of the theme's.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.popup = self.popup.style(style);
        self
    }

    /// Border style, in place of the theme's.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.popup = self.popup.border_style(style);
        self
    }

    /// Title style, in place of the theme's.
    #[must_use]
    pub fn title_style(mut self, style: Style) -> Self {
        self.popup = self.popup.title_style(style);
        self
    }

    /// The anchor the popover belongs to.
    #[must_use]
    pub const fn anchor(&self) -> Rect {
        self.anchor
    }
}

impl<W: Widget> Popover<W> {
    /// The cells the panel covers inside `area`, or `None` when no side of the anchor has room.
    #[must_use]
    pub fn area(&self, area: Rect) -> Option<Rect> {
        let area = clip_to_coordinates(area);
        let size = self.popup.outer(area);
        if size.width < 3 || size.height < 3 {
            return None;
        }
        place(self.anchor, size, area, self.side)
    }
}

impl<W: Widget> Widget for Popover<W> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if let Some(panel) = self.area(area) {
            self.popup.draw(clip_to_coordinates(area), panel, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

/// Where a click landed, as [`PopoverState::click`] saw it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopoverClick {
    /// On the anchor: the popover opened or closed.
    Anchor,
    /// Inside the open popover: it stays open and the click belongs to its content.
    Inside,
    /// Anywhere else: an open popover closed.
    Outside,
}

/// Whether a [`Popover`] is open, kept by the app between frames.
///
/// A click on the anchor opens or closes it, a click inside keeps it open, and a click anywhere
/// else closes it, as does [`close`](Self::close) for Escape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PopoverState {
    open: bool,
}

impl PopoverState {
    /// A closed popover.
    #[must_use]
    pub const fn new() -> Self {
        Self { open: false }
    }

    /// Whether the popover is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the popover.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Closes the popover.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Opens a closed popover and closes an open one.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Handles a click at `(column, row)`, given the `anchor` and, while it is drawn, the
    /// `panel` from [`Popover::area`].
    pub fn click(
        &mut self,
        column: u16,
        row: u16,
        anchor: Rect,
        panel: Option<Rect>,
    ) -> PopoverClick {
        if contains(anchor, column, row) {
            self.toggle();
            return PopoverClick::Anchor;
        }
        if self.open && panel.is_some_and(|panel| contains(panel, column, row)) {
            return PopoverClick::Inside;
        }
        self.close();
        PopoverClick::Outside
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column - area.x < area.width && row >= area.y && row - area.y < area.height
}
