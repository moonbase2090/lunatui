// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::string::String;

use super::{RowIndex, Wrap, wrap_rows};

/// Cached wrap index for append-friendly logs (PRD §2.5).
#[derive(Clone, Debug, Default)]
pub struct WrapCache {
    text: String,
    width: u16,
    wrap: Wrap,
    index: RowIndex,
}

impl WrapCache {
    /// Empty cache; call [`sync`] before reading rows.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the index when `text`, `width`, or `wrap` changed.
    pub fn sync(&mut self, text: &str, width: u16, wrap: Wrap) {
        if self.text.as_str() == text && self.width == width && self.wrap == wrap {
            return;
        }
        self.text.clear();
        self.text.push_str(text);
        self.width = width;
        self.wrap = wrap;
        self.index = wrap_rows(text, width, wrap);
    }

    /// Visual row count for the cached text.
    #[must_use]
    pub fn visual_rows(&self) -> usize {
        self.index.len()
    }

    /// Text slice for visual row `row`.
    #[must_use]
    pub fn row_text(&self, row: usize) -> Option<&str> {
        self.index.row_text(self.text.as_str(), row)
    }

    /// Cached row index (read-only).
    #[must_use]
    pub fn index(&self) -> &RowIndex {
        &self.index
    }
}
