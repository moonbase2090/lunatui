// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_layout::{Breakpoints, Constraint, Layout};

fn sizes(rows: &[(u16, &'static str)]) -> Breakpoints<&'static str> {
    let mut chosen = Breakpoints::new("base");
    for (min, name) in rows {
        chosen = chosen.at(*min, name);
    }
    chosen
}

#[test]
fn sizes_below_the_first_breakpoint_use_the_base() {
    let chosen = sizes(&[(40, "medium"), (80, "wide")]);
    assert_eq!(*chosen.select(0), "base");
    assert_eq!(*chosen.select(39), "base");
    assert_eq!(chosen.index(39), 0);
}

#[test]
fn a_breakpoint_takes_over_exactly_at_its_minimum() {
    let chosen = sizes(&[(40, "medium"), (80, "wide")]);
    assert_eq!(*chosen.select(40), "medium");
    assert_eq!(*chosen.select(79), "medium");
    assert_eq!(*chosen.select(80), "wide");
    assert_eq!(*chosen.select(u16::MAX), "wide");
    assert_eq!(chosen.index(40), 1);
    assert_eq!(chosen.index(80), 2);
}

#[test]
fn breakpoints_may_be_added_in_any_order() {
    let ascending = sizes(&[(40, "medium"), (80, "wide")]);
    let shuffled = sizes(&[(80, "wide"), (40, "medium")]);
    assert_eq!(ascending, shuffled);
    assert_eq!(*shuffled.select(50), "medium");
}

#[test]
fn the_same_minimum_replaces_its_value() {
    let chosen = sizes(&[(40, "first"), (40, "second")]);
    assert_eq!(*chosen.select(40), "second");
    assert_eq!(chosen.len(), 2);
}

#[test]
fn a_zero_minimum_replaces_the_base_at_every_size() {
    let chosen = sizes(&[(0, "always")]);
    assert_eq!(*chosen.select(0), "always");
    assert_eq!(*chosen.select(500), "always");
    assert_eq!(chosen.index(0), 1);
}

#[test]
fn a_lone_base_is_always_selected_and_counts_as_one_arrangement() {
    let chosen = Breakpoints::new(7u8);
    assert_eq!(*chosen.select(0), 7);
    assert_eq!(*chosen.select(u16::MAX), 7);
    assert_eq!(chosen.len(), 1);
    assert!(!chosen.is_empty());
    assert_eq!(chosen.index(u16::MAX), 0);
}

#[test]
fn len_counts_the_base_and_every_distinct_breakpoint() {
    assert_eq!(sizes(&[(10, "a"), (20, "b"), (30, "c")]).len(), 4);
}

#[test]
fn layouts_switch_between_stacked_and_side_by_side_by_width() {
    let chosen = Breakpoints::new(Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
    ]))
    .at(
        60,
        Layout::horizontal([Constraint::Length(20), Constraint::Fill(1)]),
    );
    let narrow = Rect::new(2, 1, 59, 10);
    let wide = Rect::new(2, 1, 60, 10);
    let stacked = chosen.select(narrow.width).split(narrow);
    let side = chosen.select(wide.width).split(wide);
    assert_eq!(stacked.segments[0], Rect::new(2, 1, 59, 3));
    assert_eq!(stacked.segments[1], Rect::new(2, 4, 59, 7));
    assert_eq!(side.segments[0], Rect::new(2, 1, 20, 10));
    assert_eq!(side.segments[1], Rect::new(22, 1, 40, 10));
}

#[test]
fn the_selector_can_follow_height_instead_of_width() {
    let chosen = Breakpoints::new("compact").at(24, "roomy");
    let area = Rect::new(0, 0, 200, 23);
    assert_eq!(*chosen.select(area.height), "compact");
    assert_eq!(*chosen.select(area.height + 1), "roomy");
}
