// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Options for an interactive [`run`](crate::run::run) session (PRD §3.2, roadmap PR 23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunOptions {
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Maximum animation rate when the scheduler is animating.
    pub fps_cap: u32,
}

impl RunOptions {
    /// Builds options for a fullscreen session at the given size.
    #[must_use]
    pub const fn new(width: u16, height: u16, fps_cap: u32) -> Self {
        Self {
            width,
            height,
            fps_cap,
        }
    }
}
