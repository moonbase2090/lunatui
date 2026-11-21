// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid};
use lunatui_image::ImageSink;

use crate::RenderMode;

/// Buffers and pixel image sink for one app render pass.
pub struct RenderFrame<'a> {
    grid: &'a mut Grid,
    clusters: &'a mut ClusterTable,
    images: &'a mut dyn ImageSink,
    mode: RenderMode,
}

impl<'a> RenderFrame<'a> {
    pub(crate) fn new(
        grid: &'a mut Grid,
        clusters: &'a mut ClusterTable,
        images: &'a mut dyn ImageSink,
        mode: RenderMode,
    ) -> Self {
        Self {
            grid,
            clusters,
            images,
            mode,
        }
    }

    /// How this frame is drawn. A backend that shows cells only always gives [`RenderMode::Cell`].
    #[must_use]
    pub const fn mode(&self) -> RenderMode {
        self.mode
    }

    /// Borrows the cell buffers and image sink so cell and pixel output use
    /// geometry from the same widget render pass.
    pub fn parts(&mut self) -> (&mut Grid, &mut ClusterTable, &mut dyn ImageSink) {
        (self.grid, self.clusters, self.images)
    }
}
