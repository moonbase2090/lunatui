// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::vec::Vec;

use crate::grid::Grid;
use crate::simd;

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
    let mut runs = Vec::new();
    diff_runs_into(previous, next, &mut runs);
    runs
}

/// Like [`diff_runs`], but replaces the contents of `runs`, so one buffer serves every frame.
pub fn diff_runs_into(previous: &Grid, next: &Grid, runs: &mut Vec<ChangedRun>) {
    runs.clear();
    if previous.width() != next.width() || previous.height() != next.height() {
        runs.extend(diff_runs_naive(previous, next));
        return;
    }
    for row in 0..next.height() {
        let Some(dirty) = next.row_dirty(row) else {
            continue;
        };
        if !dirty.dirty {
            continue;
        }
        simd::collect_runs_in_row_simd_into(previous, next, row, dirty.min_x, dirty.max_x, runs);
    }
}

/// Full-grid diff used for tests and for size changes (full invalidation).
#[must_use]
pub fn diff_runs_naive(previous: &Grid, next: &Grid) -> Vec<ChangedRun> {
    if previous.width() != next.width() || previous.height() != next.height() {
        return diff_runs_different_size(previous, next);
    }
    let mut runs = Vec::new();
    for row in 0..next.height() {
        runs.extend(simd::collect_runs_in_row_scalar(
            previous,
            next,
            row,
            0,
            next.width(),
        ));
    }
    runs
}

fn diff_runs_different_size(previous: &Grid, next: &Grid) -> Vec<ChangedRun> {
    let mut runs = Vec::new();
    for row in 0..next.height() {
        runs.extend(simd::collect_runs_in_row_scalar(
            previous,
            next,
            row,
            0,
            next.width(),
        ));
    }
    runs
}
