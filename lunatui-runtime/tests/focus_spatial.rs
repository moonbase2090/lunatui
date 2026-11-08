// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{FocusDirection, FocusManager, HitId};

fn manager(entries: &[(u32, Rect)], focused: u32) -> FocusManager {
    let mut focus = FocusManager::new();
    for (id, area) in entries {
        focus.register(HitId(*id), *area);
    }
    focus.focus(HitId(focused));
    focus
}

#[test]
fn right_prefers_the_aligned_neighbor_over_a_closer_diagonal() {
    let mut focus = manager(
        &[
            (1, Rect::new(0, 0, 10, 3)),
            (2, Rect::new(11, 10, 2, 2)),
            (3, Rect::new(20, 0, 20, 3)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(3)));
}

#[test]
fn down_prefers_the_aligned_neighbor_over_a_closer_diagonal() {
    let mut focus = manager(
        &[
            (1, Rect::new(0, 0, 3, 10)),
            (2, Rect::new(10, 11, 2, 2)),
            (3, Rect::new(0, 20, 3, 20)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Down);
    assert_eq!(focus.focused(), Some(HitId(3)));
}

#[test]
fn arrow_never_moves_into_a_rect_that_overlaps_the_current_one() {
    let mut focus = manager(
        &[
            (1, Rect::new(10, 10, 4, 2)),
            (2, Rect::new(0, 0, 40, 20)),
            (3, Rect::new(20, 10, 4, 2)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(3)));
}

#[test]
fn arrow_stays_put_when_every_candidate_overlaps() {
    let mut focus = manager(
        &[(1, Rect::new(10, 10, 4, 2)), (2, Rect::new(0, 0, 40, 20))],
        1,
    );
    for direction in [
        FocusDirection::Left,
        FocusDirection::Right,
        FocusDirection::Up,
        FocusDirection::Down,
    ] {
        focus.arrow(direction);
        assert_eq!(focus.focused(), Some(HitId(1)), "{direction:?}");
    }
}

#[test]
fn aligned_candidates_pick_the_smallest_edge_gap() {
    let mut focus = manager(
        &[
            (1, Rect::new(0, 0, 4, 2)),
            (2, Rect::new(20, 0, 4, 2)),
            (3, Rect::new(8, 0, 4, 2)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(3)));
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(2)));
    focus.arrow(FocusDirection::Left);
    assert_eq!(focus.focused(), Some(HitId(3)));
}

#[test]
fn without_an_aligned_candidate_the_nearest_rect_in_direction_wins() {
    let mut focus = manager(
        &[
            (1, Rect::new(0, 0, 4, 2)),
            (2, Rect::new(30, 10, 4, 2)),
            (3, Rect::new(8, 6, 4, 2)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(3)));
}

#[test]
fn adjacent_panes_with_no_gap_are_reachable() {
    let mut focus = manager(
        &[
            (1, Rect::new(0, 0, 10, 5)),
            (2, Rect::new(10, 0, 10, 5)),
            (3, Rect::new(0, 5, 10, 5)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(2)));
    focus.arrow(FocusDirection::Left);
    assert_eq!(focus.focused(), Some(HitId(1)));
    focus.arrow(FocusDirection::Down);
    assert_eq!(focus.focused(), Some(HitId(3)));
    focus.arrow(FocusDirection::Up);
    assert_eq!(focus.focused(), Some(HitId(1)));
}

#[test]
fn a_tie_keeps_the_first_registered_candidate() {
    let mut focus = manager(
        &[
            (1, Rect::new(0, 0, 4, 2)),
            (2, Rect::new(8, 0, 4, 2)),
            (3, Rect::new(8, 0, 4, 2)),
        ],
        1,
    );
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(2)));
}

#[test]
fn arrow_with_no_focus_starts_at_the_first_entry() {
    let mut focus = FocusManager::new();
    focus.register(HitId(7), Rect::new(0, 0, 2, 2));
    focus.register(HitId(8), Rect::new(5, 0, 2, 2));
    focus.arrow(FocusDirection::Right);
    assert_eq!(focus.focused(), Some(HitId(7)));
}
