// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Options for an interactive [`run`](crate::run::run) session (PRD §3.2, roadmap PR 23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunOptions {
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Whether to probe for pixel images, with automatic fallback when unsupported.
    pub pixel_images: bool,
}

impl RunOptions {
    /// Builds options for a fullscreen session at the given size.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixel_images: true,
        }
    }

    /// Enables pixel images (the default), or skips graphics probing and uses cell fallbacks.
    #[must_use]
    pub const fn pixels(mut self, enabled: bool) -> Self {
        self.pixel_images = enabled;
        self
    }
}
