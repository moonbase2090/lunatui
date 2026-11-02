// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Keyboard resizing: `SplitTree::resize` and `SplitTree::divider_for`.

use lunatui_core::Rect;
use lunatui_widgets::{DividerId, PaneId, SplitAxis, SplitDrag, SplitLayout, SplitTree};

const AREA: Rect = Rect::new(0, 0, 21, 5);
const D0: DividerId = DividerId(0);
const D1: DividerId = DividerId(1);
const D2: DividerId = DividerId(2);

fn pair(axis: SplitAxis, ratio: u16) -> (SplitTree, PaneId, PaneId) {
    let (mut tree, first) = SplitTree::new();
    let second = tree.split(first, axis, ratio).unwrap_or(first);
    (tree, first, second)
}

fn rects(layout: &SplitLayout) -> Vec<(u32, Rect)> {
    layout
        .panes
        .iter()
        .map(|area| (area.pane.0, area.rect))
        .collect()
}

/// Panes 0 | (1 over (2 | 3)): dividers 0 horizontal, 1 vertical, 2 horizontal.
fn three_deep() -> SplitTree {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(b, SplitAxis::Vertical, 500).unwrap_or(a);
    tree.split(c, SplitAxis::Horizontal, 500);
    tree
}

#[test]
fn growing_the_first_pane_moves_the_divider_right() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    assert!(tree.resize(D0, 3, AREA));
    assert_eq!(tree.ratio(D0), Some(650));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 13, 5)), (b.0, Rect::new(14, 0, 7, 5))]
    );
}

#[test]
fn a_negative_delta_shrinks_the_first_pane() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    assert!(tree.resize(D0, -4, AREA));
    assert_eq!(tree.ratio(D0), Some(300));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 6, 5)), (b.0, Rect::new(7, 0, 14, 5))]
    );
}

#[test]
fn a_vertical_split_resizes_by_rows() {
    let (mut tree, a, b) = pair(SplitAxis::Vertical, 500);
    assert!(tree.resize(D0, 1, AREA));
    assert_eq!(tree.ratio(D0), Some(750));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 21, 3)), (b.0, Rect::new(0, 4, 21, 1))]
    );
}

#[test]
fn repeated_steps_add_up() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    for _ in 0..3 {
        assert!(tree.resize(D0, 1, AREA));
    }
    assert_eq!(tree.ratio(D0), Some(650));
    assert!(tree.resize(D0, -1, AREA));
    assert_eq!(tree.ratio(D0), Some(600));
}

#[test]
fn a_zero_delta_leaves_the_ratio_alone() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 333);
    assert!(tree.resize(D0, 0, AREA));
    assert_eq!(tree.ratio(D0), Some(333));
}

#[test]
fn the_first_pane_stops_at_its_minimum() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    tree.set_min(D0, 4, 5);
    assert!(tree.resize(D0, -100, AREA));
    assert_eq!(tree.ratio(D0), Some(200));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 4, 5)), (b.0, Rect::new(5, 0, 16, 5))]
    );
}

#[test]
fn the_second_pane_stops_at_its_minimum() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    tree.set_min(D0, 4, 5);
    assert!(tree.resize(D0, 100, AREA));
    assert_eq!(tree.ratio(D0), Some(750));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 15, 5)), (b.0, Rect::new(16, 0, 5, 5))]
    );
}

#[test]
fn without_minimums_the_divider_reaches_each_edge() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    assert!(tree.resize(D0, i16::MAX, AREA));
    assert_eq!(tree.ratio(D0), Some(1000));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 20, 5)), (b.0, Rect::new(21, 0, 0, 5))]
    );
    assert!(tree.resize(D0, i16::MIN, AREA));
    assert_eq!(tree.ratio(D0), Some(0));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [(a.0, Rect::new(0, 0, 0, 5)), (b.0, Rect::new(1, 0, 20, 5))]
    );
}

#[test]
fn minimums_too_big_for_the_area_favor_the_first_pane() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    tree.set_min(D0, 15, 15);
    assert!(tree.resize(D0, -1, AREA));
    assert_eq!(tree.ratio(D0), Some(750));
}

#[test]
fn a_step_lands_where_a_drag_to_the_same_cell_does() {
    let (mut stepped, _, _) = pair(SplitAxis::Horizontal, 500);
    stepped.set_min(D0, 3, 4);
    let (mut dragged, _, _) = pair(SplitAxis::Horizontal, 500);
    dragged.set_min(D0, 3, 4);
    let layout = dragged.layout(AREA);
    let mut drag = SplitDrag::new();
    assert!(drag.press(&dragged, &layout, 10, 2));
    drag.drag(&mut dragged, 13, 2);
    assert!(stepped.resize(D0, 3, AREA));
    assert_eq!(stepped.ratio(D0), Some(650));
    assert_eq!(dragged.ratio(D0), Some(650));
    let mut drag = SplitDrag::new();
    let layout = dragged.layout(AREA);
    assert!(drag.press(&dragged, &layout, 13, 2));
    drag.drag(&mut dragged, 20, 2);
    assert!(stepped.resize(D0, 7, AREA));
    assert_eq!(stepped.ratio(D0), Some(800));
    assert_eq!(dragged.ratio(D0), Some(800));
}

#[test]
fn a_nested_divider_moves_inside_its_own_split() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(b, SplitAxis::Vertical, 500).unwrap_or(a);
    assert!(tree.resize(D1, 1, AREA));
    assert_eq!(tree.ratio(D0), Some(500));
    assert_eq!(tree.ratio(D1), Some(750));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [
            (a.0, Rect::new(0, 0, 10, 5)),
            (b.0, Rect::new(11, 0, 10, 3)),
            (c.0, Rect::new(11, 4, 10, 1)),
        ]
    );
}

#[test]
fn a_nested_divider_in_the_first_pane_uses_that_panes_width() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(a, SplitAxis::Horizontal, 500).unwrap_or(a);
    assert!(tree.resize(D1, 2, AREA));
    assert_eq!(tree.ratio(D1), Some(778));
    assert_eq!(
        rects(&tree.layout(AREA)),
        [
            (a.0, Rect::new(0, 0, 7, 5)),
            (c.0, Rect::new(8, 0, 2, 5)),
            (b.0, Rect::new(11, 0, 10, 5)),
        ]
    );
    tree.set_min(D1, 0, 3);
    assert!(tree.resize(D1, 100, AREA));
    assert_eq!(tree.ratio(D1), Some(667));
    assert_eq!(tree.ratio(D0), Some(500));
}

#[test]
fn an_unknown_divider_changes_nothing() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    let before = tree.clone();
    assert!(!tree.resize(DividerId(5), 3, AREA));
    assert_eq!(tree, before);
    let (mut single, _) = SplitTree::new();
    assert!(!single.resize(D0, 1, AREA));
}

#[test]
fn a_zoomed_tree_does_not_resize() {
    let (mut tree, a, _) = pair(SplitAxis::Horizontal, 500);
    tree.zoom(Some(a));
    assert!(!tree.resize(D0, 3, AREA));
    assert_eq!(tree.ratio(D0), Some(500));
    tree.zoom(None);
    assert!(tree.resize(D0, 3, AREA));
    assert_eq!(tree.ratio(D0), Some(650));
}

#[test]
fn an_area_too_small_for_the_divider_leaves_the_ratio_alone() {
    let (mut tree, _, _) = pair(SplitAxis::Horizontal, 500);
    assert!(tree.resize(D0, 3, Rect::new(0, 0, 0, 5)));
    assert_eq!(tree.ratio(D0), Some(500));
}

#[test]
fn divider_for_finds_the_nearest_split_of_each_axis() {
    let tree = three_deep();
    let h = SplitAxis::Horizontal;
    let v = SplitAxis::Vertical;
    assert_eq!(tree.divider_for(PaneId(0), h), Some(D0));
    assert_eq!(tree.divider_for(PaneId(0), v), None);
    assert_eq!(tree.divider_for(PaneId(1), v), Some(D1));
    assert_eq!(tree.divider_for(PaneId(1), h), Some(D0));
    assert_eq!(tree.divider_for(PaneId(2), h), Some(D2));
    assert_eq!(tree.divider_for(PaneId(2), v), Some(D1));
    assert_eq!(tree.divider_for(PaneId(3), h), Some(D2));
    assert_eq!(tree.divider_for(PaneId(3), v), Some(D1));
}

#[test]
fn divider_for_counts_the_splits_of_earlier_subtrees() {
    let (mut tree, a, b) = pair(SplitAxis::Horizontal, 500);
    let c = tree.split(a, SplitAxis::Vertical, 500).unwrap_or(a);
    let v = SplitAxis::Vertical;
    assert_eq!(tree.divider_for(c, SplitAxis::Horizontal), Some(D0));
    assert_eq!(tree.divider_for(a, v), Some(D1));
    assert_eq!(tree.divider_for(c, v), Some(D1));
    assert_eq!(tree.divider_for(b, v), None);
}

#[test]
fn divider_for_a_missing_pane_or_a_lone_pane_is_none() {
    let tree = three_deep();
    assert_eq!(tree.divider_for(PaneId(9), SplitAxis::Horizontal), None);
    let (single, only) = SplitTree::new();
    assert_eq!(single.divider_for(only, SplitAxis::Vertical), None);
}

#[test]
fn divider_for_is_none_while_zoomed() {
    let mut tree = three_deep();
    tree.zoom(Some(PaneId(2)));
    assert_eq!(tree.divider_for(PaneId(2), SplitAxis::Horizontal), None);
    assert_eq!(tree.divider_for(PaneId(0), SplitAxis::Horizontal), None);
    tree.zoom(None);
    assert_eq!(tree.divider_for(PaneId(2), SplitAxis::Horizontal), Some(D2));
}

#[test]
fn resizing_the_focused_pane_moves_its_nearest_divider() {
    let mut tree = three_deep();
    let divider = tree.divider_for(PaneId(3), SplitAxis::Horizontal);
    assert_eq!(divider, Some(D2));
    assert!(tree.resize(D2, 1, AREA));
    assert_eq!(tree.ratio(D0), Some(500));
    assert_eq!(tree.ratio(D1), Some(500));
    assert_eq!(tree.ratio(D2), Some(667));
}
