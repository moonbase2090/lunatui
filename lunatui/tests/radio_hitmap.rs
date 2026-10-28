// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use lunatui_widgets::{RadioFlow, RadioGroup};

const FIRST: HitId = HitId(70);

fn group() -> RadioGroup {
    RadioGroup::new(["Light", "Dark", "System"])
}

fn map(group: &RadioGroup, area: Rect) -> HitMap {
    let mut map = HitMap::new();
    map.push_indexed(FIRST, group.option_areas(area), 0);
    map
}

fn option_at(map: &HitMap, column: u16, row: u16) -> Option<usize> {
    map.pick(column, row)?.index_in(FIRST, 3)
}

#[test]
fn a_click_on_a_row_picks_that_option_and_the_widget_follows() {
    let area = Rect::new(4, 2, 20, 6);
    let mut selected = None;
    for (column, row, expected) in [(4, 2, 0), (9, 2, 0), (6, 3, 1), (11, 4, 2)] {
        let hits = map(&group().select(selected), area);
        selected = option_at(&hits, column, row);
        assert_eq!(selected, Some(expected), "({column}, {row})");
    }
    let hits = map(&group().select(selected), area);
    assert_eq!(option_at(&hits, 3, 2), None);
    assert_eq!(option_at(&hits, 4, 5), None);
    assert_eq!(option_at(&hits, 4 + 7, 2), None);
}

#[test]
fn horizontal_options_are_hit_across_the_row() {
    let across = group().flow(RadioFlow::Horizontal).gap(2);
    let hits = map(&across, Rect::new(0, 0, 40, 1));
    assert_eq!(option_at(&hits, 0, 0), Some(0));
    assert_eq!(option_at(&hits, 6, 0), Some(0));
    assert_eq!(option_at(&hits, 7, 0), None);
    assert_eq!(option_at(&hits, 9, 0), Some(1));
    assert_eq!(option_at(&hits, 14, 0), Some(1));
    assert_eq!(option_at(&hits, 15, 0), None);
    assert_eq!(option_at(&hits, 16, 0), None);
    assert_eq!(option_at(&hits, 17, 0), Some(2));
    assert_eq!(option_at(&hits, 24, 0), Some(2));
    assert_eq!(option_at(&hits, 25, 0), None);
}

#[test]
fn arrow_keys_walk_the_group_skipping_disabled_options() {
    let walker = group().disabled_options([1]);
    let mut selected = Some(0);
    selected = walker.step(selected, true);
    assert_eq!(selected, Some(2));
    selected = walker.step(selected, true);
    assert_eq!(selected, Some(0));
    selected = walker.step(selected, false);
    assert_eq!(selected, Some(2));
}

#[test]
fn a_dialog_over_the_group_takes_the_click() {
    let area = Rect::new(0, 0, 20, 3);
    let mut hits = map(&group(), area);
    hits.push(HitId(900), Rect::new(0, 1, 10, 1), 1);
    assert_eq!(hits.pick(3, 1), Some(HitId(900)));
    assert_eq!(option_at(&hits, 3, 1), None);
    assert_eq!(option_at(&hits, 3, 0), Some(0));
    assert_eq!(option_at(&hits, 3, 2), Some(2));
}
