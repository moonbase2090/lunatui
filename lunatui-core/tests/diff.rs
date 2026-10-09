// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use lunatui_core::{
    ChangedRun, ClusterTable, Color, Grid, Rect, Style, diff_runs, diff_runs_naive,
};
use proptest::prelude::*;

#[test]
fn identical_grids_produce_no_runs() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(8, 3);
    grid.put_str(1, 1, Style::new(), "hi", &mut clusters);
    grid.clear_dirty();
    assert_eq!(diff_runs(&grid, &grid), []);
    assert_eq!(diff_runs_naive(&grid, &grid), []);
}

#[test]
fn single_cell_change_is_one_run() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(6, 2);
    prev.put_str(0, 0, style, "aaaa", &mut clusters);
    prev.clear_dirty();

    let mut next = prev.clone();
    next.put_str(2, 0, style, "X", &mut clusters);
    let runs = diff_runs(&prev, &next);
    assert_eq!(
        runs,
        vec![ChangedRun {
            row: 0,
            col: 2,
            width: 1,
        }]
    );
}

#[test]
fn contiguous_changes_merge_into_one_run() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(8, 1);
    prev.put_str(0, 0, style, "........", &mut clusters);
    prev.clear_dirty();

    let mut next = prev.clone();
    next.put_str(2, 0, style, "###", &mut clusters);
    assert_eq!(
        diff_runs(&prev, &next),
        vec![ChangedRun {
            row: 0,
            col: 2,
            width: 3,
        }]
    );
}

#[test]
fn separated_changes_are_two_runs() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(10, 1);
    prev.put_str(0, 0, style, "..........", &mut clusters);
    prev.clear_dirty();

    let mut next = prev.clone();
    next.put_str(1, 0, style, "A", &mut clusters);
    next.put_str(7, 0, style, "B", &mut clusters);
    assert_eq!(
        diff_runs(&prev, &next),
        vec![
            ChangedRun {
                row: 0,
                col: 1,
                width: 1,
            },
            ChangedRun {
                row: 0,
                col: 7,
                width: 1,
            },
        ]
    );
}

#[test]
fn resize_falls_back_to_full_grid_diff() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(4, 2);
    prev.put_str(0, 0, style, "ab", &mut clusters);
    prev.clear_dirty();

    let mut next = Grid::new(5, 2);
    next.put_str(0, 0, style, "ab", &mut clusters);
    next.put_str(0, 1, style, "cd", &mut clusters);
    let runs = diff_runs(&prev, &next);
    assert_ne!(runs, []);
    assert_eq!(diff_runs(&prev, &next), diff_runs_naive(&prev, &next));
}

#[test]
fn style_only_change_produces_a_run() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(4, 1);
    prev.put_str(0, 0, style, "ab", &mut clusters);
    prev.clear_dirty();

    let mut next = prev.clone();
    next.set_style(
        Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        },
        Style::new().fg(Color::Indexed(9)),
    );
    assert_eq!(
        diff_runs(&prev, &next),
        vec![ChangedRun {
            row: 0,
            col: 1,
            width: 1,
        }]
    );
}

proptest! {
    #[test]
    fn dirty_span_diff_matches_naive_full_scan(
        w in 4u16..24,
        h in 2u16..8,
        ops in prop::collection::vec((0u16..20, 0u16..6, 0u16..10), 1..30),
    ) {
        let mut clusters = ClusterTable::new();
        let style = Style::new();
        let mut prev = Grid::new(w, h);
        let samples = ["a", "bb", "日", " ", "x"];
        for (col, row, pick) in &ops {
            let row = *row % h;
            let col = *col % w;
            let text = samples[usize::from(*pick) % samples.len()];
            prev.put_str(col, row, style, text, &mut clusters);
        }
        prev.clear_dirty();

        let mut next = prev.clone();
        for (col, row, pick) in ops.iter().take(ops.len() / 2 + 1) {
            let row = *row % h;
            let col = *col % w;
            let text = samples[usize::from(*pick + 1) % samples.len()];
            next.put_str(col, row, style, text, &mut clusters);
            if pick % 4 == 0 {
                next.set_style(
                    Rect { x: col, y: row, width: 1, height: 1 },
                    Style::new().fg(Color::Indexed((*pick % 15) as u8)),
                );
            }
        }

        let fast = diff_runs(&prev, &next);
        let naive = diff_runs_naive(&prev, &next);
        prop_assert_eq!(fast, naive);
    }
}
