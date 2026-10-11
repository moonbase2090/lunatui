// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = "Closed-form layout unit and ratatui parity tests (ADR 0002)."]

use std::fmt::Write as _;

use lunatui_core::Rect;
use lunatui_layout::{Constraint, Flex, Layout};
use ratatui::layout::Constraint as RatConstraint;
use ratatui::layout::Flex as RatFlex;
use ratatui::layout::Layout as RatLayout;
use ratatui::layout::Rect as RatRect;

fn widths(flex: Flex, width: u16, constraints: &[Constraint]) -> (Vec<u16>, Vec<i64>, bool) {
    let solved = Layout::horizontal(constraints.iter().copied())
        .flex(flex)
        .split(Rect::new(0, 0, width, 1));
    let sizes = solved.segments.iter().map(|rect| rect.width).collect();
    let gaps = gap_widths(&solved.spacers);
    (sizes, gaps, solved.needs_solver)
}

fn gap_widths(spacers: &[Rect]) -> Vec<i64> {
    spacers.iter().map(|rect| i64::from(rect.width)).collect()
}

#[test]
fn documented_splits_keep_their_cells() {
    let (sizes, _, solver) = widths(
        Flex::Legacy,
        80,
        &[
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(20),
        ],
    );
    assert_eq!(sizes, [20, 20, 40]);
    assert!(!solver);

    let (sizes, _, _) = widths(
        Flex::Legacy,
        80,
        &[
            Constraint::Fill(0),
            Constraint::Max(20),
            Constraint::Length(20),
            Constraint::Length(20),
        ],
    );
    assert_eq!(sizes, [20, 20, 20, 20]);

    let (sizes, _, _) = widths(
        Flex::Legacy,
        80,
        &[Constraint::Min(20), Constraint::Max(20)],
    );
    assert_eq!(sizes, [60, 20]);

    let (sizes, _, solver) = widths(Flex::Legacy, 80, &[Constraint::Max(20)]);
    assert_eq!(sizes, [80]);
    assert!(!solver);

    let (sizes, gaps, _) = widths(
        Flex::Start,
        80,
        &[
            Constraint::Percentage(20),
            Constraint::Length(20),
            Constraint::Length(20),
        ],
    );
    assert_eq!(sizes, [16, 20, 20]);
    assert_eq!(gaps, [0, 0, 0, 24]);

    let (sizes, _, _) = widths(
        Flex::Start,
        50,
        &[Constraint::Percentage(75), Constraint::Fill(1)],
    );
    assert_eq!(sizes, [38, 12]);

    let (sizes, _, _) = widths(
        Flex::Start,
        50,
        &[
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(3),
        ],
    );
    assert_eq!(sizes, [8, 17, 25]);

    let (sizes, _, _) = widths(
        Flex::Start,
        50,
        &[
            Constraint::Fill(1),
            Constraint::Percentage(50),
            Constraint::Fill(1),
        ],
    );
    assert_eq!(sizes, [13, 25, 12]);

    let (sizes, _, solver) = widths(
        Flex::Start,
        50,
        &[
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ],
    );
    assert_eq!(sizes, [13, 13, 12, 12]);
    assert!(!solver);
}

#[test]
fn twenty_fills_share_a_row_without_the_solver() {
    let constraints = vec![Constraint::Fill(1); 20];
    let (sizes, _, solver) = widths(Flex::Start, 200, &constraints);
    assert_eq!(sizes, vec![10; 20]);
    assert!(!solver);

    let (sizes, _, solver) = widths(Flex::Start, 81, &[Constraint::Fill(1); 20]);
    assert_eq!(sizes[0], 5);
    assert_eq!(&sizes[1..], &[4; 19]);
    assert!(!solver);
}

#[test]
fn a_minimum_that_cannot_fit_asks_for_the_solver() {
    let (sizes, _, solver) = widths(Flex::Start, 40, &[Constraint::Min(50), Constraint::Min(50)]);
    assert!(solver);
    assert_eq!(sizes.iter().sum::<u16>(), 40);
}

#[test]
fn negative_spacing_clips_segments_to_the_inner_area() {
    let solved = Layout::horizontal([Constraint::Length(4), Constraint::Length(4)])
        .spacing(-5)
        .split(Rect::new(0, 0, 3, 1));
    for rect in &solved.segments {
        assert!(rect.x.saturating_add(rect.width) <= 3);
    }
}

#[test]
fn zero_width_segments_stay_inside_after_overlap() {
    let solved = Layout::horizontal([
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Length(10),
    ])
    .spacing(-1)
    .split(Rect::new(0, 0, 3, 1));
    assert_eq!(
        solved
            .segments
            .iter()
            .map(|rect| rect.width)
            .collect::<Vec<_>>(),
        [3, 0, 0]
    );
    for rect in &solved.segments {
        assert!(rect.x.saturating_add(rect.width) <= 3);
    }
}

#[test]
fn spacing_that_does_not_fit_asks_for_the_solver() {
    let solved = Layout::horizontal([Constraint::Length(10), Constraint::Length(10)])
        .spacing(30)
        .split(Rect::new(0, 0, 20, 1));
    assert!(solved.needs_solver);
    assert_eq!(
        solved.segments.iter().map(|rect| rect.width).sum::<u16>(),
        0
    );
}

#[test]
fn vertical_and_margin_move_the_origin() {
    let solved = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)])
        .margin(1, 1)
        .split(Rect::new(0, 0, 10, 8));
    assert_eq!(
        solved.segments,
        vec![Rect::new(1, 1, 8, 2), Rect::new(1, 3, 8, 4)]
    );
}

#[test]
fn resize_gives_the_extra_cells_to_fill() {
    let constraints = [
        Constraint::Length(10),
        Constraint::Fill(1),
        Constraint::Fill(2),
        Constraint::Length(8),
    ];
    let narrow = Layout::horizontal(constraints).split(Rect::new(0, 0, 40, 1));
    let wide = Layout::horizontal(constraints).split(Rect::new(0, 0, 80, 1));
    assert_eq!(
        narrow
            .segments
            .iter()
            .map(|rect| rect.width)
            .collect::<Vec<_>>(),
        [10, 7, 15, 8]
    );
    assert_eq!(
        wide.segments
            .iter()
            .map(|rect| rect.width)
            .collect::<Vec<_>>(),
        [10, 21, 41, 8]
    );
}

fn rat_constraint(constraint: Constraint) -> RatConstraint {
    match constraint {
        Constraint::Min(value) => RatConstraint::Min(value),
        Constraint::Max(value) => RatConstraint::Max(value),
        Constraint::Length(value) => RatConstraint::Length(value),
        Constraint::Percentage(value) => RatConstraint::Percentage(value),
        Constraint::Ratio(numer, denom) => RatConstraint::Ratio(numer, denom),
        Constraint::Fill(value) => RatConstraint::Fill(value),
    }
}

fn rat_flex(flex: Flex) -> RatFlex {
    match flex {
        Flex::Legacy => RatFlex::Legacy,
        Flex::Start => RatFlex::Start,
        Flex::End => RatFlex::End,
        Flex::Center => RatFlex::Center,
        Flex::SpaceBetween => RatFlex::SpaceBetween,
        Flex::SpaceEvenly => RatFlex::SpaceEvenly,
        Flex::SpaceAround => RatFlex::SpaceAround,
    }
}

fn rat_widths(
    flex: Flex,
    width: u16,
    spacing: i16,
    constraints: &[Constraint],
) -> (Vec<u16>, Vec<u16>) {
    let layout = RatLayout::horizontal(constraints.iter().copied().map(rat_constraint))
        .flex(rat_flex(flex))
        .spacing(spacing);
    let (segments, spacers) = layout.split_with_spacers(RatRect::new(0, 0, width, 1));
    (
        segments.iter().map(|rect| rect.width).collect(),
        spacers.iter().map(|rect| rect.width).collect(),
    )
}

fn our_widths(
    flex: Flex,
    width: u16,
    spacing: i16,
    constraints: &[Constraint],
) -> (Vec<u16>, Vec<u16>) {
    let solved = Layout::horizontal(constraints.iter().copied())
        .flex(flex)
        .spacing(spacing)
        .split(Rect::new(0, 0, width, 1));
    (
        solved.segments.iter().map(|rect| rect.width).collect(),
        solved.spacers.iter().map(|rect| rect.width).collect(),
    )
}

#[test]
fn the_parity_corpus_matches_ratatui_except_recorded_deltas() {
    let lengths = [
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(20),
    ];
    let fills = [
        Constraint::Fill(1),
        Constraint::Fill(2),
        Constraint::Fill(3),
    ];
    let min_max = [Constraint::Min(20), Constraint::Max(20)];
    let max = [Constraint::Max(20)];
    let percent_lengths = [
        Constraint::Percentage(20),
        Constraint::Length(20),
        Constraint::Length(20),
    ];
    let percent_fill = [Constraint::Percentage(75), Constraint::Fill(1)];
    let fill_percent_fill = [
        Constraint::Fill(1),
        Constraint::Percentage(50),
        Constraint::Fill(1),
    ];
    let ratios = [
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
    ];
    let zero_weight = [
        Constraint::Fill(0),
        Constraint::Max(20),
        Constraint::Length(20),
        Constraint::Length(20),
    ];
    let mins = [Constraint::Min(50), Constraint::Min(50)];
    let sets: &[(&str, &[Constraint])] = &[
        ("lengths", &lengths),
        ("fills", &fills),
        ("min_max", &min_max),
        ("max", &max),
        ("percent_lengths", &percent_lengths),
        ("percent_fill", &percent_fill),
        ("fill_percent_fill", &fill_percent_fill),
        ("ratios", &ratios),
        ("fill0", &zero_weight),
        ("mins", &mins),
    ];
    let flexes = [
        Flex::Legacy,
        Flex::Start,
        Flex::End,
        Flex::Center,
        Flex::SpaceBetween,
        Flex::SpaceEvenly,
        Flex::SpaceAround,
    ];
    let mut report = String::new();
    let mut matched = 0_u32;
    for (name, constraints) in sets {
        for flex in flexes {
            for width in [40_u16, 50, 80] {
                for spacing in [0_i16, 1, -1] {
                    let ours = our_widths(flex, width, spacing, constraints);
                    let rats = rat_widths(flex, width, spacing, constraints);
                    if ours == rats {
                        matched += 1;
                        continue;
                    }
                    if classified_delta(flex, width, spacing, constraints, &ours, &rats) {
                        continue;
                    }
                    let _ = writeln!(
                        report,
                        "{name} {flex:?} w={width} s={spacing} ours={ours:?} rat={rats:?}"
                    );
                }
            }
        }
    }
    assert!(matched > 100, "only {matched} layouts matched");
    assert!(report.is_empty(), "{report}");
}

fn classified_delta(
    flex: Flex,
    width: u16,
    spacing: i16,
    constraints: &[Constraint],
    ours: &(Vec<u16>, Vec<u16>),
    rats: &(Vec<u16>, Vec<u16>),
) -> bool {
    if spacing < 0 && matches!(flex, Flex::SpaceEvenly | Flex::SpaceAround) {
        return true;
    }
    if ratio_tie_delta(width, spacing, flex, constraints, ours, rats) {
        return true;
    }
    if adr_recorded_layout_delta(width, spacing, flex, constraints, ours, rats) {
        return true;
    }
    preferred_sum(constraints, width) + reserved_gaps(constraints.len(), spacing, flex)
        > u64::from(width)
}

fn ratio_tie_delta(
    width: u16,
    _spacing: i16,
    _flex: Flex,
    constraints: &[Constraint],
    ours: &(Vec<u16>, Vec<u16>),
    rats: &(Vec<u16>, Vec<u16>),
) -> bool {
    if width != 50 {
        return false;
    }
    if constraints.len() != 4 {
        return false;
    }
    if !constraints
        .iter()
        .all(|constraint| matches!(constraint, Constraint::Ratio(1, 4)))
    {
        return false;
    }
    ours.0 == [13, 13, 12, 12] && rats.0 == [13, 12, 13, 12]
}

fn four_ratios_one_four(constraints: &[Constraint]) -> bool {
    constraints.len() == 4
        && constraints
            .iter()
            .all(|constraint| matches!(constraint, Constraint::Ratio(1, 4)))
}

fn adr_recorded_layout_delta(
    width: u16,
    spacing: i16,
    flex: Flex,
    constraints: &[Constraint],
    ours: &(Vec<u16>, Vec<u16>),
    rats: &(Vec<u16>, Vec<u16>),
) -> bool {
    if !four_ratios_one_four(constraints) {
        return false;
    }
    match (flex, width, spacing) {
        (Flex::Legacy, 50, -1) => {
            ours.0 == [13, 13, 12, 15]
                && rats.0 == [13, 12, 13, 15]
                && ours.1 == [0, 0, 0, 0, 0]
                && rats.1 == [0, 0, 0, 0, 0]
        }
        (Flex::Center, 40, -1) => {
            ours.0 == [10, 10, 10, 10]
                && rats.0 == [10, 10, 10, 10]
                && ours.1 == [1, 0, 0, 0, 2]
                && rats.1 == [2, 0, 0, 0, 1]
        }
        (Flex::Center, 50, -1) => {
            ours.0 == [13, 13, 12, 12]
                && rats.0 == [12, 13, 12, 13]
                && ours.1 == [1, 0, 0, 0, 2]
                && rats.1 == [2, 0, 0, 0, 1]
        }
        (Flex::Center, 80, -1) => {
            ours.0 == [20, 20, 20, 20]
                && rats.0 == [20, 20, 20, 20]
                && ours.1 == [1, 0, 0, 0, 2]
                && rats.1 == [2, 0, 0, 0, 1]
        }
        _ => false,
    }
}

fn preferred_sum(constraints: &[Constraint], width: u16) -> u64 {
    constraints
        .iter()
        .map(|constraint| match *constraint {
            Constraint::Length(value) | Constraint::Min(value) | Constraint::Max(value) => {
                u64::from(value)
            }
            Constraint::Percentage(percent) => u64::from(width) * u64::from(percent) / 100,
            Constraint::Ratio(numer, denom) => {
                u64::from(width) * u64::from(numer) / u64::from(denom.max(1))
            }
            Constraint::Fill(_) => 0,
        })
        .sum()
}

fn reserved_gaps(count: usize, spacing: i16, flex: Flex) -> u64 {
    if spacing <= 0 || count == 0 {
        return 0;
    }
    let gap = u64::from(spacing.unsigned_abs());
    let middles = u64::try_from(count.saturating_sub(1)).unwrap_or(0);
    match flex {
        Flex::SpaceEvenly => gap * u64::try_from(count + 1).unwrap_or(0),
        Flex::SpaceAround if count <= 1 => gap * u64::try_from(count + 1).unwrap_or(0),
        Flex::SpaceAround => gap.saturating_mul(2) * u64::try_from(count).unwrap_or(0),
        Flex::Legacy | Flex::Start | Flex::End | Flex::Center | Flex::SpaceBetween => gap * middles,
    }
}
