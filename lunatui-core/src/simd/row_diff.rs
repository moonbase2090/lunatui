// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::vec::Vec;

use crate::diff::ChangedRun;
use crate::grid::Grid;

use super::first_mismatch;

fn index_to_u16(index: usize) -> u16 {
    u16::try_from(index).unwrap_or(u16::MAX)
}

/// Scalar row scan (benchmark baseline for `collect_runs_in_row_simd`).
#[must_use]
pub fn collect_runs_in_row_scalar(
    previous: &Grid,
    next: &Grid,
    row: u16,
    start: u16,
    end: u16,
) -> Vec<ChangedRun> {
    let mut runs = Vec::new();
    collect_runs_in_row_scalar_tail(previous, next, row, start, end, &mut runs);
    runs
}

/// SIMD-accelerated row scan (benchmark harness; production `diff_runs` stays scalar until the Phase 7 gate).
#[must_use]
pub fn collect_runs_in_row_simd(
    previous: &Grid,
    next: &Grid,
    row: u16,
    start: u16,
    end: u16,
) -> Vec<ChangedRun> {
    let mut runs = Vec::new();
    if row >= previous.height() {
        collect_runs_in_row_scalar_tail(previous, next, row, start, end, &mut runs);
        return runs;
    }
    let prev_row = previous.row_word_slice(row);
    let next_row = next.row_word_slice(row);
    let start_usize = usize::from(start);
    let end_usize = usize::from(end);
    let simd_end = end_usize.min(prev_row.len()).min(next_row.len());
    let mut col = start_usize;
    while col < simd_end {
        let prev_tail = &prev_row[col..simd_end];
        let next_tail = &next_row[col..simd_end];
        match first_mismatch(prev_tail, next_tail) {
            None => break,
            Some(0) => {
                let run_start = col;
                col += 1;
                while col < simd_end && prev_row[col] != next_row[col] {
                    col += 1;
                }
                runs.push(ChangedRun {
                    row,
                    col: index_to_u16(run_start),
                    width: index_to_u16(col - run_start),
                });
            }
            Some(skip) => col += skip,
        }
    }
    if col < end_usize {
        collect_runs_in_row_scalar_tail(previous, next, row, index_to_u16(col), end, &mut runs);
    }
    coalesce_adjacent_runs(&mut runs);
    runs
}

fn coalesce_adjacent_runs(runs: &mut Vec<ChangedRun>) {
    let mut merged: Vec<ChangedRun> = Vec::new();
    for run in runs.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.row == run.row
            && last.col.saturating_add(last.width) == run.col
        {
            last.width = last.width.saturating_add(run.width);
        } else {
            merged.push(run);
        }
    }
    *runs = merged;
}

fn collect_runs_in_row_scalar_tail(
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
