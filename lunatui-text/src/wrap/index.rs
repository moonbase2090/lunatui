// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::vec::Vec;

/// Byte offsets where each visual row starts in the wrapped source text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowIndex {
    /// Start offset of each visual row; always includes `0` when non-empty.
    pub row_starts: Vec<usize>,
}

impl RowIndex {
    /// Number of visual rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.row_starts.len()
    }

    /// Whether there are no visual rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.row_starts.is_empty()
    }

    /// Byte offset where visual row `row` begins.
    #[must_use]
    pub fn start(&self, row: usize) -> Option<usize> {
        self.row_starts.get(row).copied()
    }

    /// Exclusive end byte offset for visual row `row`.
    #[must_use]
    pub fn end(&self, row: usize, text_len: usize) -> Option<usize> {
        if row >= self.row_starts.len() {
            return None;
        }
        let next = self.row_starts.get(row + 1).copied().unwrap_or(text_len);
        Some(next)
    }

    /// Slice of `text` for one visual row.
    #[must_use]
    pub fn row_text<'a>(&self, text: &'a str, row: usize) -> Option<&'a str> {
        let start = self.start(row)?;
        let end = self.end(row, text.len())?;
        text.get(start..end)
    }
}
