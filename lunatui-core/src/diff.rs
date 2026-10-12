// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::vec::Vec;

use crate::grid::Grid;

/// A contiguous run of changed cells on one row (PRD §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChangedRun {
    /// Row index.
    pub row: u16,
    /// Left column (inclusive).
    pub col: u16,
    /// Width in columns.
    pub width: u16,
}

/// Collects changed runs between `previous` and `next`, visiting only dirty rows and their
/// dirty spans on `next` when dimensions match (PRD §2.3).
#[must_use]
pub fn diff_runs(previous: &Grid, next: &Grid) -> Vec<ChangedRun> {
    if previous.width() != next.width() || previous.height() != next.height() {
        return diff_runs_naive(previous, next);
    }
    let mut runs = Vec::new();
    for row in 0..next.height() {
        let Some(dirty) = next.row_dirty(row) else {
            continue;
        };
        if !dirty.dirty {
            continue;
        }
        collect_runs_in_row(previous, next, row, dirty.min_x, dirty.max_x, &mut runs);
    }
    runs
}

/// Full-grid diff used for tests and for size changes (full invalidation).
#[must_use]
pub fn diff_runs_naive(previous: &Grid, next: &Grid) -> Vec<ChangedRun> {
    if previous.width() != next.width() || previous.height() != next.height() {
        return diff_runs_different_size(previous, next);
    }
    let mut runs = Vec::new();
    for row in 0..next.height() {
        collect_runs_in_row(previous, next, row, 0, next.width(), &mut runs);
    }
    runs
}

fn diff_runs_different_size(previous: &Grid, next: &Grid) -> Vec<ChangedRun> {
    let mut runs = Vec::new();
    for row in 0..next.height() {
        collect_runs_in_row(previous, next, row, 0, next.width(), &mut runs);
    }
    runs
}

fn collect_runs_in_row(
    previous: &Grid,
    next: &Grid,
    row: u16,
    start: u16,
    end: u16,
    runs: &mut Vec<ChangedRun>,
) {
    let mut col = start;
    while col < end {
        let prev_bits = previous.cell_bits(col, row);
        let next_bits = next.cell_bits(col, row);
        if prev_bits == next_bits {
            col += 1;
            continue;
        }
        let run_start = col;
        col += 1;
        while col < end && previous.cell_bits(col, row) != next.cell_bits(col, row) {
            col += 1;
        }
        runs.push(ChangedRun {
            row,
            col: run_start,
            width: col - run_start,
        });
    }
}
