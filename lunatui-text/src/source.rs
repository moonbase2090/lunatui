// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Reusable cell buffer for [`RowSource::row`].
#[derive(Clone, Debug, Default)]
pub struct RowBuilder {
    /// One string per table column.
    pub cells: Vec<String>,
}

impl RowBuilder {
    /// Clears prior row contents.
    pub fn clear(&mut self) {
        self.cells.clear();
    }
}

/// Virtualized row backing store for lists and tables (PRD §2.5).
pub trait RowSource {
    /// Total logical row count.
    fn len(&self) -> usize;

    /// Whether there are no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `out` with column values for row `index`.
    fn row(&self, index: usize, out: &mut RowBuilder);
}

/// In-memory [`RowSource`] from owned rows.
#[derive(Clone, Debug, Default)]
pub struct VecSource {
    rows: Vec<Vec<String>>,
}

impl VecSource {
    /// Empty source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds from fully materialized rows.
    #[must_use]
    pub fn from_rows(rows: Vec<Vec<String>>) -> Self {
        Self { rows }
    }

    /// Appends one row.
    pub fn push_row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }
}

impl RowSource for VecSource {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn row(&self, index: usize, out: &mut RowBuilder) {
        out.clear();
        if let Some(row) = self.rows.get(index) {
            out.cells.extend(row.iter().cloned());
        }
    }
}

/// [`RowSource`] over a borrowed slice of row slices.
pub struct SliceSource<'a> {
    rows: &'a [&'a [String]],
}

impl<'a> SliceSource<'a> {
    /// Wraps borrowed rows.
    #[must_use]
    pub fn new(rows: &'a [&'a [String]]) -> Self {
        Self { rows }
    }
}

impl RowSource for SliceSource<'_> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn row(&self, index: usize, out: &mut RowBuilder) {
        out.clear();
        if let Some(row) = self.rows.get(index) {
            out.cells.extend(row.iter().cloned());
        }
    }
}
