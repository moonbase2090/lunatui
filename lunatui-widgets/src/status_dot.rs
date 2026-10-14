// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Style};

use crate::{RenderCx, Size, Widget};

/// Semantic status indicator (PRD §3.2 sketch).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusDot {
    state: StatusState,
}

/// Visual state for [`StatusDot`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusState {
    /// Success tone.
    Ok,
    /// Error tone.
    Fail,
    /// Neutral / idle tone.
    Idle,
}

impl StatusDot {
    /// Builds a dot for `state`.
    #[must_use]
    pub const fn new(state: StatusState) -> Self {
        Self { state }
    }
}

impl Widget for StatusDot {
    fn render(&self, area: Rect, cx: &mut RenderCx) {
        let token = match self.state {
            StatusState::Ok => "ok",
            StatusState::Fail => "fail",
            StatusState::Idle => "text.primary",
        };
        let color = cx.token_color(token);
        cx.put_str(area.x, area.y, "●", Style::new().fg(color));
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(1, 1).clamp(avail)
    }
}
