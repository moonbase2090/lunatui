// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{SplitAxis, SplitDrag, SplitLayout, SplitTree};

const AREA: Rect = Rect::new(0, 0, 41, 10);
const PANES: u32 = 10;
const DIVIDERS: u32 = 500;

fn map(layout: &SplitLayout) -> HitMap {
    let mut map = HitMap::new();
    for area in &layout.panes {
        map.push(HitId(PANES + area.pane.0), area.rect, 0);
    }
    for divider in &layout.dividers {
        map.push(
            HitId(DIVIDERS + u32::try_from(divider.id.0).unwrap_or(0)),
            divider.rect,
            1,
        );
    }
    map
}

fn tree() -> SplitTree {
    let (mut tree, left) = SplitTree::new();
    let Some(right) = tree.split(left, SplitAxis::Horizontal, 500) else {
        unreachable!("the pane exists");
    };
    tree.split(right, SplitAxis::Vertical, 500);
    tree
}

#[test]
fn clicks_reach_panes_and_dividers_through_the_hit_map() {
    let tree = tree();
    let map = map(&tree.layout(AREA));
    assert_eq!(map.pick(5, 5), Some(HitId(10)));
    assert_eq!(map.pick(25, 1), Some(HitId(11)));
    assert_eq!(map.pick(25, 8), Some(HitId(12)));
    assert_eq!(map.pick(20, 5), Some(HitId(500)));
    assert_eq!(map.pick(30, 5), Some(HitId(501)));
    assert_eq!(map.pick(30, 4), Some(HitId(11)));
}

#[test]
fn a_press_drag_release_sequence_resizes_the_panes_and_the_map_follows() {
    let mut tree = tree();
    let mut drag = SplitDrag::new();
    let before = tree.layout(AREA);
    assert_eq!(map(&before).pick(20, 5), Some(HitId(500)));
    assert!(drag.press(&tree, &before, 20, 5));
    for column in [18, 15, 12, 10] {
        drag.drag(&mut tree, column, 5);
        let layout = tree.layout(AREA);
        assert_eq!(layout.divider_at(column, 5).map(|d| d.id.0), Some(0));
        assert_eq!(map(&layout).pick(column, 5), Some(HitId(500)));
    }
    assert!(drag.release());
    let after = tree.layout(AREA);
    assert_eq!(map(&after).pick(5, 5), Some(HitId(10)));
    assert_eq!(map(&after).pick(10, 5), Some(HitId(500)));
    assert_eq!(map(&after).pick(20, 5), Some(HitId(501)));
    assert_eq!(map(&after).pick(20, 8), Some(HitId(12)));
}

#[test]
fn zooming_leaves_one_pane_and_no_dividers_to_hit() {
    let mut tree = tree();
    let Some(&pane) = tree.panes().get(1) else {
        unreachable!("three panes");
    };
    tree.zoom(Some(pane));
    let map = map(&tree.layout(AREA));
    assert_eq!(map.pick(20, 5), Some(HitId(11)));
    assert_eq!(map.pick(0, 0), Some(HitId(11)));
    assert_eq!(map.pick(40, 9), Some(HitId(11)));
}
