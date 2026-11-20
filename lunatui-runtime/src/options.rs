// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::ModePolicy;

/// Options for an interactive [`run`](crate::run::run) session (PRD §3.2, roadmap PR 23).
///
/// Downstream code cannot build this with a struct literal, because fields can be added. Start from the constructor and change fields on the value:
///
/// ```compile_fail,E0639
/// use lunatui_runtime::RunOptions;
///
/// fn literal() -> RunOptions {
///     let base = RunOptions::new(1, 1);
///     RunOptions { width: 2, ..base }
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunOptions {
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Whether to probe for pixel images, with automatic fallback when unsupported.
    pub pixel_images: bool,
    /// What the application asks for: Graphite mode when the backend can show it, cells only, or
    /// automatic. The `LUNATUI_MODE` environment variable overrides it.
    pub mode: ModePolicy,
}

impl RunOptions {
    /// Builds options for a fullscreen session at the given size.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixel_images: true,
            mode: ModePolicy::Auto,
        }
    }

    /// Enables pixel images (the default), or skips graphics probing and uses cell fallbacks.
    #[must_use]
    pub const fn pixels(mut self, enabled: bool) -> Self {
        self.pixel_images = enabled;
        self
    }

    /// Sets the rendering mode policy.
    #[must_use]
    pub const fn mode(mut self, mode: ModePolicy) -> Self {
        self.mode = mode;
        self
    }
}
