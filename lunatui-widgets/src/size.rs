// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Width and height in terminal cells (PRD §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Size {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

impl Size {
    /// Builds a size from component lengths.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// Minimum of each axis with `avail` (layout clamps intrinsic size).
    #[must_use]
    pub fn clamp(self, avail: Size) -> Self {
        Self {
            width: self.width.min(avail.width),
            height: self.height.min(avail.height),
        }
    }
}
