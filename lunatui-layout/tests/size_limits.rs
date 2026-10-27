// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_layout::{Constraint, Flex, Layout, Limit, aspect_fit};
use proptest::prelude::*;

fn rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
    Rect::new(x, y, width, height)
}

fn widths(layout: &Layout, width: u16) -> (Vec<u16>, bool) {
    let solved = layout.split(rect(0, 0, width, 1));
    let sizes = solved
        .segments
        .iter()
        .map(|segment| segment.width)
        .collect();
    (sizes, solved.needs_solver)
}

#[test]
fn a_floor_keeps_a_fill_from_collapsing_and_the_rest_is_shared_again() {
    let layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(3)])
        .limits([Limit::at_least(20)]);
    assert_eq!(widths(&layout, 40), (vec![20, 20], false));
}

#[test]
fn an_unlimited_segment_splits_as_before() {
    let layout =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(3)]).limits([Limit::NONE]);
    assert_eq!(widths(&layout, 40), (vec![10, 30], false));
}

#[test]
fn a_capped_fill_gives_its_extra_cells_to_the_other_fills() {
    let layout =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).limits([Limit::at_most(10)]);
    assert_eq!(widths(&layout, 100), (vec![10, 90], false));
}

#[test]
fn a_lone_capped_fill_leaves_the_rest_to_flex() {
    let solved = Layout::horizontal([Constraint::Fill(1)])
        .limits([Limit::between(30, 60)])
        .split(rect(0, 0, 100, 1));
    assert_eq!(solved.segments, [rect(0, 0, 60, 1)]);
    let gaps: Vec<u16> = solved.spacers.iter().map(|spacer| spacer.width).collect();
    assert_eq!(gaps, [0, 40]);
}

#[test]
fn center_flex_centres_a_capped_segment() {
    let solved = Layout::horizontal([Constraint::Fill(1)])
        .limits([Limit::at_most(10)])
        .flex(Flex::Center)
        .split(rect(0, 0, 30, 1));
    assert_eq!(solved.segments, [rect(10, 0, 10, 1)]);
}

#[test]
fn a_floor_the_area_cannot_hold_is_cut_and_sets_needs_solver() {
    let layout = Layout::horizontal([Constraint::Fill(1)]).limits([Limit::between(30, 60)]);
    assert_eq!(widths(&layout, 20), (vec![20], true));
}

#[test]
fn floors_that_do_not_fit_together_cut_the_last_segment_first() {
    let layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
        .limits([Limit::at_least(30), Limit::at_least(30)]);
    assert_eq!(widths(&layout, 50), (vec![30, 20], true));
}

#[test]
fn a_cap_shortens_a_length_and_the_fill_takes_the_cells() {
    let layout = Layout::horizontal([Constraint::Length(50), Constraint::Fill(1)])
        .limits([Limit::at_most(30)]);
    assert_eq!(widths(&layout, 80), (vec![30, 50], false));
}

#[test]
fn a_floor_above_the_cap_wins() {
    let layout = Layout::horizontal([Constraint::Max(10)]).limits([Limit::at_least(20)]);
    assert_eq!(widths(&layout, 80), (vec![20], false));
}

#[test]
fn legacy_flex_still_covers_the_area_past_a_cap() {
    let layout = Layout::horizontal([Constraint::Fill(1)])
        .limits([Limit::at_most(10)])
        .flex(Flex::Legacy);
    assert_eq!(widths(&layout, 30), (vec![30], false));
}

#[test]
fn limits_apply_to_heights_in_a_vertical_split() {
    let solved = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)])
        .limits([Limit::at_most(3)])
        .split(rect(0, 0, 12, 10));
    assert_eq!(solved.segments, [rect(0, 0, 12, 3), rect(0, 3, 12, 7)]);
}

#[test]
fn limits_past_the_last_constraint_are_ignored() {
    let layout = Layout::horizontal([Constraint::Fill(1)]).limits([Limit::NONE, Limit::at_most(1)]);
    assert_eq!(widths(&layout, 10), (vec![10], false));
}

#[test]
fn aspect_fit_is_height_bound_in_a_wide_area() {
    assert_eq!(aspect_fit(rect(0, 0, 100, 20), 2, 1), rect(30, 0, 40, 20));
}

#[test]
fn aspect_fit_is_width_bound_in_a_tall_area() {
    assert_eq!(aspect_fit(rect(0, 0, 20, 50), 2, 1), rect(0, 20, 20, 10));
}

#[test]
fn aspect_fit_puts_the_odd_cell_after_the_rectangle() {
    assert_eq!(aspect_fit(rect(0, 0, 41, 10), 2, 1), rect(10, 0, 20, 10));
}

#[test]
fn aspect_fit_keeps_the_area_offset() {
    assert_eq!(aspect_fit(rect(5, 3, 30, 30), 3, 2), rect(5, 8, 30, 20));
}

#[test]
fn aspect_fit_treats_a_zero_in_the_ratio_as_one() {
    assert_eq!(aspect_fit(rect(0, 0, 10, 4), 0, 1), rect(3, 0, 4, 4));
}

#[test]
fn aspect_fit_of_an_empty_area_is_empty() {
    assert_eq!(aspect_fit(rect(0, 0, 0, 0), 2, 1), rect(0, 0, 0, 0));
}

fn limited_fills() -> impl Strategy<Value = (u16, Vec<(u16, u16, u16)>)> {
    (
        20_u16..200,
        prop::collection::vec((1_u16..5, 0_u16..20, 0_u16..60), 1..6),
    )
}

proptest! {
    #[test]
    fn feasible_limits_hold_and_fills_use_every_cell_they_may(
        (width, parts) in limited_fills(),
    ) {
        let floors: u16 = parts.iter().map(|&(_, floor, _)| floor).sum();
        prop_assume!(floors <= width);
        let layout = Layout::horizontal(parts.iter().map(|&(weight, _, _)| Constraint::Fill(weight)))
            .limits(parts.iter().map(|&(_, floor, room)| Limit::between(floor, floor + room)));
        let (sizes, needs_solver) = widths(&layout, width);
        prop_assert!(!needs_solver);
        for (size, &(_, floor, room)) in sizes.iter().zip(&parts) {
            prop_assert!(*size >= floor && *size <= floor + room, "{size} outside {floor}..={}", floor + room);
        }
        let caps: u32 = parts.iter().map(|&(_, floor, room)| u32::from(floor + room)).sum();
        let used: u32 = sizes.iter().map(|&size| u32::from(size)).sum();
        prop_assert_eq!(used, caps.min(u32::from(width)));
    }

    #[test]
    fn aspect_fit_is_the_largest_centred_fit(
        (x, y, width, height) in (0_u16..50, 0_u16..50, 0_u16..300, 0_u16..300),
        (cols, rows) in (1_u16..9, 1_u16..9),
    ) {
        let area = rect(x, y, width, height);
        let fit = aspect_fit(area, cols, rows);
        prop_assert!(fit.width <= width && fit.height <= height);
        prop_assert!(fit.width == width || fit.height == height);
        let skew = (i64::from(fit.width) * i64::from(rows) - i64::from(fit.height) * i64::from(cols)).abs();
        prop_assert!(skew < i64::from(cols.max(rows)), "{}x{} is not {cols}:{rows}", fit.width, fit.height);
        let left = fit.x - x;
        let right = width - fit.width - left;
        prop_assert!(right == left || right == left + 1);
        let top = fit.y - y;
        let bottom = height - fit.height - top;
        prop_assert!(bottom == top || bottom == top + 1);
    }
}
