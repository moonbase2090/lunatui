// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = "Property tests for closed-form layout invariants."]

use lunatui_core::Rect;
use lunatui_layout::{Constraint, Flex, Layout};
use proptest::prelude::*;

fn axis_use(width: u16, constraints: &[Constraint], flex: Flex, spacing: i16) -> (u16, bool) {
    let solved = Layout::horizontal(constraints.iter().copied())
        .flex(flex)
        .spacing(spacing)
        .split(Rect::new(0, 0, width, 1));
    let segments = solved.segments.iter().map(|rect| rect.width).sum::<u16>();
    let gaps = solved
        .spacers
        .iter()
        .map(|rect| rect.width)
        .map(i64::from)
        .sum::<i64>();
    let total = i64::from(segments) + gaps;
    (
        u16::try_from(total.max(0)).unwrap_or(u16::MAX),
        solved.needs_solver,
    )
}

proptest! {
    #[test]
    fn length_splits_never_exceed_axis(
        width in 1_u16..120,
        spacing in 0_i16..3,
        flex in prop_oneof![Just(Flex::Start), Just(Flex::Legacy)],
        lengths in prop::collection::vec(1_u16..30, 1..6),
    ) {
        let constraints: Vec<Constraint> = lengths
            .into_iter()
            .map(Constraint::Length)
            .collect();
        let (used, needs_solver) = axis_use(width, &constraints, flex, spacing);
        if needs_solver {
            return Ok(());
        }
        prop_assert!(used <= width);
    }

    #[test]
    fn fill_rows_sum_to_width(
        width in 20_u16..200,
        count in 2_usize..12,
    ) {
        let constraints = vec![Constraint::Fill(1); count];
        let (used, needs_solver) = axis_use(width, &constraints, Flex::Start, 0);
        prop_assert!(!needs_solver);
        prop_assert_eq!(used, width);
    }
}
