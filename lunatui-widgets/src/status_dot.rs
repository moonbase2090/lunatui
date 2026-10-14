// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style};
use lunatui_theme::Token;

use crate::{RenderCx, Widget};

/// Semantic status indicator (PRD §3.2 sketch).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusDot {
    state: StatusState,
}

/// Visual state for [`StatusDot`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusState {
    /// Success tone.
    Ok,
    /// Error tone.
    Fail,
    /// Neutral / idle tone (`Token::TextPrimary`; no `dim` token yet).
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
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let token = match self.state {
            StatusState::Ok => Token::Ok,
            StatusState::Fail => Token::Fail,
            StatusState::Idle => Token::TextPrimary,
        };
        let color = cx.token(token);
        cx.put_str(area.x, area.y, "●", Style::new().fg(color));
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(1, 1).clamp(avail)
    }
}
