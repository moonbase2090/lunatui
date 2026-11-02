// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Rect};
use lunatui_term::{Event, KeyCode, Modifiers};

#[cfg(feature = "image-presenter")]
use crate::frame::RenderFrame;

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

    /// Draws one frame with access to the optional image presenter.
    ///
    /// The default keeps existing applications source-compatible. Override
    /// this when widgets should send pixel images through the runtime.
    #[cfg(feature = "image-presenter")]
    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let (grid, clusters, _) = frame.parts();
        self.render(grid, area, clusters);
    }

    /// When true, [`run`](crate::run::run) schedules timer-driven frames until this returns false.
    fn needs_animation_frames(&self) -> bool {
        false
    }

    /// When set, [`run`](crate::run::run) resolves [`Color::Token`] through this palette at encode time.
    fn token_palette(&self) -> Option<&[Color]> {
        None
    }
}

/// Returns true when `event` is a plain `q` press (quit).
#[must_use]
pub fn is_quit(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(key) if key.code == KeyCode::Char('q') && key.modifiers == Modifiers::new()
    )
}

#[cfg(test)]
mod tests {
    use lunatui_term::{Event, KeyCode, KeyEvent};

    use super::is_quit;

    #[test]
    fn is_quit_matches_plain_q() {
        assert!(is_quit(&Event::Key(KeyEvent::press(KeyCode::Char('q')))));
        assert!(!is_quit(&Event::Key(KeyEvent::press(KeyCode::Char('x')))));
    }
}
