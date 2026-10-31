// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Style};
use lunatui_theme::Token;
use lunatui_widgets::{BorderType, RenderCx};

#[cfg(feature = "graphite")]
const RAISED: Token = Token::SurfaceRaised;
#[cfg(not(feature = "graphite"))]
const RAISED: Token = Token::Surface1;
#[cfg(feature = "graphite")]
const MUTED: Token = Token::TextMuted;
#[cfg(not(feature = "graphite"))]
const MUTED: Token = Token::TextPrimary;
#[cfg(feature = "graphite")]
const BORDER_TOKEN: Token = Token::Border;
#[cfg(not(feature = "graphite"))]
const BORDER_TOKEN: Token = Token::FocusRing;
#[cfg(feature = "graphite")]
const ON_DANGER: Token = Token::OnDanger;
#[cfg(not(feature = "graphite"))]
const ON_DANGER: Token = Token::Surface0;

/// Semantic color roles shared by UI components.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Page background.
    Background,
    /// Default text on the background.
    Foreground,
    /// Raised surface behind cards.
    Card,
    /// Surface behind overlays and popovers.
    Popover,
    /// Main brand or action color.
    Primary,
    /// Secondary, low-emphasis text.
    Muted,
    /// Highlight color for the active item.
    Accent,
    /// Destructive or error color.
    Destructive,
    /// Border and divider lines.
    Border,
    /// Focus ring.
    Ring,
}

impl Role {
    /// The core theme token this role resolves to.
    #[must_use]
    pub const fn token(self) -> Token {
        match self {
            Self::Background => Token::Surface0,
            Self::Foreground => Token::TextPrimary,
            Self::Card | Self::Popover => RAISED,
            Self::Primary | Self::Accent => Token::Accent,
            Self::Muted => MUTED,
            Self::Destructive => Token::Fail,
            Self::Border => BORDER_TOKEN,
            Self::Ring => Token::FocusRing,
        }
    }
}

/// Emphasis of a badge, button or alert.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Quiet raised fill.
    #[default]
    Neutral,
    /// Accent fill.
    Primary,
    /// Success fill.
    Success,
    /// Warning fill.
    Warning,
    /// Danger fill.
    Danger,
}

impl Tone {
    /// Token used for the tone's fill.
    #[must_use]
    pub const fn fill(self) -> Token {
        match self {
            Self::Neutral => RAISED,
            Self::Primary => Token::Accent,
            Self::Success => Token::Ok,
            Self::Warning => Token::Warn,
            Self::Danger => Token::Fail,
        }
    }

    /// Token used for text drawn on the tone's fill.
    #[must_use]
    pub const fn on_fill(self) -> Token {
        match self {
            Self::Neutral => Token::TextPrimary,
            Self::Danger => ON_DANGER,
            Self::Primary | Self::Success | Self::Warning => Token::Surface0,
        }
    }

    /// Filled style: [`on_fill`](Self::on_fill) text, bold, on the fill.
    #[must_use]
    pub fn style(self, cx: &RenderCx<'_>) -> Style {
        Style::new()
            .fg(cx.token(self.on_fill()))
            .bg(cx.token(self.fill()))
            .with_attrs(Attrs::new().set_bold(true))
    }
}

/// Spacing scale in cells; horizontal gaps use [`Spacing::x`] (double width).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Spacing(pub u16);

impl Spacing {
    /// `space.0`.
    pub const S0: Self = Self(0);
    /// `space.1`.
    pub const S1: Self = Self(1);
    /// `space.2`.
    pub const S2: Self = Self(2);
    /// `space.3`.
    pub const S3: Self = Self(3);
    /// `space.4`.
    pub const S4: Self = Self(4);

    /// Vertical cells.
    #[must_use]
    pub const fn y(self) -> u16 {
        self.0
    }

    /// Horizontal cells; cells are about twice as tall as wide.
    #[must_use]
    pub const fn x(self) -> u16 {
        self.0 * 2
    }
}

/// Default border style for component frames.
pub const BORDER: BorderType = BorderType::Rounded;

/// Style for a [`Role`] used as text.
#[must_use]
pub fn text(cx: &RenderCx<'_>, role: Role) -> Style {
    Style::new().fg(cx.token(role.token()))
}

/// Style for a [`Role`] used as a fill with `Foreground` text on top.
#[must_use]
pub fn surface(cx: &RenderCx<'_>, role: Role) -> Style {
    Style::new()
        .fg(cx.token(Role::Foreground.token()))
        .bg(cx.token(role.token()))
}
