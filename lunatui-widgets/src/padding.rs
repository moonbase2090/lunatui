// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Inner padding inside a [`Block`](crate::Block) border.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Padding {
    /// Left padding in columns.
    pub left: u16,
    /// Right padding in columns.
    pub right: u16,
    /// Top padding in rows.
    pub top: u16,
    /// Bottom padding in rows.
    pub bottom: u16,
}

impl Padding {
    /// No padding.
    pub const ZERO: Self = Self {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    };
}
