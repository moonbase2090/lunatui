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

/// Like [`diff_runs_into`], but leaves out cells whose only change is an alpha or sub-cell
/// attribute.
///
/// Those attributes never reach ANSI output, so an ANSI encoder that repainted such a cell
/// would write the bytes it wrote before. A run is cut around them. Grids without attributes
/// take the same path as [`diff_runs_into`]. A size change keeps every cell.
pub fn diff_runs_visible_into(previous: &Grid, next: &Grid, runs: &mut Vec<ChangedRun>) {
    diff_runs_into(previous, next, runs);
    #[cfg(feature = "native-attrs")]
    if previous.width() == next.width()
        && previous.height() == next.height()
        && !(previous.ext_table().is_empty() && next.ext_table().is_empty())
    {
        drop_attribute_only(previous, next, runs);
    }
}

#[cfg(feature = "native-attrs")]
fn drop_attribute_only(previous: &Grid, next: &Grid, runs: &mut Vec<ChangedRun>) {
    let original = runs.len();
    for at in 0..original {
        let run = runs[at];
        let before = previous.row_bits(run.row);
        let after = next.row_bits(run.row);
        let (start, end) = (usize::from(run.col), usize::from(run.col + run.width));
        let mut open: Option<usize> = None;
        for column in start..=end {
            let visible = column < end
                && match (before.get(column), after.get(column)) {
                    (Some(old), Some(new)) => (old ^ new) & !crate::grid::EXT_BITS != 0,
                    _ => true,
                };
            match (visible, open) {
                (true, None) => open = Some(column),
                (false, Some(from)) => {
                    runs.push(ChangedRun {
                        row: run.row,
                        col: u16::try_from(from).unwrap_or(run.col),
                        width: u16::try_from(column - from).unwrap_or(run.width),
                    });
                    open = None;
                }
                _ => {}
            }
        }
    }
    runs.drain(..original);
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
