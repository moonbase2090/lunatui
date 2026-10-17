// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Style, diff_runs_naive, simd::collect_runs_in_row_simd};
use proptest::prelude::*;

#[test]
fn simd_row_collect_matches_naive_on_width_growth() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(1, 1);
    prev.put_str(0, 0, style, "a", &mut clusters);
    prev.clear_dirty();
    let mut next = Grid::new(2, 1);
    next.put_str(0, 0, style, "b", &mut clusters);
    next.put_str(1, 0, style, "c", &mut clusters);
    let naive = diff_runs_naive(&prev, &next);
    let simd = collect_runs_in_row_simd(&prev, &next, 0, 0, next.width());
    assert_eq!(simd, naive);
    assert_eq!(
        simd,
        vec![lunatui_core::ChangedRun {
            row: 0,
            col: 0,
            width: 2,
        }]
    );
}

#[test]
fn simd_row_collect_matches_naive_on_height_growth() {
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    let mut prev = Grid::new(5, 1);
    prev.put_str(0, 0, style, "ab", &mut clusters);
    prev.clear_dirty();
    let mut next = Grid::new(5, 2);
    next.put_str(0, 0, style, "ab", &mut clusters);
    next.put_str(0, 1, style, "xy", &mut clusters);
    let naive = diff_runs_naive(&prev, &next);
    let simd = collect_runs_in_row_simd(&prev, &next, 1, 0, next.width());
    assert_eq!(simd, naive);
}

proptest! {
    #[test]
    fn simd_row_collect_matches_naive_per_row(
        w in 4u16..20,
        h in 1u16..6,
        ops in prop::collection::vec((0u16..10, 0u16..4, 0u16..8), 1..20),
    ) {
        let mut clusters = ClusterTable::new();
        let style = Style::new();
        let mut prev = Grid::new(w, h);
        let samples = ["a", "bb", " "];
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
            let text = samples[(usize::from(*pick) + 1) % samples.len()];
            next.put_str(col, row, style, text, &mut clusters);
        }
        for row in 0..h {
            let naive_row = diff_runs_naive(&prev, &next)
                .into_iter()
                .filter(|run| run.row == row)
                .collect::<Vec<_>>();
            let simd_row = collect_runs_in_row_simd(&prev, &next, row, 0, w);
            prop_assert_eq!(simd_row, naive_row);
        }
    }
}
