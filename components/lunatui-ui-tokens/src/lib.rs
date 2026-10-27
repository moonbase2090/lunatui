// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Color;
use lunatui_theme::Token;
use lunatui_widgets::RenderCx;

/// Semantic role a component paints with.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiToken {
    /// Page background.
    Background,
    /// Default text.
    Foreground,
    /// Raised surface behind cards.
    Card,
    /// Surface behind floating panels such as dialogs.
    Popover,
    /// Main action color.
    Primary,
    /// Secondary text and quiet fills.
    Muted,
    /// Highlight for hovered or selected items.
    Accent,
    /// Errors and destructive actions.
    Destructive,
    /// Border and separator lines.
    Border,
    /// Focus ring.
    Ring,
}

impl UiToken {
    /// Theme token this role maps onto.
    #[must_use]
    pub const fn token(self) -> Token {
        match self {
            Self::Background => Token::Surface0,
            Self::Foreground => Token::TextPrimary,
            #[cfg(feature = "graphite-tokens")]
            Self::Card | Self::Popover => Token::SurfaceRaised,
            #[cfg(not(feature = "graphite-tokens"))]
            Self::Card | Self::Popover => Token::Surface1,
            Self::Primary | Self::Accent => Token::Accent,
            #[cfg(feature = "graphite-tokens")]
            Self::Muted => Token::TextMuted,
            #[cfg(not(feature = "graphite-tokens"))]
            Self::Muted => Token::TextPrimary,
            Self::Destructive => Token::Fail,
            #[cfg(feature = "graphite-tokens")]
            Self::Border => Token::Border,
            #[cfg(not(feature = "graphite-tokens"))]
            Self::Border => Token::Surface1,
            Self::Ring => Token::FocusRing,
        }
    }

    /// Resolves the role's color through the frame's theme or prepared palette.
    #[must_use]
    pub fn color(self, cx: &RenderCx<'_>) -> Color {
        cx.token(self.token())
    }
}

/// Color family for tone-aware components such as badges and buttons.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Quiet, theme-neutral look.
    #[default]
    Neutral,
    /// Main accent.
    Primary,
    /// Success.
    Success,
    /// Warning.
    Warning,
    /// Error or destructive.
    Destructive,
}

impl Tone {
    /// Theme token that fills this tone.
    #[must_use]
    pub const fn token(self) -> Token {
        match self {
            Self::Neutral => UiToken::Card.token(),
            Self::Primary => Token::Accent,
            Self::Success => Token::Ok,
            Self::Warning => Token::Warn,
            Self::Destructive => Token::Fail,
        }
    }

    /// Resolves the fill color for this tone.
    #[must_use]
    pub fn fill(self, cx: &RenderCx<'_>) -> Color {
        cx.token(self.token())
    }

    /// Text color that reads on [`fill`](Self::fill): the page background on colored tones.
    #[must_use]
    pub fn on_fill(self, cx: &RenderCx<'_>) -> Color {
        match self {
            Self::Neutral => UiToken::Foreground.color(cx),
            _ => UiToken::Background.color(cx),
        }
    }
}
