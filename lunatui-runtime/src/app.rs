// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Rect};
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

    /// When true, [`run`](crate::run::run) schedules timer-driven frames until this returns false.
    fn needs_animation_frames(&self) -> bool {
        false
    }

    /// When set, [`run`](crate::run::run) resolves [`Color::Token`] through this palette at encode time.
    fn token_palette(&self) -> Option<&[Color]> {
        None
    }
}
