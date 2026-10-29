// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Style};

use crate::RenderCx;

const FULL: u16 = 1000;
const GONE: PaneId = PaneId(u32::MAX);

/// Identifies one pane of a [`SplitTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(
    /// Counter value, unique within a [`SplitTree`].
    pub u32,
);

/// Identifies one divider of a [`SplitLayout`]. Numbers count the splits in tree order, so they
/// stay the same until the tree's shape changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DividerId(
    /// Position of the split in tree order.
    pub usize,
);

/// Which way a split lays its two panes out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitAxis {
    /// Side by side, with a vertical divider between them.
    Horizontal,
    /// One above the other, with a horizontal divider between them.
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Leaf(PaneId),
    Split(Box<Split>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Split {
    axis: SplitAxis,
    ratio: u16,
    min: [u16; 2],
    children: [Node; 2],
}

/// A tree of nested splits, each dividing its area between two panes or further splits, with one
/// optional pane zoomed to fill everything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitTree {
    root: Node,
    next: u32,
    zoomed: Option<PaneId>,
}

/// One pane's place in a [`SplitLayout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneArea {
    /// The pane.
    pub pane: PaneId,
    /// The cells it covers.
    pub rect: Rect,
}

/// One divider's place in a [`SplitLayout`], for drawing and hit testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divider {
    /// The divider.
    pub id: DividerId,
    /// The way the split it belongs to lays out its panes.
    pub axis: SplitAxis,
    /// The one-cell line it covers.
    pub rect: Rect,
    /// The whole area of the split it belongs to, both panes and the divider.
    pub bounds: Rect,
    /// Fewest cells the first pane may have.
    pub min_first: u16,
    /// Fewest cells the second pane may have.
    pub min_second: u16,
}

/// Where the panes and dividers of a [`SplitTree`] sit in an area.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SplitLayout {
    /// Every pane shown, in tree order. A zoomed tree shows only the zoomed pane.
    pub panes: Vec<PaneArea>,
    /// Every divider drawn, in tree order. A zoomed tree draws none.
    pub dividers: Vec<Divider>,
}

impl SplitLayout {
    /// The pane under `(column, row)`, if any.
    #[must_use]
    pub fn pane_at(&self, column: u16, row: u16) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|area| contains(area.rect, column, row))
            .map(|area| area.pane)
    }

    /// The divider under `(column, row)`, if any.
    #[must_use]
    pub fn divider_at(&self, column: u16, row: u16) -> Option<&Divider> {
        self.dividers
            .iter()
            .find(|divider| contains(divider.rect, column, row))
    }

    /// The cells a pane covers.
    #[must_use]
    pub fn rect_of(&self, pane: PaneId) -> Option<Rect> {
        self.panes
            .iter()
            .find(|area| area.pane == pane)
            .map(|area| area.rect)
    }

    /// Draws every divider as a line, in `style`, or in `active_style` for `active`.
    pub fn render_dividers(
        &self,
        style: Style,
        active: Option<DividerId>,
        active_style: Style,
        cx: &mut RenderCx<'_>,
    ) {
        for divider in &self.dividers {
            let (glyph, style) = match (divider.axis, active == Some(divider.id)) {
                (SplitAxis::Horizontal, false) => ("│", style),
                (SplitAxis::Horizontal, true) => ("┃", active_style),
                (SplitAxis::Vertical, false) => ("─", style),
                (SplitAxis::Vertical, true) => ("━", active_style),
            };
            for y in divider.rect.y..divider.rect.y + divider.rect.height {
                for x in divider.rect.x..divider.rect.x + divider.rect.width {
                    cx.put_str(x, y, glyph, style);
                }
            }
        }
    }
}

impl SplitTree {
    /// A tree with a single pane, and its id.
    #[must_use]
    pub fn new() -> (Self, PaneId) {
        let first = PaneId(0);
        (
            Self {
                root: Node::Leaf(first),
                next: 1,
                zoomed: None,
            },
            first,
        )
    }

    /// Splits `pane` in two along `axis`: `pane` stays first and the new pane goes second.
    /// `ratio` is the first pane's share in thousandths (clamped to 0..=1000). Returns the new
    /// pane, or `None` when `pane` is not in the tree.
    pub fn split(&mut self, pane: PaneId, axis: SplitAxis, ratio: u16) -> Option<PaneId> {
        let fresh = PaneId(self.next);
        if !split_leaf(&mut self.root, pane, axis, ratio.min(FULL), fresh) {
            return None;
        }
        self.next += 1;
        Some(fresh)
    }

    /// Removes `pane`; its sibling takes the whole space the split had. The last pane cannot be
    /// closed. Returns whether a pane was removed.
    pub fn close(&mut self, pane: PaneId) -> bool {
        if !close_leaf(&mut self.root, pane) {
            return false;
        }
        if self.zoomed == Some(pane) {
            self.zoomed = None;
        }
        true
    }

    /// Number of panes.
    #[must_use]
    pub fn len(&self) -> usize {
        count_leaves(&self.root)
    }

    /// Whether the tree holds no panes. A tree always holds at least one.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Whether `pane` is in the tree.
    #[must_use]
    pub fn contains(&self, pane: PaneId) -> bool {
        has_leaf(&self.root, pane)
    }

    /// The panes in tree order.
    #[must_use]
    pub fn panes(&self) -> Vec<PaneId> {
        let mut panes = Vec::new();
        collect_leaves(&self.root, &mut panes);
        panes
    }

    /// The zoomed pane, if any.
    #[must_use]
    pub fn zoomed(&self) -> Option<PaneId> {
        self.zoomed
    }

    /// Zooms `pane` to fill the whole area, or restores the layout with `None`. A pane that is not
    /// in the tree is ignored.
    pub fn zoom(&mut self, pane: Option<PaneId>) {
        match pane {
            Some(pane) if !self.contains(pane) => {}
            other => self.zoomed = other,
        }
    }

    /// Zooms `pane`, or restores the layout when it is already the zoomed one.
    pub fn toggle_zoom(&mut self, pane: PaneId) {
        if self.zoomed == Some(pane) {
            self.zoomed = None;
        } else {
            self.zoom(Some(pane));
        }
    }

    /// Sets the fewest cells the two panes of a split may have.
    pub fn set_min(&mut self, divider: DividerId, first: u16, second: u16) -> bool {
        match split_mut(&mut self.root, divider.0, &mut 0) {
            Some(split) => {
                split.min = [first, second];
                true
            }
            None => false,
        }
    }

    /// The first pane's share of a split in thousandths.
    #[must_use]
    pub fn ratio(&self, divider: DividerId) -> Option<u16> {
        split_ref(&self.root, divider.0, &mut 0).map(|split| split.ratio)
    }

    /// Sets the first pane's share of a split, in thousandths (clamped to 0..=1000). Returns
    /// whether the divider exists.
    pub fn set_ratio(&mut self, divider: DividerId, ratio: u16) -> bool {
        match split_mut(&mut self.root, divider.0, &mut 0) {
            Some(split) => {
                split.ratio = ratio.min(FULL);
                true
            }
            None => false,
        }
    }

    /// Where the panes and dividers sit when the tree fills `area`.
    #[must_use]
    pub fn layout(&self, area: Rect) -> SplitLayout {
        let mut layout = SplitLayout::default();
        if let Some(pane) = self.zoomed {
            layout.panes.push(PaneArea { pane, rect: area });
            return layout;
        }
        place(&self.root, area, &mut 0, &mut layout);
        layout
    }
}

impl Default for SplitTree {
    fn default() -> Self {
        Self::new().0
    }
}

/// Pointer-driven resizing of a [`SplitTree`].
///
/// [`SplitDrag::press`] on a divider of the current [`SplitLayout`], [`SplitDrag::drag`] while the
/// pointer moves, [`SplitDrag::release`] on button up. The pane sizes follow the pointer one cell
/// at a time and stop at the splits' minimum sizes. [`SplitDrag::cancel`] puts the split back as
/// it was.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitDrag {
    grabbed: Option<(Divider, u16)>,
}

impl SplitDrag {
    /// Starts idle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The divider being dragged.
    #[must_use]
    pub fn active(&self) -> Option<DividerId> {
        self.grabbed.map(|(divider, _)| divider.id)
    }

    /// Begins a drag when `(column, row)` is on a divider. Returns whether one began.
    pub fn press(&mut self, tree: &SplitTree, layout: &SplitLayout, column: u16, row: u16) -> bool {
        self.grabbed = layout
            .divider_at(column, row)
            .and_then(|divider| Some((*divider, tree.ratio(divider.id)?)));
        self.grabbed.is_some()
    }

    /// Moves the grabbed divider to follow the pointer at `(column, row)`.
    pub fn drag(&mut self, tree: &mut SplitTree, column: u16, row: u16) {
        let Some((divider, _)) = self.grabbed else {
            return;
        };
        let (start, along, total) = match divider.axis {
            SplitAxis::Horizontal => (divider.bounds.x, column, divider.bounds.width),
            SplitAxis::Vertical => (divider.bounds.y, row, divider.bounds.height),
        };
        let inner = total.saturating_sub(1);
        let first = along
            .saturating_sub(start)
            .max(divider.min_first)
            .min(
                inner
                    .saturating_sub(divider.min_second)
                    .max(divider.min_first),
            )
            .min(inner);
        tree.set_ratio(divider.id, ratio_for(first, inner));
    }

    /// Ends the drag, keeping the new sizes. Returns whether a drag was in progress.
    pub fn release(&mut self) -> bool {
        self.grabbed.take().is_some()
    }

    /// Ends the drag and puts the split back as it was when pressed.
    pub fn cancel(&mut self, tree: &mut SplitTree) {
        if let Some((divider, ratio)) = self.grabbed.take() {
            tree.set_ratio(divider.id, ratio);
        }
    }
}

fn contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x && column < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn first_size(ratio: u16, inner: u16, min: [u16; 2]) -> u16 {
    let raw = (u32::from(ratio) * u32::from(inner) + 500) / u32::from(FULL);
    let raw = u16::try_from(raw).unwrap_or(inner).min(inner);
    let hi = inner.saturating_sub(min[1]);
    if min[0] > hi {
        raw
    } else {
        raw.clamp(min[0], hi)
    }
}

fn ratio_for(first: u16, inner: u16) -> u16 {
    if inner == 0 {
        return FULL / 2;
    }
    let exact = (u32::from(first) * u32::from(FULL)).div_ceil(u32::from(inner));
    u16::try_from(exact).unwrap_or(FULL).min(FULL)
}

fn place(node: &Node, rect: Rect, counter: &mut usize, layout: &mut SplitLayout) {
    match node {
        Node::Leaf(pane) => layout.panes.push(PaneArea { pane: *pane, rect }),
        Node::Split(split) => {
            let id = DividerId(*counter);
            *counter += 1;
            let (total, horizontal) = match split.axis {
                SplitAxis::Horizontal => (rect.width, true),
                SplitAxis::Vertical => (rect.height, false),
            };
            let inner = total.saturating_sub(1);
            let first = first_size(split.ratio, inner, split.min);
            let second = inner - first;
            let (a, line, b) = if horizontal {
                (
                    Rect::new(rect.x, rect.y, first, rect.height),
                    Rect::new(rect.x + first, rect.y, 1.min(total), rect.height),
                    Rect::new(rect.x + first + 1.min(total), rect.y, second, rect.height),
                )
            } else {
                (
                    Rect::new(rect.x, rect.y, rect.width, first),
                    Rect::new(rect.x, rect.y + first, rect.width, 1.min(total)),
                    Rect::new(rect.x, rect.y + first + 1.min(total), rect.width, second),
                )
            };
            if total > 0 {
                layout.dividers.push(Divider {
                    id,
                    axis: split.axis,
                    rect: line,
                    bounds: rect,
                    min_first: split.min[0],
                    min_second: split.min[1],
                });
            }
            place(&split.children[0], a, counter, layout);
            place(&split.children[1], b, counter, layout);
        }
    }
}

fn split_leaf(node: &mut Node, pane: PaneId, axis: SplitAxis, ratio: u16, fresh: PaneId) -> bool {
    match node {
        Node::Leaf(id) if *id == pane => {
            *node = Node::Split(Box::new(Split {
                axis,
                ratio,
                min: [0, 0],
                children: [Node::Leaf(pane), Node::Leaf(fresh)],
            }));
            true
        }
        Node::Leaf(_) => false,
        Node::Split(split) => {
            split_leaf(&mut split.children[0], pane, axis, ratio, fresh)
                || split_leaf(&mut split.children[1], pane, axis, ratio, fresh)
        }
    }
}

fn close_leaf(node: &mut Node, pane: PaneId) -> bool {
    let Node::Split(split) = node else {
        return false;
    };
    for keep in [1, 0] {
        if split.children[1 - keep] == Node::Leaf(pane) {
            let survivor = std::mem::replace(&mut split.children[keep], Node::Leaf(GONE));
            *node = survivor;
            return true;
        }
    }
    close_leaf(&mut split.children[0], pane) || close_leaf(&mut split.children[1], pane)
}

fn count_leaves(node: &Node) -> usize {
    match node {
        Node::Leaf(_) => 1,
        Node::Split(split) => count_leaves(&split.children[0]) + count_leaves(&split.children[1]),
    }
}

fn has_leaf(node: &Node, pane: PaneId) -> bool {
    match node {
        Node::Leaf(id) => *id == pane,
        Node::Split(split) => {
            has_leaf(&split.children[0], pane) || has_leaf(&split.children[1], pane)
        }
    }
}

fn collect_leaves(node: &Node, panes: &mut Vec<PaneId>) {
    match node {
        Node::Leaf(id) => panes.push(*id),
        Node::Split(split) => {
            collect_leaves(&split.children[0], panes);
            collect_leaves(&split.children[1], panes);
        }
    }
}

fn split_mut<'a>(node: &'a mut Node, target: usize, counter: &mut usize) -> Option<&'a mut Split> {
    let Node::Split(split) = node else {
        return None;
    };
    let here = *counter;
    *counter += 1;
    if here == target {
        return Some(split);
    }
    let [first, second] = &mut split.children;
    match split_mut(first, target, counter) {
        Some(found) => Some(found),
        None => split_mut(second, target, counter),
    }
}

fn split_ref<'a>(node: &'a Node, target: usize, counter: &mut usize) -> Option<&'a Split> {
    let Node::Split(split) = node else {
        return None;
    };
    let here = *counter;
    *counter += 1;
    if here == target {
        return Some(split);
    }
    split_ref(&split.children[0], target, counter)
        .or_else(|| split_ref(&split.children[1], target, counter))
}
