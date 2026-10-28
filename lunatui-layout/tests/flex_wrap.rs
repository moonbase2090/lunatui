// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_layout::{Constraint, Flex, Layout, WrappedLine};
use proptest::prelude::*;

fn rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
    Rect::new(x, y, width, height)
}

#[test]
fn segments_that_do_not_fit_move_to_the_next_line() {
    let wrapped = Layout::horizontal([Constraint::Length(20); 4])
        .gap(2)
        .split_wrapped(rect(0, 0, 50, 20), Constraint::Length(3));
    assert_eq!(
        wrapped.segments,
        [
            rect(0, 0, 20, 3),
            rect(22, 0, 20, 3),
            rect(0, 5, 20, 3),
            rect(22, 5, 20, 3),
        ]
    );
    assert_eq!(
        wrapped.lines,
        [
            WrappedLine {
                area: rect(0, 0, 50, 3),
                segments: 0..2,
            },
            WrappedLine {
                area: rect(0, 5, 50, 3),
                segments: 2..4,
            },
        ]
    );
    assert!(!wrapped.needs_solver);
}

#[test]
fn a_wide_area_keeps_every_segment_on_one_line() {
    let wrapped = Layout::horizontal([Constraint::Length(20); 4])
        .gap(2)
        .split_wrapped(rect(0, 0, 100, 20), Constraint::Length(3));
    assert_eq!(
        wrapped.segments,
        [
            rect(0, 0, 20, 3),
            rect(22, 0, 20, 3),
            rect(44, 0, 20, 3),
            rect(66, 0, 20, 3),
        ]
    );
    assert_eq!(wrapped.lines.len(), 1);
    assert_eq!(wrapped.lines[0].area, rect(0, 0, 100, 3));
}

#[test]
fn a_segment_longer_than_the_axis_gets_its_own_clipped_line() {
    let wrapped = Layout::horizontal([Constraint::Length(60), Constraint::Length(10)])
        .split_wrapped(rect(0, 0, 50, 4), Constraint::Length(1));
    assert_eq!(wrapped.segments, [rect(0, 0, 50, 1), rect(0, 1, 10, 1)]);
    assert!(!wrapped.needs_solver);
}

#[test]
fn fill_segments_share_the_leftover_of_their_own_line() {
    let wrapped = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Fill(1),
        Constraint::Length(20),
        Constraint::Fill(1),
    ])
    .split_wrapped(rect(0, 0, 30, 2), Constraint::Length(1));
    assert_eq!(
        wrapped.segments,
        [
            rect(0, 0, 20, 1),
            rect(20, 0, 10, 1),
            rect(0, 1, 20, 1),
            rect(20, 1, 10, 1),
        ]
    );
}

#[test]
fn a_fill_after_a_full_line_starts_the_next_line() {
    let wrapped = Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)])
        .split_wrapped(rect(0, 0, 30, 2), Constraint::Length(1));
    assert_eq!(wrapped.segments, [rect(0, 0, 30, 1), rect(0, 1, 30, 1)]);
}

#[test]
fn a_fill_line_constraint_shares_the_cross_axis_between_lines() {
    let wrapped = Layout::horizontal([Constraint::Length(30); 2])
        .gap(1)
        .split_wrapped(rect(0, 0, 40, 11), Constraint::Fill(1));
    assert_eq!(wrapped.segments, [rect(0, 0, 30, 5), rect(0, 6, 30, 5)]);
    assert_eq!(wrapped.lines[1].area, rect(0, 6, 40, 5));
}

#[test]
fn a_vertical_layout_wraps_into_columns() {
    let wrapped = Layout::vertical([Constraint::Length(4); 3])
        .gap(1)
        .split_wrapped(rect(0, 0, 10, 9), Constraint::Length(4));
    assert_eq!(
        wrapped.segments,
        [rect(0, 0, 4, 4), rect(0, 5, 4, 4), rect(5, 0, 4, 4)]
    );
    assert_eq!(wrapped.lines[1].area, rect(5, 0, 4, 9));
}

#[test]
fn flex_places_each_line_on_its_own() {
    let wrapped = Layout::horizontal([Constraint::Length(10); 3])
        .flex(Flex::Center)
        .split_wrapped(rect(0, 0, 25, 2), Constraint::Length(1));
    assert_eq!(
        wrapped.segments,
        [rect(2, 0, 10, 1), rect(12, 0, 10, 1), rect(7, 1, 10, 1)]
    );
}

#[test]
fn lines_that_do_not_fit_the_cross_axis_are_clipped_from_the_end() {
    let wrapped = Layout::horizontal([Constraint::Length(30); 3])
        .gap(1)
        .split_wrapped(rect(0, 0, 40, 5), Constraint::Length(2));
    assert_eq!(
        wrapped.segments,
        [rect(0, 0, 30, 2), rect(0, 3, 30, 1), rect(0, 5, 30, 0)]
    );
    assert!(!wrapped.needs_solver);
}

#[test]
fn margins_apply_before_wrapping() {
    let wrapped = Layout::horizontal([Constraint::Length(10); 2])
        .margin(2, 1)
        .split_wrapped(rect(0, 0, 20, 10), Constraint::Length(3));
    assert_eq!(wrapped.segments, [rect(2, 1, 10, 3), rect(2, 4, 10, 3)]);
}

#[test]
fn percentages_wrap_by_their_share_of_the_full_axis() {
    let wrapped = Layout::horizontal([Constraint::Percentage(50); 3])
        .split_wrapped(rect(0, 0, 40, 2), Constraint::Length(1));
    assert_eq!(
        wrapped.segments,
        [rect(0, 0, 20, 1), rect(20, 0, 20, 1), rect(0, 1, 20, 1)]
    );
}

#[test]
fn a_cut_minimum_sets_needs_solver() {
    let wrapped = Layout::horizontal([Constraint::Min(30)])
        .split_wrapped(rect(0, 0, 20, 1), Constraint::Length(1));
    assert_eq!(wrapped.segments, [rect(0, 0, 20, 1)]);
    assert!(wrapped.needs_solver);
}

#[test]
fn no_constraints_give_no_segments_and_no_lines() {
    let wrapped = Layout::horizontal([]).split_wrapped(rect(0, 0, 20, 5), Constraint::Length(1));
    assert_eq!(wrapped.segments, []);
    assert_eq!(wrapped.lines, []);
    assert!(!wrapped.needs_solver);
}

#[test]
fn gap_sets_the_spacing_of_a_plain_split() {
    let solved = Layout::horizontal([Constraint::Length(10); 2])
        .gap(3)
        .split(rect(0, 0, 30, 1));
    assert_eq!(solved.segments, [rect(0, 0, 10, 1), rect(13, 0, 10, 1)]);
    let gaps: Vec<u16> = solved.spacers.iter().map(|spacer| spacer.width).collect();
    assert_eq!(gaps, [0, 3, 7]);
}

proptest! {
    #[test]
    fn wrapped_lengths_fill_each_line_greedily_without_overlap(
        width in 10_u16..120,
        gap in 0_u16..3,
        lengths in prop::collection::vec(1_u16..30, 1..12),
    ) {
        let constraints: Vec<Constraint> = lengths.iter().copied().map(Constraint::Length).collect();
        let area = rect(0, 0, width, 200);
        let wrapped = Layout::horizontal(constraints)
            .gap(gap)
            .split_wrapped(area, Constraint::Length(1));
        prop_assert_eq!(wrapped.segments.len(), lengths.len());
        let mut next = 0;
        for (index, line) in wrapped.lines.iter().enumerate() {
            prop_assert_eq!(line.segments.start, next);
            prop_assert!(line.segments.end > line.segments.start);
            next = line.segments.end;
            let used: u16 = lengths[line.segments.clone()].iter().sum::<u16>()
                + gap * u16::try_from(line.segments.len() - 1).unwrap_or(0);
            if line.segments.len() > 1 {
                prop_assert!(used <= width, "line {index} holds {used} cells in {width}");
            }
            if let Some(&first_of_next) = lengths.get(line.segments.end) {
                prop_assert!(
                    used + gap + first_of_next > width,
                    "line {index} could also have held the next segment"
                );
            }
            let row = &wrapped.segments[line.segments.clone()];
            for pair in row.windows(2) {
                prop_assert!(pair[0].x + pair[0].width + gap <= pair[1].x);
            }
            for segment in row {
                prop_assert!(segment.x + segment.width <= width);
                prop_assert_eq!(segment.y, line.area.y);
            }
        }
        prop_assert_eq!(next, lengths.len());
    }
}
