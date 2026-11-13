// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;
use lunatui_term::{Event, KeyCode, KeyKind, Modifiers, MouseKind};

use crate::{FocusManager, HitId};

const Z_STEP: u16 = 100;

/// Identifies one layer in a [`LayerStack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(
    /// Counter value, unique within a [`LayerStack`].
    pub u32,
);

/// What a layer is: its area, whether it blocks the layers below, and how it is dismissed.
///
/// A modal layer, such as a dialog or command palette, traps focus and swallows pointer input
/// outside its area; Esc dismisses it and an outside press does not. A non-modal layer, such as a
/// popover or dropdown, lets focus and clicks reach the layers below, and both Esc and a press
/// outside it dismiss it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    area: Rect,
    anchor: Option<Rect>,
    modal: bool,
    escape: bool,
    outside: bool,
    return_focus: Option<HitId>,
}

impl Layer {
    /// A modal layer covering `area`, dismissed by Esc but not by an outside press.
    #[must_use]
    pub fn modal(area: Rect) -> Self {
        Self {
            area,
            anchor: None,
            modal: true,
            escape: true,
            outside: false,
            return_focus: None,
        }
    }

    /// A non-modal layer covering `area`, dismissed by Esc or an outside press.
    #[must_use]
    pub fn popover(area: Rect) -> Self {
        Self {
            modal: false,
            outside: true,
            ..Self::modal(area)
        }
    }

    /// Whether Esc and outside presses dismiss the layer. A layer that is not dismissable stays
    /// until the app closes it, as a toast does.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.escape = dismissable;
        self.outside = dismissable;
        self
    }

    /// Whether a press outside the layer dismisses it. On for a popover, off for a modal.
    #[must_use]
    pub fn outside_press(mut self, dismisses: bool) -> Self {
        self.outside = dismisses;
        self
    }

    /// The control that opened the layer, such as a dropdown's field. A press on it is delivered
    /// instead of dismissing the layer, so the control can close the layer itself rather than
    /// reopen it.
    #[must_use]
    pub fn anchor(mut self, anchor: Rect) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// The id that gets focus back when this layer is removed, usually the control that opened it.
    #[must_use]
    pub fn return_focus(mut self, id: HitId) -> Self {
        self.return_focus = Some(id);
        self
    }

    /// The screen area the layer covers.
    #[must_use]
    pub fn area(&self) -> Rect {
        self.area
    }

    /// Whether the layer traps focus and blocks pointer input below it.
    #[must_use]
    pub fn is_modal(&self) -> bool {
        self.modal
    }
}

/// Where a pointer press lands, as decided by [`LayerStack::route_press`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressRoute {
    /// Inside this layer, or on the base screen when no layer is open. Hit test as usual,
    /// with `z` from [`LayerStack::z`] so the topmost region wins.
    Deliver,
    /// Outside a layer that an outside press dismisses: remove it. The press is consumed when a
    /// modal layer lies under it and otherwise should also be delivered, so one click can close a
    /// popover and press a button.
    Dismiss {
        /// The layer to remove.
        layer: LayerId,
        /// Whether the press should also be delivered once the layer is gone.
        deliver: bool,
    },
    /// Outside a modal layer that an outside press does not dismiss: ignore the press.
    Block,
}

/// What [`LayerStack::handle`] did with an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerEvent {
    /// The layers had nothing to do with it: handle the event as usual, hit testing a press with
    /// the layers' `z`.
    Pass,
    /// A press outside a modal layer: drop the event.
    Consumed,
    /// Esc or a press dismissed `layer`, which is gone and has given focus back to its opener.
    /// Handle the event as usual only when `deliver` is set.
    Dismissed {
        /// The layer that closed.
        layer: LayerId,
        /// Whether the event should also be handled as usual.
        deliver: bool,
    },
}

/// Open overlays in z order, with modal focus trapping and dismissal routing.
///
/// Layers stack bottom to top in the order they are pushed. [`z`](Self::z) gives each layer the
/// `z` to register its regions with in [`HitMap::push`](crate::HitMap::push), so the topmost layer
/// wins overlaps. [`focus_entries`](Self::focus_entries) filters the entries given to
/// [`FocusManager::sync`](crate::FocusManager::sync): while a modal layer is open, Tab only
/// visits it and the layers above it.
///
/// # Example
///
/// ```
/// use lunatui_core::Rect;
/// use lunatui_runtime::{HitId, Layer, LayerStack, PressRoute};
///
/// let mut layers = LayerStack::new();
/// let dialog = layers.push(Layer::modal(Rect::new(10, 5, 20, 8)).return_focus(HitId(1)));
/// layers.assign(dialog, HitId(7), Rect::new(12, 10, 6, 1));
///
/// assert_eq!(layers.route_press(0, 0), PressRoute::Block);
/// assert_eq!(layers.route_press(12, 10), PressRoute::Deliver);
/// assert_eq!(layers.remove(dialog), Some(HitId(1)));
/// ```
///
/// [`handle`](Self::handle) is the one place that turns Esc and pointer presses into dismissals.
/// Overlay state such as a dialog's or a popover's keeps a [`LayerSlot`] and learns that its layer
/// closed from [`LayerSlot::sync`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayerStack {
    layers: Vec<Entry>,
    next: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    id: LayerId,
    layer: Layer,
    focusable: Vec<(HitId, Rect)>,
}

impl LayerStack {
    /// An empty stack: only the base screen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens `layer` above every other layer.
    pub fn push(&mut self, layer: Layer) -> LayerId {
        let id = LayerId(self.next);
        self.next = self.next.wrapping_add(1);
        self.layers.push(Entry {
            id,
            layer,
            focusable: Vec::new(),
        });
        id
    }

    /// Closes `id` and returns the id to refocus, if the layer asked for one. `None` when the
    /// layer is unknown or did not ask. Layers above it stay open.
    pub fn remove(&mut self, id: LayerId) -> Option<HitId> {
        let at = self.layers.iter().position(|entry| entry.id == id)?;
        self.layers.remove(at).layer.return_focus
    }

    /// Closes `id` and gives focus back to the control it names in
    /// [`Layer::return_focus`]. Returns whether the layer was open.
    pub fn close(&mut self, id: LayerId, focus: &mut FocusManager) -> bool {
        let Some(at) = self.layers.iter().position(|entry| entry.id == id) else {
            return false;
        };
        if let Some(opener) = self.layers.remove(at).layer.return_focus {
            focus.restore(opener);
        }
        true
    }

    /// Applies `event` to the open layers: Esc closes the layer [`escape`](Self::escape) picks,
    /// and a pointer press goes through [`route_press`](Self::route_press). A closed layer gives
    /// focus back to its opener. Other events pass.
    pub fn handle(&mut self, event: &Event, focus: &mut FocusManager) -> LayerEvent {
        match event {
            Event::Key(key)
                if key.code == KeyCode::Esc
                    && key.kind == KeyKind::Press
                    && key.modifiers == Modifiers::new() =>
            {
                let Some(layer) = self.escape() else {
                    return LayerEvent::Pass;
                };
                self.close(layer, focus);
                LayerEvent::Dismissed {
                    layer,
                    deliver: false,
                }
            }
            Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::Down(_)) => {
                match self.route_press(mouse.column, mouse.row) {
                    PressRoute::Deliver => LayerEvent::Pass,
                    PressRoute::Block => LayerEvent::Consumed,
                    PressRoute::Dismiss { layer, deliver } => {
                        self.close(layer, focus);
                        LayerEvent::Dismissed { layer, deliver }
                    }
                }
            }
            _ => LayerEvent::Pass,
        }
    }

    /// Closes the topmost layer. See [`remove`](Self::remove).
    pub fn pop(&mut self) -> Option<HitId> {
        let top = self.layers.last()?.id;
        self.remove(top)
    }

    /// Moves the layer's area, for a popover that follows its anchor. Does nothing for an
    /// unknown layer.
    pub fn set_area(&mut self, id: LayerId, area: Rect) {
        if let Some(entry) = self.entries_mut(id) {
            entry.layer.area = area;
        }
    }

    /// Records a focusable region of `id` in Tab order. Call it each frame after
    /// [`clear_focusable`](Self::clear_focusable).
    pub fn assign(&mut self, id: LayerId, hit: HitId, area: Rect) {
        if let Some(entry) = self.entries_mut(id) {
            entry.focusable.push((hit, area));
        }
    }

    /// Forgets the focusable regions of every layer, ready for the next frame.
    pub fn clear_focusable(&mut self) {
        for entry in &mut self.layers {
            entry.focusable.clear();
        }
    }

    /// The layer with `id`, if it is open.
    #[must_use]
    pub fn get(&self, id: LayerId) -> Option<&Layer> {
        self.layers
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| &entry.layer)
    }

    /// The topmost layer.
    #[must_use]
    pub fn top(&self) -> Option<LayerId> {
        self.layers.last().map(|entry| entry.id)
    }

    /// Number of open layers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Whether only the base screen is showing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Whether any open layer is modal.
    #[must_use]
    pub fn is_modal(&self) -> bool {
        self.layers.iter().any(|entry| entry.layer.modal)
    }

    /// The `z` for regions of `id` in [`HitMap::push`](crate::HitMap::push). The base screen is
    /// `0` and each layer is above the one pushed before it. `None` for an unknown layer.
    #[must_use]
    pub fn z(&self, id: LayerId) -> Option<u16> {
        let at = self.layers.iter().position(|entry| entry.id == id)?;
        let rank = u16::try_from(at + 1).unwrap_or(u16::MAX);
        Some(rank.saturating_mul(Z_STEP))
    }

    /// Filters the base screen's Tab entries and appends the layers' own, in stack order.
    ///
    /// With no modal layer, the base entries stay and every layer's entries follow. With a
    /// modal layer, the base entries and every layer under the topmost modal are dropped.
    #[must_use]
    pub fn focus_entries(&self, base: Vec<(HitId, Rect)>) -> Vec<(HitId, Rect)> {
        let first = self
            .layers
            .iter()
            .rposition(|entry| entry.layer.modal)
            .unwrap_or(0);
        let mut entries = if self.is_modal() { Vec::new() } else { base };
        for entry in &self.layers[first..] {
            entries.extend(entry.focusable.iter().copied());
        }
        entries
    }

    /// Decides what a pointer press at `(column, row)` does. A press inside the topmost layer or
    /// on its anchor is delivered. Outside it, a topmost layer that outside presses dismiss is
    /// dismissed. While any modal layer is open nothing below the top is reachable: the press is
    /// delivered only when it falls inside the topmost modal, and otherwise it is consumed or
    /// blocked.
    #[must_use]
    pub fn route_press(&self, column: u16, row: u16) -> PressRoute {
        let Some(top) = self.layers.last() else {
            return PressRoute::Deliver;
        };
        let on_anchor = top
            .layer
            .anchor
            .is_some_and(|anchor| contains(anchor, column, row));
        if on_anchor || contains(top.layer.area, column, row) {
            return PressRoute::Deliver;
        }
        let in_modal = self
            .layers
            .iter()
            .rfind(|entry| entry.layer.modal)
            .map(|entry| contains(entry.layer.area, column, row));
        let reachable = in_modal.unwrap_or(true);
        if top.layer.outside {
            return PressRoute::Dismiss {
                layer: top.id,
                deliver: reachable,
            };
        }
        if reachable {
            PressRoute::Deliver
        } else {
            PressRoute::Block
        }
    }

    /// The layer an Esc press dismisses: the topmost layer Esc dismisses, unless a modal layer
    /// that Esc does not dismiss sits above it. `None` leaves Esc to the app.
    #[must_use]
    pub fn escape(&self) -> Option<LayerId> {
        for entry in self.layers.iter().rev() {
            if entry.layer.escape {
                return Some(entry.id);
            }
            if entry.layer.modal {
                return None;
            }
        }
        None
    }

    fn entries_mut(&mut self, id: LayerId) -> Option<&mut Entry> {
        self.layers.iter_mut().find(|entry| entry.id == id)
    }
}

/// One overlay's place in a [`LayerStack`]: open while its layer is on the stack.
///
/// A dialog, popover, dropdown, command palette or toast stack keeps one. [`open`](Self::open)
/// pushes the layer and [`close`](Self::close) removes it. When the stack closes the layer
/// itself, on Esc or an outside press, [`sync`](Self::sync) reports it once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerSlot {
    id: Option<LayerId>,
}

impl LayerSlot {
    /// A closed slot.
    #[must_use]
    pub const fn new() -> Self {
        Self { id: None }
    }

    /// Pushes `layer` above every other layer, unless the slot's layer is already open.
    pub fn open(&mut self, layers: &mut LayerStack, layer: Layer) -> LayerId {
        if let Some(id) = self.id.filter(|id| layers.get(*id).is_some()) {
            return id;
        }
        let id = layers.push(layer);
        self.id = Some(id);
        id
    }

    /// Closes the slot's layer and gives focus back to its opener. Returns whether it was open.
    pub fn close(&mut self, layers: &mut LayerStack, focus: &mut FocusManager) -> bool {
        self.id.take().is_some_and(|id| layers.close(id, focus))
    }

    /// Notices a layer that the stack closed. Returns `true` once, on the first call after the
    /// layer left the stack.
    pub fn sync(&mut self, layers: &LayerStack) -> bool {
        let gone = self.id.is_some_and(|id| layers.get(id).is_none());
        if gone {
            self.id = None;
        }
        gone
    }

    /// Moves the open layer to `area`, the cells the overlay drew this frame.
    pub fn place(&self, layers: &mut LayerStack, area: Rect) {
        if let Some(id) = self.id {
            layers.set_area(id, area);
        }
    }

    /// Whether the slot's layer is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.id.is_some()
    }

    /// The open layer.
    #[must_use]
    pub const fn id(&self) -> Option<LayerId> {
        self.id
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    let right = area.x.saturating_add(area.width);
    let bottom = area.y.saturating_add(area.height);
    (area.x..right).contains(&column) && (area.y..bottom).contains(&row)
}

#[cfg(test)]
mod tests {
    use lunatui_core::Rect;
    use lunatui_term::{
        Event, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
    };

    use super::{Layer, LayerEvent, LayerSlot, LayerStack, PressRoute};
    use crate::{FocusManager, HitId, HitMap};

    const AREA: Rect = Rect::new(10, 5, 20, 8);
    const OPENER: HitId = HitId(1);
    const OTHER: HitId = HitId(2);

    fn esc() -> Event {
        Event::Key(KeyEvent::press(KeyCode::Esc))
    }

    fn press(column: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent::new(
            MouseKind::Down(MouseButton::Left),
            column,
            row,
        ))
    }

    fn base_focus() -> FocusManager {
        let mut focus = FocusManager::new();
        focus.sync(vec![
            (OPENER, Rect::new(0, 0, 4, 1)),
            (OTHER, Rect::new(5, 0, 4, 1)),
        ]);
        focus
    }

    #[test]
    fn esc_closes_only_the_top_layer_and_then_the_next() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let dialog = layers.push(Layer::modal(AREA));
        let menu = layers.push(Layer::popover(Rect::new(12, 6, 4, 2)));
        assert_eq!(
            layers.handle(&esc(), &mut focus),
            LayerEvent::Dismissed {
                layer: menu,
                deliver: false
            }
        );
        assert_eq!(layers.top(), Some(dialog));
        assert_eq!(
            layers.handle(&esc(), &mut focus),
            LayerEvent::Dismissed {
                layer: dialog,
                deliver: false
            }
        );
        assert!(layers.is_empty());
        assert_eq!(layers.handle(&esc(), &mut focus), LayerEvent::Pass);
    }

    #[test]
    fn esc_release_and_modified_esc_do_not_dismiss() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        layers.push(Layer::popover(AREA));
        let release = KeyEvent::press(KeyCode::Esc).kind(KeyKind::Release);
        let shifted = KeyEvent::press(KeyCode::Esc).modifiers(Modifiers::new().shift());
        assert_eq!(
            layers.handle(&Event::Key(release), &mut focus),
            LayerEvent::Pass
        );
        assert_eq!(
            layers.handle(&Event::Key(shifted), &mut focus),
            LayerEvent::Pass
        );
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn outside_press_closes_a_popover_and_still_reaches_the_screen() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let menu = layers.push(Layer::popover(AREA));
        assert_eq!(layers.handle(&press(15, 7), &mut focus), LayerEvent::Pass);
        assert_eq!(
            layers.handle(&press(0, 0), &mut focus),
            LayerEvent::Dismissed {
                layer: menu,
                deliver: true
            }
        );
        assert!(layers.is_empty());
    }

    #[test]
    fn outside_press_leaves_a_modal_open_and_is_dropped() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let dialog = layers.push(Layer::modal(AREA));
        assert_eq!(
            layers.handle(&press(0, 0), &mut focus),
            LayerEvent::Consumed
        );
        assert_eq!(layers.top(), Some(dialog));
        assert_eq!(layers.handle(&press(12, 6), &mut focus), LayerEvent::Pass);
    }

    #[test]
    fn closing_a_layer_returns_focus_to_its_opener_on_the_next_frame() {
        let base = vec![
            (OPENER, Rect::new(0, 0, 4, 1)),
            (OTHER, Rect::new(5, 0, 4, 1)),
        ];
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let dialog = layers.push(Layer::modal(AREA).return_focus(OTHER));
        layers.assign(dialog, HitId(7), Rect::new(12, 10, 6, 1));
        focus.sync(layers.focus_entries(base.clone()));
        assert_eq!(focus.focused(), Some(HitId(7)));
        layers.handle(&esc(), &mut focus);
        focus.sync(layers.focus_entries(base));
        assert_eq!(focus.focused(), Some(OTHER));
    }

    #[test]
    fn press_on_the_anchor_reaches_the_opener_instead_of_dismissing() {
        let field = Rect::new(10, 4, 12, 1);
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        layers.push(Layer::popover(AREA).anchor(field));
        assert_eq!(layers.handle(&press(11, 4), &mut focus), LayerEvent::Pass);
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn slot_follows_its_layer_and_reports_a_stack_close_once() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let mut slot = LayerSlot::new();
        let id = slot.open(&mut layers, Layer::popover(AREA).return_focus(OTHER));
        assert_eq!(slot.open(&mut layers, Layer::popover(AREA)), id);
        assert_eq!(layers.len(), 1);
        assert!(!slot.sync(&layers));
        layers.handle(&press(0, 0), &mut focus);
        assert!(slot.sync(&layers));
        assert!(!slot.sync(&layers));
        assert!(!slot.is_open());
        assert_eq!(focus.focused(), Some(OTHER));
    }

    #[test]
    fn slot_close_removes_the_layer_and_restores_focus() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let mut slot = LayerSlot::new();
        slot.open(&mut layers, Layer::modal(AREA).return_focus(OTHER));
        slot.place(&mut layers, Rect::new(0, 0, 2, 2));
        assert_eq!(layers.route_press(1, 1), PressRoute::Deliver);
        assert!(slot.close(&mut layers, &mut focus));
        assert!(!slot.close(&mut layers, &mut focus));
        assert!(layers.is_empty());
        assert_eq!(focus.focused(), Some(OTHER));
    }

    #[test]
    fn a_toast_layer_ignores_esc_and_outside_presses() {
        let mut layers = LayerStack::new();
        let mut focus = base_focus();
        let dialog = layers.push(Layer::modal(AREA));
        layers.push(Layer::popover(Rect::new(40, 0, 10, 2)).dismissable(false));
        assert_eq!(
            layers.handle(&esc(), &mut focus),
            LayerEvent::Dismissed {
                layer: dialog,
                deliver: false
            }
        );
        assert_eq!(layers.handle(&press(0, 0), &mut focus), LayerEvent::Pass);
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn empty_stack_delivers_presses_and_has_no_escape_target() {
        let layers = LayerStack::new();
        assert!(layers.is_empty());
        assert_eq!(layers.route_press(0, 0), PressRoute::Deliver);
        assert_eq!(layers.escape(), None);
    }

    #[test]
    fn later_layers_get_higher_z_and_win_overlaps() {
        let mut layers = LayerStack::new();
        let below = layers.push(Layer::popover(AREA));
        let above = layers.push(Layer::popover(AREA));
        let (z_below, z_above) = (layers.z(below), layers.z(above));
        assert!(z_below < z_above);
        let mut map = HitMap::new();
        map.push(HitId(1), AREA, z_above.unwrap_or(0));
        map.push(HitId(2), AREA, z_below.unwrap_or(0));
        assert_eq!(map.pick(12, 6), Some(HitId(1)));
    }

    #[test]
    fn z_of_unknown_layer_is_none() {
        let mut layers = LayerStack::new();
        let id = layers.push(Layer::modal(AREA));
        layers.remove(id);
        assert_eq!(layers.z(id), None);
    }

    #[test]
    fn modal_trap_keeps_tab_inside_the_topmost_modal() {
        let base = vec![(HitId(1), Rect::new(0, 0, 4, 1))];
        let mut layers = LayerStack::new();
        let lower = layers.push(Layer::modal(AREA));
        layers.assign(lower, HitId(2), Rect::new(11, 6, 4, 1));
        let upper = layers.push(Layer::modal(AREA));
        layers.assign(upper, HitId(3), Rect::new(12, 7, 4, 1));
        let ids: Vec<HitId> = layers
            .focus_entries(base)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, vec![HitId(3)]);
    }

    #[test]
    fn popover_over_a_modal_adds_to_the_trap() {
        let mut layers = LayerStack::new();
        let modal = layers.push(Layer::modal(AREA));
        layers.assign(modal, HitId(2), Rect::new(11, 6, 4, 1));
        let menu = layers.push(Layer::popover(Rect::new(11, 7, 6, 3)));
        layers.assign(menu, HitId(3), Rect::new(11, 7, 6, 1));
        let ids: Vec<HitId> = layers
            .focus_entries(vec![(HitId(1), Rect::new(0, 0, 4, 1))])
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, vec![HitId(2), HitId(3)]);
    }

    #[test]
    fn popover_alone_leaves_the_base_focusable() {
        let mut layers = LayerStack::new();
        let menu = layers.push(Layer::popover(AREA));
        layers.assign(menu, HitId(2), Rect::new(11, 6, 4, 1));
        let ids: Vec<HitId> = layers
            .focus_entries(vec![(HitId(1), Rect::new(0, 0, 4, 1))])
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, vec![HitId(1), HitId(2)]);
    }

    #[test]
    fn clear_focusable_drops_last_frames_entries() {
        let mut layers = LayerStack::new();
        let id = layers.push(Layer::modal(AREA));
        layers.assign(id, HitId(2), AREA);
        layers.clear_focusable();
        assert_eq!(layers.focus_entries(Vec::new()).len(), 0);
    }

    #[test]
    fn press_outside_a_modal_is_blocked_unless_outside_presses_dismiss_it() {
        let mut layers = LayerStack::new();
        layers.push(Layer::modal(AREA));
        assert_eq!(layers.route_press(0, 0), PressRoute::Block);
        assert_eq!(layers.route_press(12, 6), PressRoute::Deliver);
        let palette = layers.push(Layer::modal(AREA).outside_press(true));
        assert_eq!(
            layers.route_press(0, 0),
            PressRoute::Dismiss {
                layer: palette,
                deliver: false
            }
        );
    }

    #[test]
    fn press_outside_a_popover_dismisses_and_passes_through() {
        let mut layers = LayerStack::new();
        let id = layers.push(Layer::popover(AREA));
        assert_eq!(
            layers.route_press(0, 0),
            PressRoute::Dismiss {
                layer: id,
                deliver: true
            }
        );
    }

    #[test]
    fn undismissable_modal_blocks_outside_presses() {
        let mut layers = LayerStack::new();
        layers.push(Layer::modal(AREA).dismissable(false));
        assert_eq!(layers.route_press(0, 0), PressRoute::Block);
        assert_eq!(layers.escape(), None);
    }

    #[test]
    fn undismissable_popover_over_a_modal_blocks_presses_outside_the_modal() {
        let mut layers = LayerStack::new();
        layers.push(Layer::modal(AREA));
        layers.push(Layer::popover(Rect::new(12, 6, 4, 2)).dismissable(false));
        assert_eq!(layers.route_press(0, 0), PressRoute::Block);
        assert_eq!(layers.route_press(25, 11), PressRoute::Deliver);
    }

    #[test]
    fn dismissable_popover_over_a_modal_closes_without_leaking_the_press() {
        let mut layers = LayerStack::new();
        layers.push(Layer::modal(AREA));
        let menu = layers.push(Layer::popover(Rect::new(12, 6, 4, 2)));
        assert_eq!(
            layers.route_press(0, 0),
            PressRoute::Dismiss {
                layer: menu,
                deliver: false
            }
        );
        assert_eq!(
            layers.route_press(25, 11),
            PressRoute::Dismiss {
                layer: menu,
                deliver: true
            }
        );
    }

    #[test]
    fn undismissable_popover_ignores_outside_presses() {
        let mut layers = LayerStack::new();
        layers.push(Layer::popover(AREA).dismissable(false));
        assert_eq!(layers.route_press(0, 0), PressRoute::Deliver);
    }

    #[test]
    fn escape_dismisses_the_topmost_dismissable_layer() {
        let mut layers = LayerStack::new();
        let lower = layers.push(Layer::modal(AREA));
        let upper = layers.push(Layer::popover(AREA));
        assert_eq!(layers.escape(), Some(upper));
        layers.remove(upper);
        assert_eq!(layers.escape(), Some(lower));
    }

    #[test]
    fn escape_stops_at_an_undismissable_modal() {
        let mut layers = LayerStack::new();
        layers.push(Layer::modal(AREA));
        layers.push(Layer::modal(AREA).dismissable(false));
        assert_eq!(layers.escape(), None);
    }

    #[test]
    fn escape_passes_over_an_undismissable_popover() {
        let mut layers = LayerStack::new();
        let modal = layers.push(Layer::modal(AREA));
        layers.push(Layer::popover(AREA).dismissable(false));
        assert_eq!(layers.escape(), Some(modal));
    }

    #[test]
    fn remove_returns_the_focus_to_restore() {
        let mut layers = LayerStack::new();
        let id = layers.push(Layer::modal(AREA).return_focus(HitId(9)));
        assert_eq!(layers.remove(id), Some(HitId(9)));
        assert_eq!(layers.remove(id), None);
    }

    #[test]
    fn pop_closes_the_top_layer_only() {
        let mut layers = LayerStack::new();
        let lower = layers.push(Layer::modal(AREA).return_focus(HitId(1)));
        layers.push(Layer::popover(AREA).return_focus(HitId(2)));
        assert_eq!(layers.pop(), Some(HitId(2)));
        assert_eq!(layers.top(), Some(lower));
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn set_area_moves_the_press_target() {
        let mut layers = LayerStack::new();
        let id = layers.push(Layer::popover(AREA));
        layers.set_area(id, Rect::new(0, 0, 3, 3));
        assert_eq!(layers.route_press(1, 1), PressRoute::Deliver);
        assert_eq!(layers.get(id).map(Layer::area), Some(Rect::new(0, 0, 3, 3)));
    }

    #[test]
    fn is_modal_reports_any_modal_layer() {
        let mut layers = LayerStack::new();
        layers.push(Layer::popover(AREA));
        assert!(!layers.is_modal());
        layers.push(Layer::modal(AREA));
        assert!(layers.is_modal());
    }
}
