// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Semantic colors with stable packed token ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Token {
    /// Semantic `surface.0` color.
    Surface0 = 0,
    /// Semantic `surface.1` color.
    Surface1 = 1,
    /// Semantic `text.primary` color.
    TextPrimary = 2,
    /// Semantic `accent` color.
    Accent = 3,
    /// Semantic `ok` color.
    Ok = 4,
    /// Semantic `warn` color.
    Warn = 5,
    /// Semantic `fail` color.
    Fail = 6,
    /// Semantic `focus.ring` color.
    FocusRing = 7,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `border` color.
    Border = 8,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `text.muted` color.
    TextMuted = 9,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `text.strong` color.
    TextStrong = 10,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `surface.raised` color.
    SurfaceRaised = 11,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `border.title` color.
    BorderTitle = 12,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `hover.outline` color.
    HoverOutline = 13,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `text.idle` color.
    TextIdle = 14,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `focus.fill` color.
    FocusFill = 15,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `focus.line` color.
    FocusLine = 16,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `text.muted.focus` color.
    TextMutedOnFocus = 17,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `on.danger` color.
    OnDanger = 18,
    #[cfg(feature = "graphite-tokens")]
    /// Semantic `motion.head` color.
    MotionHead = 19,
}

impl Token {
    /// Number of enabled tokens.
    #[cfg(feature = "graphite-tokens")]
    pub const COUNT: usize = 20;
    /// Number of enabled tokens.
    #[cfg(not(feature = "graphite-tokens"))]
    pub const COUNT: usize = 8;

    /// Stable id stored in [`lunatui_core::Color::Token`].
    #[must_use]
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// Parse a packed token id.
    #[must_use]
    pub const fn from_id(id: u32) -> Option<Self> {
        let tokens = Self::all();
        let index = id as usize;
        if index < tokens.len() {
            Some(tokens[index])
        } else {
            None
        }
    }

    /// Theme-file name for this token.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Surface0 => "surface.0",
            Self::Surface1 => "surface.1",
            Self::TextPrimary => "text.primary",
            Self::Accent => "accent",
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::FocusRing => "focus.ring",
            #[cfg(feature = "graphite-tokens")]
            Self::Border => "border",
            #[cfg(feature = "graphite-tokens")]
            Self::TextMuted => "text.muted",
            #[cfg(feature = "graphite-tokens")]
            Self::TextStrong => "text.strong",
            #[cfg(feature = "graphite-tokens")]
            Self::SurfaceRaised => "surface.raised",
            #[cfg(feature = "graphite-tokens")]
            Self::BorderTitle => "border.title",
            #[cfg(feature = "graphite-tokens")]
            Self::HoverOutline => "hover.outline",
            #[cfg(feature = "graphite-tokens")]
            Self::TextIdle => "text.idle",
            #[cfg(feature = "graphite-tokens")]
            Self::FocusFill => "focus.fill",
            #[cfg(feature = "graphite-tokens")]
            Self::FocusLine => "focus.line",
            #[cfg(feature = "graphite-tokens")]
            Self::TextMutedOnFocus => "text.muted.focus",
            #[cfg(feature = "graphite-tokens")]
            Self::OnDanger => "on.danger",
            #[cfg(feature = "graphite-tokens")]
            Self::MotionHead => "motion.head",
        }
    }

    /// All enabled tokens in stable id order.
    #[must_use]
    pub const fn all() -> [Self; Self::COUNT] {
        [
            Self::Surface0,
            Self::Surface1,
            Self::TextPrimary,
            Self::Accent,
            Self::Ok,
            Self::Warn,
            Self::Fail,
            Self::FocusRing,
            #[cfg(feature = "graphite-tokens")]
            Self::Border,
            #[cfg(feature = "graphite-tokens")]
            Self::TextMuted,
            #[cfg(feature = "graphite-tokens")]
            Self::TextStrong,
            #[cfg(feature = "graphite-tokens")]
            Self::SurfaceRaised,
            #[cfg(feature = "graphite-tokens")]
            Self::BorderTitle,
            #[cfg(feature = "graphite-tokens")]
            Self::HoverOutline,
            #[cfg(feature = "graphite-tokens")]
            Self::TextIdle,
            #[cfg(feature = "graphite-tokens")]
            Self::FocusFill,
            #[cfg(feature = "graphite-tokens")]
            Self::FocusLine,
            #[cfg(feature = "graphite-tokens")]
            Self::TextMutedOnFocus,
            #[cfg(feature = "graphite-tokens")]
            Self::OnDanger,
            #[cfg(feature = "graphite-tokens")]
            Self::MotionHead,
        ]
    }

    pub(crate) const fn file_required() -> [Self; 8] {
        [
            Self::Surface0,
            Self::Surface1,
            Self::TextPrimary,
            Self::Accent,
            Self::Ok,
            Self::Warn,
            Self::Fail,
            Self::FocusRing,
        ]
    }

    /// Parse a theme-file token name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "surface.0" => Some(Self::Surface0),
            "surface.1" => Some(Self::Surface1),
            "text.primary" => Some(Self::TextPrimary),
            "accent" => Some(Self::Accent),
            "ok" => Some(Self::Ok),
            "warn" => Some(Self::Warn),
            "fail" => Some(Self::Fail),
            "focus.ring" => Some(Self::FocusRing),
            #[cfg(feature = "graphite-tokens")]
            "border" => Some(Self::Border),
            #[cfg(feature = "graphite-tokens")]
            "text.muted" => Some(Self::TextMuted),
            #[cfg(feature = "graphite-tokens")]
            "text.strong" => Some(Self::TextStrong),
            #[cfg(feature = "graphite-tokens")]
            "surface.raised" => Some(Self::SurfaceRaised),
            #[cfg(feature = "graphite-tokens")]
            "border.title" => Some(Self::BorderTitle),
            #[cfg(feature = "graphite-tokens")]
            "hover.outline" => Some(Self::HoverOutline),
            #[cfg(feature = "graphite-tokens")]
            "text.idle" => Some(Self::TextIdle),
            #[cfg(feature = "graphite-tokens")]
            "focus.fill" => Some(Self::FocusFill),
            #[cfg(feature = "graphite-tokens")]
            "focus.line" => Some(Self::FocusLine),
            #[cfg(feature = "graphite-tokens")]
            "text.muted.focus" => Some(Self::TextMutedOnFocus),
            #[cfg(feature = "graphite-tokens")]
            "on.danger" => Some(Self::OnDanger),
            #[cfg(feature = "graphite-tokens")]
            "motion.head" => Some(Self::MotionHead),
            _ => None,
        }
    }
}
