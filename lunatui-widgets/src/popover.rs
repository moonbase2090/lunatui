// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};
use lunatui_runtime::{FocusManager, HitId, Layer, LayerId, LayerSlot, LayerStack};

use crate::anchor::{AnchorSide, place};
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
            popup: Popup::new(content).backdrop(0.0),
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

/// Whether a [`Popover`] is open, kept by the app between frames.
///
/// The popover is a non-modal layer in the app's [`LayerStack`]: [`toggle`](Self::toggle) opens
/// it from its anchor and closes it again, and the stack's [`handle`](LayerStack::handle)
/// closes it on Esc or a press outside it and gives focus back to the anchor. Call
/// [`sync`](Self::sync) after `handle` to learn that it closed, and [`place`](Self::place) each
/// frame with the panel from [`Popover::area`] so presses inside it reach its content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PopoverState {
    slot: LayerSlot,
}

impl PopoverState {
    /// A closed popover.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slot: LayerSlot::new(),
        }
    }

    /// Whether the popover is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.slot.is_open()
    }

    /// The popover's layer while it is open.
    #[must_use]
    pub const fn layer(&self) -> Option<LayerId> {
        self.slot.id()
    }

    /// Opens the popover above every other layer. A press on `anchor` reaches the anchor
    /// instead of closing the popover, and `opener`, the anchor's id, gets focus back when it
    /// closes.
    pub fn open(&mut self, layers: &mut LayerStack, anchor: Rect, opener: Option<HitId>) {
        let mut layer = Layer::popover(Rect::new(0, 0, 0, 0)).anchor(anchor);
        if let Some(opener) = opener {
            layer = layer.return_focus(opener);
        }
        self.slot.open(layers, layer);
    }

    /// Closes the popover and gives focus back to its opener.
    pub fn close(&mut self, layers: &mut LayerStack, focus: &mut FocusManager) {
        self.slot.close(layers, focus);
    }

    /// Opens a closed popover and closes an open one, as a press on its anchor does.
    pub fn toggle(
        &mut self,
        layers: &mut LayerStack,
        focus: &mut FocusManager,
        anchor: Rect,
        opener: Option<HitId>,
    ) {
        if self.slot.sync(layers) || !self.is_open() {
            self.open(layers, anchor, opener);
        } else {
            self.close(layers, focus);
        }
    }

    /// Whether the stack closed the popover since the last call, on Esc or an outside press.
    pub fn sync(&mut self, layers: &LayerStack) -> bool {
        self.slot.sync(layers)
    }

    /// Records the cells the panel covers this frame, from [`Popover::area`].
    pub fn place(&self, layers: &mut LayerStack, panel: Option<Rect>) {
        self.slot
            .place(layers, panel.unwrap_or(Rect::new(0, 0, 0, 0)));
    }
}
