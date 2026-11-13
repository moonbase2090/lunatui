// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_theme::Token;

/// Interaction state a widget paints itself for.
///
/// Widgets map each state to semantic theme tokens, so a theme change recolors them without
/// per-widget styling. [`WidgetState::token`] is that mapping.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetState {
    /// Resting appearance.
    #[default]
    Normal,
    /// The widget has input focus.
    Focused,
    /// The widget is inactive and drawn dimmed.
    Disabled,
    /// The pointer is over the widget.
    Hover,
    /// The widget is held down.
    Pressed,
    /// The value fails validation.
    Invalid,
    /// A disclosure or popup is expanded.
    Open,
}

impl WidgetState {
    /// Theme token this state paints with.
    ///
    /// `Normal` is `text.primary`, `Focused` and `Hover` are `focus.ring`, `Disabled` is
    /// `surface.1`, `Pressed` is `surface.0`, `Invalid` is `fail`, and `Open` is `accent`.
    #[must_use]
    pub const fn token(self) -> Token {
        match self {
            Self::Normal => Token::TextPrimary,
            Self::Focused | Self::Hover => Token::FocusRing,
            Self::Disabled => Token::Surface1,
            Self::Pressed => Token::Surface0,
            Self::Invalid => Token::Fail,
            Self::Open => Token::Accent,
        }
    }
}
