// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use core::ops::Range;

use lunatui_core::Rect;
use lunatui_layout::{Axis, Constraint, Flex, GridError, GridLayout};
use proptest::prelude::*;

#[test]
fn named_screen_areas_share_tracks_and_span_columns() -> Result<(), GridError> {
    let layout = GridLayout::new(
        [
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ],
        [Constraint::Length(18), Constraint::Fill(1)],
    )
    .area("header", 0..1, 0..2)?
    .area("sidebar", 1..2, 0..1)?
    .area("body", 1..2, 1..2)?
    .area("status", 2..3, 0..2)?;
    let regions = layout.split(Rect::new(4, 2, 60, 24));
    assert_eq!(regions.get("header"), Some(Rect::new(4, 2, 60, 3)));
    assert_eq!(regions.get("sidebar"), Some(Rect::new(4, 5, 18, 20)));
    assert_eq!(regions.get("body"), Some(Rect::new(22, 5, 42, 20)));
    assert_eq!(regions.get("status"), Some(Rect::new(4, 25, 60, 1)));
    assert_eq!(regions.cell(1, 1), Some(Rect::new(22, 5, 42, 20)));
    assert_eq!(regions.span(0..2, 0..2), Some(Rect::new(4, 2, 60, 23)));
    assert_eq!(regions.get("missing"), None);
    assert!(!regions.needs_solver);
    Ok(())
}

#[test]
fn spans_include_internal_gaps_on_both_axes() -> Result<(), GridError> {
    let regions = GridLayout::new(
        [
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(4),
        ],
        [
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Length(3),
        ],
    )
    .row_gap(2)
    .column_gap(1)
    .area("group", 0..2, 1..3)?
    .split(Rect::new(10, 20, 17, 13));
    assert_eq!(regions.get("group"), Some(Rect::new(16, 20, 11, 7)));
    assert_eq!(regions.cell(2, 2), Some(Rect::new(24, 29, 3, 4)));
    assert_eq!(regions.span(0..3, 0..3), Some(Rect::new(10, 20, 17, 13)));
    Ok(())
}

#[test]
fn generated_names_and_results_are_owned_and_ordered() -> Result<(), GridError> {
    let regions = {
        let mut layout = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 2]);
        for row in 0..2 {
            layout = layout.area(format!("panel-{row}"), row..row + 1, 0..2)?;
        }
        layout.split(Rect::new(0, 0, 20, 8))
    };
    assert_eq!(regions.get("panel-1"), Some(Rect::new(0, 4, 20, 4)));
    assert_eq!(
        regions.iter().collect::<Vec<_>>(),
        [
            ("panel-0", Rect::new(0, 0, 20, 4)),
            ("panel-1", Rect::new(0, 4, 20, 4)),
        ]
    );
    Ok(())
}

#[test]
fn names_are_exact_and_differently_named_areas_may_overlap() -> Result<(), GridError> {
    let regions = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 2])
        .area("body", 0..2, 0..2)?
        .area("Body", 0..2, 0..2)?
        .area("部品", 1..2, 1..2)?
        .area(" ", 0..1, 1..2)?
        .split(Rect::new(0, 0, 20, 10));
    assert_eq!(regions.get("body"), Some(Rect::new(0, 0, 20, 10)));
    assert_eq!(regions.get("Body"), Some(Rect::new(0, 0, 20, 10)));
    assert_eq!(regions.get("部品"), Some(Rect::new(10, 5, 10, 5)));
    assert_eq!(regions.get(" "), Some(Rect::new(10, 0, 10, 5)));
    assert_eq!(regions.get("BODY"), None);
    assert_eq!(regions.get(""), None);
    Ok(())
}

#[test]
fn empty_and_duplicate_names_are_rejected_before_ranges() -> Result<(), GridError> {
    let layout = GridLayout::new([Constraint::Fill(1)], [Constraint::Fill(1)]);
    assert_eq!(
        layout.clone().area("", 0..0, 0..0),
        Err(GridError::EmptyName)
    );
    let layout = layout.area("body", 0..1, 0..1)?;
    assert_eq!(
        layout.area("body", 0..0, 0..0),
        Err(GridError::DuplicateName {
            name: String::from("body"),
        })
    );
    Ok(())
}

#[test]
fn declarations_reject_empty_reversed_and_out_of_bounds_spans() {
    for (rows, columns, axis, start, end) in [
        (0..0, 0..1, Axis::Vertical, 0, 0),
        (Range { start: 2, end: 1 }, 0..1, Axis::Vertical, 2, 1),
        (0..3, 0..1, Axis::Vertical, 0, 3),
        (0..1, 2..2, Axis::Horizontal, 2, 2),
        (0..1, 1..3, Axis::Horizontal, 1, 3),
    ] {
        let layout = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 2]);
        assert_eq!(
            layout.area("invalid", rows, columns),
            Err(GridError::InvalidSpan {
                axis,
                start,
                end,
                tracks: 2
            })
        );
    }
}

#[test]
fn invalid_queries_return_none_without_clamping_or_overflow() {
    let regions = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 2])
        .split(Rect::new(0, 0, 20, 10));
    assert_eq!(regions.cell(0, 0), Some(Rect::new(0, 0, 10, 5)));
    assert_eq!(regions.cell(2, 0), None);
    assert_eq!(regions.cell(0, 2), None);
    assert_eq!(regions.cell(usize::MAX, 0), None);
    assert_eq!(regions.cell(0, usize::MAX), None);
    assert_eq!(regions.span(0..0, 0..1), None);
    assert_eq!(regions.span(Range { start: 2, end: 1 }, 0..1), None);
    assert_eq!(regions.span(0..1, 0..3), None);
    assert_eq!(regions.span(0..usize::MAX, 0..1), None);
}

#[test]
fn empty_axes_have_no_cells_or_named_placements() {
    let empty = GridLayout::new([], []).split(Rect::new(3, 4, 20, 10));
    assert_eq!(empty.cell(0, 0), None);
    assert_eq!(empty.iter().count(), 0);
    assert!(!empty.needs_solver);
    let rows = GridLayout::new([Constraint::Fill(1)], []);
    assert_eq!(rows.split(Rect::new(0, 0, 8, 4)).cell(0, 0), None);
    assert_eq!(
        rows.area("missing", 0..1, 0..1),
        Err(GridError::InvalidSpan {
            axis: Axis::Horizontal,
            start: 0,
            end: 1,
            tracks: 0
        })
    );
    assert_eq!(
        GridLayout::new([], [Constraint::Fill(1)]).area("missing", 0..1, 0..1),
        Err(GridError::InvalidSpan {
            axis: Axis::Vertical,
            start: 0,
            end: 1,
            tracks: 0
        })
    );
}

#[test]
fn margins_are_applied_once_and_may_collapse_both_axes() {
    let regions = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 2])
        .margin(2, 1)
        .split(Rect::new(10, 20, 20, 10));
    assert_eq!(regions.cell(0, 0), Some(Rect::new(12, 21, 8, 4)));
    assert_eq!(regions.span(0..2, 0..2), Some(Rect::new(12, 21, 16, 8)));
    let collapsed = GridLayout::new([Constraint::Fill(1)], [Constraint::Fill(1)])
        .margin(8, 9)
        .split(Rect::new(3, 4, 5, 6));
    assert_eq!(collapsed.cell(0, 0), Some(Rect::new(11, 13, 0, 0)));
}

#[test]
fn zero_size_tracks_keep_their_solved_positions() {
    let layout = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 3]);
    assert_eq!(
        layout.split(Rect::new(5, 7, 0, 0)).cell(1, 2),
        Some(Rect::new(5, 7, 0, 0))
    );
    assert_eq!(
        layout.split(Rect::new(5, 7, 0, 8)).cell(1, 2),
        Some(Rect::new(5, 11, 0, 4))
    );
    assert_eq!(
        layout.split(Rect::new(5, 7, 9, 0)).cell(1, 2),
        Some(Rect::new(11, 7, 3, 0))
    );
}

#[test]
fn row_and_column_flex_pack_independently() {
    let regions = GridLayout::new(
        [Constraint::Length(2), Constraint::Length(3)],
        [Constraint::Length(4), Constraint::Length(5)],
    )
    .row_flex(Flex::End)
    .column_flex(Flex::Center)
    .split(Rect::new(10, 20, 20, 12));
    assert_eq!(regions.cell(0, 0), Some(Rect::new(15, 27, 4, 2)));
    assert_eq!(regions.span(0..2, 0..2), Some(Rect::new(15, 27, 9, 5)));
}

#[test]
fn fractional_rounding_stays_with_lower_track_indexes() {
    let regions = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 3])
        .split(Rect::new(2, 3, 17, 9));
    assert_eq!(regions.cell(0, 0), Some(Rect::new(2, 3, 6, 5)));
    assert_eq!(regions.cell(1, 2), Some(Rect::new(14, 8, 5, 4)));
    let ratios = GridLayout::new(
        [Constraint::Percentage(75), Constraint::Fill(1)],
        [Constraint::Ratio(1, 4); 4],
    )
    .split(Rect::new(0, 0, 50, 50));
    assert_eq!(ratios.cell(1, 3), Some(Rect::new(38, 38, 12, 12)));
    assert_eq!(ratios.span(0..2, 1..4), Some(Rect::new(13, 0, 37, 50)));
}

#[test]
fn minimum_conflicts_on_either_axis_propagate_the_solver_diagnostic() {
    for layout in [
        GridLayout::new([Constraint::Min(20); 2], [Constraint::Fill(1)]),
        GridLayout::new([Constraint::Fill(1)], [Constraint::Min(20); 2]),
    ] {
        let regions = layout.split(Rect::new(0, 0, 10, 10));
        assert!(regions.needs_solver);
        assert_eq!(regions.cell(0, 0).map(|rect| rect.x), Some(0));
    }
    let gaps = GridLayout::new([Constraint::Fill(1)], [Constraint::Length(1); 2])
        .column_gap(30)
        .split(Rect::new(0, 0, 10, 4));
    assert!(gaps.needs_solver);
    assert_eq!(gaps.span(0..1, 0..2), Some(Rect::new(0, 0, 10, 4)));
}

#[test]
fn negative_gaps_include_an_interior_tracks_furthest_endpoint() -> Result<(), GridError> {
    let tracks = [
        Constraint::Length(8),
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Length(1),
    ];
    let regions = GridLayout::new(tracks, tracks)
        .row_gap(-5)
        .column_gap(-5)
        .area("all", 0..4, 0..4)?
        .split(Rect::new(10, 20, 20, 20));
    assert_eq!(regions.get("all"), Some(Rect::new(10, 20, 9, 9)));
    assert_eq!(regions.cell(1, 1), Some(Rect::new(13, 23, 1, 1)));
    assert_eq!(regions.cell(2, 2), Some(Rect::new(10, 20, 9, 9)));
    assert_eq!(regions.cell(3, 3), Some(Rect::new(14, 24, 1, 1)));
    Ok(())
}

#[test]
fn saturated_origins_preserve_locally_solved_widths_and_heights() {
    let regions = GridLayout::new([Constraint::Length(4); 2], [Constraint::Length(10); 2])
        .split(Rect::new(65_530, 65_533, 20, 8));
    assert_eq!(
        regions.cell(1, 1),
        Some(Rect::new(u16::MAX, u16::MAX, 10, 4))
    );
    assert_eq!(
        regions.span(0..2, 0..2),
        Some(Rect::new(65_530, 65_533, 20, 8))
    );
    let margin = GridLayout::new([Constraint::Fill(1); 2], [Constraint::Fill(1); 2])
        .margin(1, 1)
        .split(Rect::new(65_534, 65_534, 10, 10));
    assert_eq!(
        margin.span(0..2, 0..2),
        Some(Rect::new(u16::MAX, u16::MAX, 8, 8))
    );
}

#[test]
fn repeated_resizes_do_not_change_previously_returned_geometry() -> Result<(), GridError> {
    let layout = GridLayout::new(
        [Constraint::Fill(1)],
        [Constraint::Length(4), Constraint::Fill(1)],
    )
    .area("body", 0..1, 1..2)?;
    let first = layout.split(Rect::new(0, 0, 10, 4));
    let second = layout.split(Rect::new(0, 0, 20, 6));
    assert_eq!(first.get("body"), Some(Rect::new(4, 0, 6, 4)));
    assert_eq!(second.get("body"), Some(Rect::new(4, 0, 16, 6)));
    assert_eq!(layout.split(Rect::new(0, 0, 10, 4)), first);
    Ok(())
}

proptest! {
    #[test]
    fn equal_fill_cells_cover_each_ungapped_axis(
        rows in 1_usize..6, columns in 1_usize..6,
        width in 0_u16..200, height in 0_u16..100,
        x in 0_u16..1000, y in 0_u16..1000,
    ) {
        let regions = GridLayout::new(
            vec![Constraint::Fill(1); rows], vec![Constraint::Fill(1); columns],
        ).split(Rect::new(x, y, width, height));
        prop_assert_eq!(regions.span(0..rows, 0..columns), Some(Rect::new(x, y, width, height)));
        let mut cells = 0;
        for row in 0..rows {
            let mut used_width = 0_u32;
            for column in 0..columns {
                let Some(rect) = regions.cell(row, column) else {
                    return Err(TestCaseError::fail("valid cell is missing"));
                };
                prop_assert!(rect.x >= x && u32::from(rect.x) + u32::from(rect.width) <= u32::from(x) + u32::from(width));
                prop_assert!(rect.y >= y && u32::from(rect.y) + u32::from(rect.height) <= u32::from(y) + u32::from(height));
                used_width += u32::from(rect.width);
                cells += 1;
            }
            prop_assert_eq!(used_width, u32::from(width));
        }
        prop_assert_eq!(cells, rows * columns);
        prop_assert!(!regions.needs_solver);
    }
}
