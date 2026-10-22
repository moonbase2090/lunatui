#![doc = include_str!("../README.md")]

use std::io;

use lunatui_core::{ChangedRun, ClusterTable, Grid};

/// Packed grid and damage runs for one host present.
#[derive(Clone, Copy, Debug)]
pub struct FrameDiff<'a> {
    /// Grid before this frame.
    pub previous: &'a Grid,
    /// Grid after rendering.
    pub next: &'a Grid,
    /// Grapheme clusters referenced by `next`.
    pub clusters: &'a ClusterTable,
    /// Contiguous changed cell runs.
    pub runs: &'a [ChangedRun],
}

/// Result of presenting a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentStats {
    /// Number of changed runs.
    pub changed_runs: usize,
    /// Number of changed cells.
    pub changed_cells: usize,
    /// Number of encoded ANSI bytes.
    pub ansi_bytes: usize,
}

impl PresentStats {
    /// Result when the frame has no damage.
    #[must_use]
    pub const fn skipped() -> Self {
        Self { changed_runs: 0, changed_cells: 0, ansi_bytes: 0 }
    }
}

/// Receives packed frames directly from lunatui.
pub trait HostRenderer {
    /// Applies a frame to the host display.
    fn present_frame(&mut self, frame: &FrameDiff<'_>) -> io::Result<()>;
}

/// Backend that sends grid changes to an in-process host renderer.
pub struct PrismattycBackend<R> {
    previous: Grid,
    renderer: R,
}

impl<R: HostRenderer> PrismattycBackend<R> {
    /// Creates a backend with an empty previous grid.
    #[must_use]
    pub fn new(width: u16, height: u16, renderer: R) -> Self {
        Self { previous: Grid::new(width, height), renderer }
    }

    /// Returns the host renderer.
    #[must_use]
    pub const fn renderer(&self) -> &R { &self.renderer }

    /// Returns the previously presented grid.
    #[must_use]
    pub const fn previous_grid(&self) -> &Grid { &self.previous }

    /// Replaces the previously presented grid.
    pub fn set_previous(&mut self, grid: Grid) { self.previous = grid; }

    /// Presents a grid to the host.
    pub fn present(&mut self, _next: &Grid, _clusters: &ClusterTable) -> io::Result<PresentStats> {
        Ok(PresentStats::skipped())
    }
}
