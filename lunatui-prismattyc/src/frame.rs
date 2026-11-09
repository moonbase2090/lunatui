// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ChangedRun, ClusterTable, Grid};

/// Packed grid plus damage runs for one present (PRD §2.12).
///
/// Downstream code cannot list every field without `..`, because fields can be added:
///
/// ```compile_fail,E0638
/// use lunatui_prismattyc::FrameDiff;
///
/// fn destructures(value: FrameDiff<'_>) {
///     let FrameDiff { previous: _, next: _, clusters: _, runs: _ } = value;
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FrameDiff<'a> {
    /// Grid before this frame.
    pub previous: &'a Grid,
    /// Grid after rendering.
    pub next: &'a Grid,
    /// Grapheme clusters referenced by `next`.
    pub clusters: &'a ClusterTable,
    /// Contiguous changed cell runs between `previous` and `next`.
    pub runs: &'a [ChangedRun],
}

/// Accounting returned from [`crate::PrismattycBackend::present`].
///
/// Downstream code cannot list every field without `..`, because fields can be added:
///
/// ```compile_fail,E0638
/// use lunatui_prismattyc::PresentStats;
///
/// fn destructures(value: PresentStats) {
///     let PresentStats { changed_runs: _, changed_cells: _, ansi_bytes: _ } = value;
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentStats {
    /// Number of [`ChangedRun`] entries applied.
    pub changed_runs: usize,
    /// Sum of run widths (changed cells).
    pub changed_cells: usize,
    /// ANSI bytes emitted (zero for the in-process sink).
    pub ansi_bytes: usize,
}

impl PresentStats {
    /// Stats for a skipped present (no damage).
    #[must_use]
    pub const fn skipped() -> Self {
        Self {
            changed_runs: 0,
            changed_cells: 0,
            ansi_bytes: 0,
        }
    }

    /// Builds stats from diff runs for the in-process path.
    #[must_use]
    pub fn from_runs(runs: &[ChangedRun]) -> Self {
        let changed_cells = runs.iter().map(|run| usize::from(run.width)).sum();
        Self {
            changed_runs: runs.len(),
            changed_cells,
            ansi_bytes: 0,
        }
    }
}
