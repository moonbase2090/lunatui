// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;

use crate::HitId;

/// Spacing between the `z` values of adjacent layers, so an app can slot its own regions between.
const Z_STEP: u16 = 100;

/// Identifies one layer in a [`LayerStack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(
    /// Counter value, unique within a [`LayerStack`].
    pub u32,
);

/// What a layer is: its area, whether it blocks the layers below, and how it is dismissed.
///
/// A modal layer traps focus and swallows pointer input outside its area. A non-modal layer, such
/// as a popover or dropdown, lets focus and clicks reach the layers below, but a click outside it
/// still dismisses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    area: Rect,
    modal: bool,
    dismissable: bool,
    return_focus: Option<HitId>,
}

impl Layer {
    /// A modal layer covering `area`, dismissed by Esc or an outside click.
    #[must_use]
    pub fn modal(area: Rect) -> Self {
        Self {
            area,
            modal: true,
            dismissable: true,
            return_focus: None,
        }
    }

    /// A non-modal layer covering `area`, dismissed by Esc or an outside click.
    #[must_use]
    pub fn popover(area: Rect) -> Self {
        Self {
            modal: false,
            ..Self::modal(area)
        }
    }

    /// Whether Esc and outside clicks dismiss the layer. On by default. A layer that is not
    /// dismissable stays until the app removes it.
    #[must_use]
    pub fn dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
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
    /// Outside a dismissable layer: remove it. The press is consumed when the layer is modal and
    /// otherwise should also be delivered, so one click can close a popover and press a button.
    Dismiss {
        /// The layer to remove.
        layer: LayerId,
        /// Whether the press should also be delivered once the layer is gone.
        deliver: bool,
    },
    /// Outside a modal layer that cannot be dismissed: ignore the press.
    Block,
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
/// assert_eq!(layers.route_press(0, 0), PressRoute::Dismiss { layer: dialog, deliver: false });
/// assert_eq!(layers.route_press(12, 10), PressRoute::Deliver);
/// assert_eq!(layers.remove(dialog), Some(HitId(1)));
/// ```
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

    /// Decides what a pointer press at `(column, row)` does. Only the topmost layer is
    /// asked: a press outside it dismisses it, and a modal layer stops that press from reaching
    /// anything below.
    #[must_use]
    pub fn route_press(&self, column: u16, row: u16) -> PressRoute {
        let Some(top) = self.layers.last() else {
            return PressRoute::Deliver;
        };
        if contains(top.layer.area, column, row) {
            return PressRoute::Deliver;
        }
        if top.layer.dismissable {
            return PressRoute::Dismiss {
                layer: top.id,
                deliver: !top.layer.modal,
            };
        }
        if top.layer.modal {
            PressRoute::Block
        } else {
            PressRoute::Deliver
        }
    }

    /// The layer an Esc press dismisses: the topmost dismissable layer, unless a modal layer
    /// that cannot be dismissed sits above it. `None` leaves Esc to the app.
    #[must_use]
    pub fn escape(&self) -> Option<LayerId> {
        for entry in self.layers.iter().rev() {
            if entry.layer.dismissable {
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

fn contains(area: Rect, column: u16, row: u16) -> bool {
    let right = area.x.saturating_add(area.width);
    let bottom = area.y.saturating_add(area.height);
    (area.x..right).contains(&column) && (area.y..bottom).contains(&row)
}

#[cfg(test)]
mod tests {
    use lunatui_core::Rect;

    use super::{Layer, LayerStack, PressRoute};
    use crate::{HitId, HitMap};

    const AREA: Rect = Rect::new(10, 5, 20, 8);

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
    fn press_outside_a_modal_dismisses_and_is_consumed() {
        let mut layers = LayerStack::new();
        let id = layers.push(Layer::modal(AREA));
        assert_eq!(
            layers.route_press(0, 0),
            PressRoute::Dismiss {
                layer: id,
                deliver: false
            }
        );
        assert_eq!(layers.route_press(12, 6), PressRoute::Deliver);
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
