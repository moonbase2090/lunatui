// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io;

use lunatui_core::{ClusterTable, Grid, diff_runs};

use crate::frame::{FrameDiff, PresentStats};

/// Host-side renderer for Prismattyc chrome (in-process mode, PRD §2.12).
pub trait HostRenderer {
    /// Consumes `frame` without ANSI encoding.
    ///
    /// # Errors
    ///
    /// Returns an error when the host cannot apply the frame.
    fn present_frame(&mut self, frame: &FrameDiff<'_>) -> io::Result<()>;
}

/// In-process Prismattyc backend: delivers packed grid + damage to a host renderer.
pub struct PrismattycBackend<R> {
    previous: Grid,
    renderer: R,
}

impl<R: HostRenderer> PrismattycBackend<R> {
    /// Backend sized to `width`×`height` with an empty previous grid.
    #[must_use]
    pub fn new(width: u16, height: u16, renderer: R) -> Self {
        Self {
            previous: Grid::new(width, height),
            renderer,
        }
    }

    /// Host renderer (Prismattyc chrome GPU path).
    #[must_use]
    pub fn renderer(&self) -> &R {
        &self.renderer
    }

    /// Mutable host renderer.
    pub fn renderer_mut(&mut self) -> &mut R {
        &mut self.renderer
    }

    /// Previous grid retained between presents.
    #[must_use]
    pub fn previous_grid(&self) -> &Grid {
        &self.previous
    }

    /// Replaces the retained previous grid (harnesses that replay a fixed diff).
    pub fn set_previous(&mut self, grid: Grid) {
        self.previous = grid;
    }

    /// Presents `next` when damage exists; updates the retained previous grid.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`HostRenderer::present_frame`].
    pub fn present(&mut self, next: &Grid, clusters: &ClusterTable) -> io::Result<PresentStats> {
        let size_changed =
            self.previous.width() != next.width() || self.previous.height() != next.height();
        let runs = diff_runs(&self.previous, next);
        if runs.is_empty() && !size_changed {
            return Ok(PresentStats::skipped());
        }
        let frame = FrameDiff {
            previous: &self.previous,
            next,
            clusters,
            runs: &runs,
        };
        self.renderer.present_frame(&frame)?;
        let stats = PresentStats::from_runs(&runs);
        self.previous = next.clone();
        Ok(stats)
    }
}
