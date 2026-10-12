// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_term::Event;

/// Whether the host loop should keep running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlFlow {
    /// Continue processing input and rendering.
    Continue,
    /// Exit after the current frame is presented.
    Break,
}

/// Application state and rendering for [`run`](crate::run::run).
pub trait App {
    /// Handle one input event. Return [`ControlFlow::Break`] to quit (for example on `q`).
    fn update(&mut self, event: &Event) -> ControlFlow;

    /// Draw the current UI into `grid` for `area`.
    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable);
}
