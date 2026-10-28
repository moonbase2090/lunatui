// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Rect, Style};
use lunatui_runtime::{HitId, HitMap};
use lunatui_term::{Modifiers, MouseButton, MouseEvent, MouseKind, MouseUnit};
use lunatui_widgets::{List, Tabs, VecSource};

const TABS: HitId = HitId(100);
const ROWS: HitId = HitId(200);
const POPUP: HitId = HitId(900);

fn click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: Modifiers::default(),
        unit: MouseUnit::Cells,
    }
}

fn tabs() -> Tabs {
    Tabs::new(["One", "Two", "Three"])
        .style(Style::new())
        .indent(1)
}

fn list() -> List<VecSource> {
    let rows = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|text| vec![(*text).to_owned()])
        .collect();
    List::new(VecSource::from_rows(rows)).offset(1)
}

fn tab_hit(map: &HitMap, event: &MouseEvent) -> Option<usize> {
    map.pick(event.column, event.row)?.index_in(TABS, 3)
}

fn row_hit(map: &HitMap, event: &MouseEvent) -> Option<usize> {
    map.pick(event.column, event.row)?.index_in(ROWS, 5)
}

#[test]
fn a_click_on_a_tab_title_reaches_that_tab() {
    let area = Rect::new(2, 1, 20, 1);
    let mut map = HitMap::new();
    map.push_indexed(TABS, tabs().tab_areas(area), 0);
    // One at 3..6, Two at 7..10, Three at 11..16.
    for (column, expected) in [
        (3, Some(0)),
        (5, Some(0)),
        (6, None),
        (7, Some(1)),
        (9, Some(1)),
        (10, None),
        (11, Some(2)),
        (15, Some(2)),
        (16, None),
        (2, None),
    ] {
        assert_eq!(
            tab_hit(&map, &click(column, 1)),
            expected,
            "column {column}"
        );
    }
    assert_eq!(tab_hit(&map, &click(4, 0)), None);
    assert_eq!(tab_hit(&map, &click(4, 2)), None);
}

#[test]
fn a_click_on_a_boxed_tab_reaches_its_slot_but_not_the_divider() {
    let area = Rect::new(0, 0, 20, 3);
    let boxed = Tabs::new(["A", "BB", "C"]).boxed(Style::new(), Style::new());
    let mut map = HitMap::new();
    map.push_indexed(TABS, boxed.tab_areas(area), 0);
    // Slots: A 0..5, BB 6..12, C 13..18.
    for (column, row, expected) in [
        (0, 0, Some(0)),
        (4, 1, Some(0)),
        (5, 1, None),
        (6, 0, Some(1)),
        (11, 1, Some(1)),
        (12, 1, None),
        (13, 1, Some(2)),
        (4, 2, None),
    ] {
        assert_eq!(
            tab_hit(&map, &click(column, row)),
            expected,
            "({column}, {row})"
        );
    }
}

#[test]
fn a_clipped_last_tab_is_clickable_only_where_it_shows() {
    let boxed = Tabs::new(["A", "BB"]).boxed(Style::new(), Style::new());
    let areas = boxed.tab_areas(Rect::new(0, 0, 8, 3));
    assert_eq!(
        areas,
        [(0, Rect::new(0, 0, 5, 2)), (1, Rect::new(6, 0, 2, 2))]
    );
}

#[test]
fn tabs_that_do_not_fit_get_no_region() {
    let areas = tabs().tab_areas(Rect::new(0, 0, 5, 1));
    assert_eq!(areas, [(0, Rect::new(1, 0, 3, 1))]);
    assert_eq!(tabs().tab_areas(Rect::new(0, 0, 0, 1)), []);
    assert_eq!(tabs().tab_areas(Rect::new(0, 0, 5, 0)), []);
}

#[test]
fn a_click_on_a_list_row_reaches_the_item_under_it() {
    let area = Rect::new(4, 2, 6, 3);
    let mut map = HitMap::new();
    map.push_indexed(ROWS, list().row_areas(area), 0);
    // Offset 1: screen row 2 is item 1, then 2 and 3.
    for (column, row, expected) in [
        (4, 2, Some(1)),
        (9, 2, Some(1)),
        (10, 2, None),
        (3, 2, None),
        (5, 3, Some(2)),
        (5, 4, Some(3)),
        (5, 5, None),
        (5, 1, None),
    ] {
        assert_eq!(
            row_hit(&map, &click(column, row)),
            expected,
            "({column}, {row})"
        );
    }
}

#[test]
fn rows_past_the_end_of_the_source_are_not_clickable() {
    let area = Rect::new(0, 0, 4, 10);
    let areas = list().row_areas(area);
    assert_eq!(areas.len(), 4);
    assert_eq!(areas[3], (4, Rect::new(0, 3, 4, 1)));
    assert_eq!(list().row_areas(Rect::new(0, 0, 0, 3)), []);
    assert_eq!(list().row_areas(Rect::new(0, 0, 4, 0)), []);
}

#[test]
fn a_higher_layer_over_the_widgets_takes_the_click() {
    let mut map = HitMap::new();
    map.push_indexed(TABS, tabs().tab_areas(Rect::new(0, 0, 20, 1)), 0);
    map.push_indexed(ROWS, list().row_areas(Rect::new(0, 1, 10, 4)), 0);
    map.push(POPUP, Rect::new(2, 0, 6, 3), 1);
    assert_eq!(map.pick(4, 0), Some(POPUP));
    assert_eq!(tab_hit(&map, &click(4, 0)), None);
    assert_eq!(map.pick(4, 2), Some(POPUP));
    assert_eq!(row_hit(&map, &click(4, 2)), None);
    // Outside the popup the widgets answer again.
    assert_eq!(tab_hit(&map, &click(12, 0)), Some(2));
    assert_eq!(row_hit(&map, &click(4, 3)), Some(3));
    assert_eq!(map.pick(9, 1), Some(HitId(201)));
}

#[test]
fn a_later_registration_at_the_same_layer_wins() {
    let mut map = HitMap::new();
    map.push_indexed(ROWS, list().row_areas(Rect::new(0, 0, 10, 4)), 0);
    map.push_indexed(TABS, tabs().tab_areas(Rect::new(0, 0, 20, 1)), 0);
    assert_eq!(map.pick(2, 0), Some(HitId(100)));
    assert_eq!(map.pick(2, 1), Some(HitId(202)));
}

#[test]
fn index_in_rejects_ids_outside_the_group() {
    assert_eq!(HitId(100).index_in(TABS, 3), Some(0));
    assert_eq!(HitId(102).index_in(TABS, 3), Some(2));
    assert_eq!(HitId(103).index_in(TABS, 3), None);
    assert_eq!(HitId(99).index_in(TABS, 3), None);
    assert_eq!(HitId(100).index_in(TABS, 0), None);
}

#[test]
fn push_indexed_saturates_instead_of_wrapping() {
    let mut map = HitMap::new();
    map.push_indexed(HitId(u32::MAX), [(3, Rect::new(0, 0, 1, 1))], 0);
    assert_eq!(map.pick(0, 0), Some(HitId(u32::MAX)));
}

#[test]
fn a_plain_strip_clips_titles_at_the_right_edge() {
    let plain = Tabs::new(["abc", "defgh", "ij"]);
    assert_eq!(
        plain.tab_areas(Rect::new(3, 2, 7, 1)),
        [(0, Rect::new(3, 2, 3, 1)), (1, Rect::new(7, 2, 3, 1))]
    );
    // A title that ends exactly on the edge leaves nothing for the next one.
    assert_eq!(
        plain.tab_areas(Rect::new(3, 2, 3, 1)),
        [(0, Rect::new(3, 2, 3, 1))]
    );
    // Wide characters count by display width and never straddle the edge.
    assert_eq!(
        Tabs::new(["日本"]).tab_areas(Rect::new(0, 0, 3, 1)),
        [(0, Rect::new(0, 0, 2, 1))]
    );
    assert_eq!(plain.clone().indent(9).tab_areas(Rect::new(0, 0, 5, 1)), []);
}

#[test]
fn a_boxed_strip_shorter_than_three_rows_falls_back_to_titles() {
    let boxed = Tabs::new(["A", "B"]).boxed(Style::new(), Style::new());
    assert_eq!(
        boxed.tab_areas(Rect::new(1, 0, 6, 2)),
        [(0, Rect::new(1, 0, 1, 1)), (1, Rect::new(3, 0, 1, 1))]
    );
}

#[test]
fn a_boxed_strip_keeps_clear_of_joined_parent_borders() {
    let boxed = Tabs::new(["A", "BB"])
        .boxed(Style::new(), Style::new())
        .join_parent_border(true);
    assert_eq!(
        boxed.tab_areas(Rect::new(4, 1, 20, 3)),
        [(0, Rect::new(5, 1, 5, 2)), (1, Rect::new(11, 1, 6, 2))]
    );
    // The last slot may use every cell up to the inner edge, and no more.
    assert_eq!(
        boxed.tab_areas(Rect::new(4, 1, 13, 3)),
        [(0, Rect::new(5, 1, 5, 2)), (1, Rect::new(11, 1, 5, 2))]
    );
    assert_eq!(
        boxed.tab_areas(Rect::new(4, 1, 12, 3)),
        [(0, Rect::new(5, 1, 5, 2)), (1, Rect::new(11, 1, 4, 2))]
    );
    assert_eq!(boxed.tab_areas(Rect::new(4, 1, 2, 3)), []);
    assert_eq!(
        boxed.tab_areas(Rect::new(4, 1, 3, 3)),
        [(0, Rect::new(5, 1, 1, 2))]
    );
}

#[test]
fn a_boxed_strip_honors_indent_and_an_exact_fit() {
    let boxed = Tabs::new(["A", "B"])
        .boxed(Style::new(), Style::new())
        .indent(2);
    assert_eq!(
        boxed.tab_areas(Rect::new(1, 0, 14, 3)),
        [(0, Rect::new(3, 0, 5, 2)), (1, Rect::new(9, 0, 5, 2))]
    );
    // The first slot fits exactly: all five cells, nothing left for a second tab.
    assert_eq!(
        boxed.tab_areas(Rect::new(1, 0, 7, 3)),
        [(0, Rect::new(3, 0, 5, 2))]
    );
    assert_eq!(boxed.tab_areas(Rect::new(0, 0, 0, 3)), []);
}
