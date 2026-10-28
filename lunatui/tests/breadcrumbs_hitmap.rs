// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::Breadcrumbs;

const CRUMBS: HitId = HitId(40);
const PATH: [&str; 4] = ["Home", "Projects", "lunatui", "src"];

fn map(width: u16) -> HitMap {
    let mut map = HitMap::new();
    let areas = Breadcrumbs::new(PATH).segment_areas(Rect::new(2, 1, width, 1));
    map.push_indexed(CRUMBS, areas, 0);
    map
}

fn segment_at(map: &HitMap, column: u16, row: u16) -> Option<usize> {
    map.pick(column, row)?.index_in(CRUMBS, PATH.len())
}

#[test]
fn a_click_on_a_segment_reaches_its_path_index() {
    let map = map(40);
    assert_eq!(segment_at(&map, 2, 1), Some(0));
    assert_eq!(segment_at(&map, 5, 1), Some(0));
    assert_eq!(segment_at(&map, 6, 1), None);
    assert_eq!(segment_at(&map, 9, 1), Some(1));
    assert_eq!(segment_at(&map, 16, 1), None);
    assert_eq!(segment_at(&map, 20, 1), Some(2));
    assert_eq!(segment_at(&map, 30, 1), Some(3));
    assert_eq!(segment_at(&map, 33, 1), None);
    assert_eq!(segment_at(&map, 9, 0), None);
    assert_eq!(segment_at(&map, 9, 2), None);
}

#[test]
fn collapsed_segments_are_not_clickable_and_the_rest_keep_their_index() {
    let map = map(24);
    assert_eq!(segment_at(&map, 2, 1), Some(0));
    assert_eq!(segment_at(&map, 9, 1), None);
    assert_eq!(segment_at(&map, 13, 1), Some(2));
    assert_eq!(segment_at(&map, 23, 1), Some(3));
}

#[test]
fn a_layer_over_the_path_takes_the_click() {
    let mut map = map(40);
    map.push(HitId(900), Rect::new(8, 0, 6, 3), 1);
    assert_eq!(segment_at(&map, 9, 1), None);
    assert_eq!(map.pick(9, 1), Some(HitId(900)));
    assert_eq!(segment_at(&map, 3, 1), Some(0));
}
