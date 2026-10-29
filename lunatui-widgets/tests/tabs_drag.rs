// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Rect, Style};
use lunatui_widgets::{TabMove, Tabs, TabsDrag};

fn areas() -> Vec<(usize, Rect)> {
    Tabs::new(["One", "Two", "Three", "Four"]).tab_areas(Rect::new(0, 2, 20, 1))
}

fn drag_to(start: u16, end: u16) -> Option<TabMove> {
    let areas = areas();
    let mut drag = TabsDrag::new();
    assert!(drag.press(&areas, start, 2));
    drag.drag(&areas, end, 2);
    drag.release()
}

#[test]
fn dropping_on_a_later_tab_moves_the_tab_there() {
    assert_eq!(drag_to(1, 9), Some(TabMove { from: 0, to: 2 }));
    assert_eq!(drag_to(5, 15), Some(TabMove { from: 1, to: 3 }));
}

#[test]
fn dropping_on_an_earlier_tab_moves_the_tab_there() {
    assert_eq!(drag_to(15, 1), Some(TabMove { from: 3, to: 0 }));
    assert_eq!(drag_to(9, 5), Some(TabMove { from: 2, to: 1 }));
}

#[test]
fn a_press_and_release_on_the_same_tab_is_a_click() {
    assert_eq!(drag_to(5, 5), None);
    assert_eq!(drag_to(4, 6), None);
}

#[test]
fn dragging_past_either_end_picks_the_end_tab() {
    assert_eq!(drag_to(9, 0), Some(TabMove { from: 2, to: 0 }));
    assert_eq!(drag_to(5, 19), Some(TabMove { from: 1, to: 3 }));
    assert_eq!(drag_to(1, 19), Some(TabMove { from: 0, to: 3 }));
}

#[test]
fn the_gap_between_tabs_keeps_the_last_target() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    drag.press(&areas, 1, 2);
    drag.drag(&areas, 9, 2);
    assert_eq!(drag.target(), Some(2));
    drag.drag(&areas, 7, 2);
    assert_eq!(drag.target(), Some(2));
    drag.drag(&areas, 3, 2);
    assert_eq!(drag.target(), Some(2));
    drag.drag(&areas, 5, 2);
    assert_eq!(drag.target(), Some(1));
}

#[test]
fn leaving_the_strip_cancels_the_drop_and_returning_resumes_it() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    drag.press(&areas, 1, 2);
    drag.drag(&areas, 9, 5);
    assert_eq!(drag.target(), None);
    assert!(drag.is_dragging());
    assert_eq!(drag.release(), None);
    assert!(!drag.is_dragging());

    drag.press(&areas, 1, 2);
    drag.drag(&areas, 9, 5);
    drag.drag(&areas, 9, 2);
    assert_eq!(drag.target(), Some(2));
    assert_eq!(drag.release(), Some(TabMove { from: 0, to: 2 }));
}

#[test]
fn returning_into_a_gap_after_leaving_targets_the_dragged_tab() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    drag.press(&areas, 5, 2);
    drag.drag(&areas, 5, 0);
    drag.drag(&areas, 7, 2);
    assert_eq!(drag.target(), Some(1));
    assert_eq!(drag.release(), None);
}

#[test]
fn rows_above_and_below_the_strip_are_off_it() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    drag.press(&areas, 1, 2);
    for row in [0, 1, 3, 4] {
        drag.drag(&areas, 9, row);
        assert_eq!(drag.target(), None, "row {row}");
    }
}

#[test]
fn cancel_abandons_the_drag() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    drag.press(&areas, 1, 2);
    drag.drag(&areas, 9, 2);
    drag.cancel();
    assert!(!drag.is_dragging());
    assert_eq!(drag.from(), None);
    assert_eq!(drag.target(), None);
    assert_eq!(drag.release(), None);
}

#[test]
fn a_press_off_every_tab_starts_nothing() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    assert!(!drag.press(&areas, 3, 2));
    assert!(!drag.press(&areas, 1, 1));
    assert!(!drag.press(&areas, 19, 2));
    drag.drag(&areas, 9, 2);
    assert_eq!(drag.target(), None);
    assert_eq!(drag.release(), None);
}

#[test]
fn a_new_press_replaces_an_unfinished_drag() {
    let areas = areas();
    let mut drag = TabsDrag::new();
    drag.press(&areas, 1, 2);
    drag.drag(&areas, 9, 2);
    assert!(!drag.press(&areas, 3, 2));
    assert!(!drag.is_dragging());
    assert!(drag.press(&areas, 9, 2));
    assert_eq!(drag.from(), Some(2));
    assert_eq!(drag.target(), Some(2));
}

#[test]
fn boxed_strips_drag_by_their_slots() {
    let boxed = Tabs::new(["A", "B", "C"]).boxed(Style::new(), Style::new());
    let areas = boxed.tab_areas(Rect::new(0, 0, 30, 3));
    let mut drag = TabsDrag::new();
    assert!(drag.press(&areas, 2, 1));
    drag.drag(&areas, 13, 0);
    assert_eq!(drag.release(), Some(TabMove { from: 0, to: 2 }));
    assert!(!drag.press(&areas, 2, 2));
}

#[test]
fn applying_a_move_reorders_and_the_selection_follows_its_tab() {
    let mut titles = vec!["a", "b", "c", "d"];
    let reorder = TabMove { from: 0, to: 2 };
    reorder.apply(&mut titles);
    assert_eq!(titles, ["b", "c", "a", "d"]);
    assert_eq!(
        [0, 1, 2, 3].map(|selected| reorder.selection(selected)),
        [2, 0, 1, 3]
    );

    let mut titles = vec!["a", "b", "c", "d"];
    let reorder = TabMove { from: 3, to: 1 };
    reorder.apply(&mut titles);
    assert_eq!(titles, ["a", "d", "b", "c"]);
    assert_eq!(
        [0, 1, 2, 3].map(|selected| reorder.selection(selected)),
        [0, 2, 3, 1]
    );
}

#[test]
fn applying_an_out_of_range_move_changes_nothing() {
    let mut titles = vec!["a", "b"];
    TabMove { from: 0, to: 2 }.apply(&mut titles);
    TabMove { from: 5, to: 0 }.apply(&mut titles);
    assert_eq!(titles, ["a", "b"]);
}

#[test]
fn the_reordered_strip_keeps_the_same_tab_selected() {
    let mut titles = vec!["One", "Two", "Three", "Four"];
    let mut selected = 2;
    let areas = Tabs::new(titles.clone()).tab_areas(Rect::new(0, 2, 20, 1));
    let mut drag = TabsDrag::new();
    drag.press(&areas, 1, 2);
    drag.drag(&areas, 15, 2);
    let reorder = drag.release();
    let Some(reorder) = reorder else {
        unreachable!("a drop on another tab reorders");
    };
    reorder.apply(&mut titles);
    selected = reorder.selection(selected);
    assert_eq!(titles, ["Two", "Three", "Four", "One"]);
    assert_eq!(titles[selected], "Three");
}

#[test]
fn a_drag_in_the_middle_of_the_strip_picks_the_tab_under_it() {
    assert_eq!(drag_to(15, 12), Some(TabMove { from: 3, to: 2 }));
    assert_eq!(drag_to(1, 12), Some(TabMove { from: 0, to: 2 }));
}

#[test]
fn an_indented_strip_picks_the_first_tab_left_of_it() {
    let areas = Tabs::new(["One", "Two"])
        .indent(3)
        .tab_areas(Rect::new(0, 0, 20, 1));
    let mut drag = TabsDrag::new();
    drag.press(&areas, 8, 0);
    drag.drag(&areas, 1, 0);
    assert_eq!(drag.target(), Some(0));
    assert_eq!(drag.release(), Some(TabMove { from: 1, to: 0 }));
}

#[test]
fn a_move_that_starts_at_the_end_of_the_list_is_ignored() {
    let mut titles = vec!["a", "b"];
    TabMove { from: 2, to: 0 }.apply(&mut titles);
    assert_eq!(titles, ["a", "b"]);
}
