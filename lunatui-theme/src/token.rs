// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Semantic colors with stable packed token ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
#[non_exhaustive]
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
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `border` color.
    Border = 8,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `text.muted` color.
    TextMuted = 9,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `text.strong` color.
    TextStrong = 10,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `surface.raised` color.
    SurfaceRaised = 11,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `border.title` color.
    BorderTitle = 12,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `hover.outline` color.
    HoverOutline = 13,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `text.idle` color.
    TextIdle = 14,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `focus.fill` color.
    FocusFill = 15,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `focus.line` color.
    FocusLine = 16,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `text.muted.focus` color.
    TextMutedOnFocus = 17,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `on.danger` color.
    OnDanger = 18,
    #[cfg(feature = "graphite-extensions")]
    /// Semantic `motion.head` color.
    MotionHead = 19,
}

impl Token {
    /// Number of tokens enabled in this Cargo feature set.
    ///
    /// This is 20 when `graphite-extensions` (or `graphite-tokens`, which turns
    /// it on) is enabled and 8 otherwise. Cargo
    /// unifies dependency features, so another dependency can enable the
    /// additional variants for the whole build. Do not persist this value as
    /// a format size or assume a fixed count across feature sets.
    #[cfg(feature = "graphite-extensions")]
    pub const COUNT: usize = 20;
    /// Number of tokens enabled in this Cargo feature set.
    ///
    /// This is 20 when `graphite-extensions` (or `graphite-tokens`, which turns
    /// it on) is enabled and 8 otherwise. Cargo
    /// unifies dependency features, so another dependency can enable the
    /// additional variants for the whole build. Do not persist this value as
    /// a format size or assume a fixed count across feature sets.
    #[cfg(not(feature = "graphite-extensions"))]
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
            #[cfg(feature = "graphite-extensions")]
            Self::Border => "border",
            #[cfg(feature = "graphite-extensions")]
            Self::TextMuted => "text.muted",
            #[cfg(feature = "graphite-extensions")]
            Self::TextStrong => "text.strong",
            #[cfg(feature = "graphite-extensions")]
            Self::SurfaceRaised => "surface.raised",
            #[cfg(feature = "graphite-extensions")]
            Self::BorderTitle => "border.title",
            #[cfg(feature = "graphite-extensions")]
            Self::HoverOutline => "hover.outline",
            #[cfg(feature = "graphite-extensions")]
            Self::TextIdle => "text.idle",
            #[cfg(feature = "graphite-extensions")]
            Self::FocusFill => "focus.fill",
            #[cfg(feature = "graphite-extensions")]
            Self::FocusLine => "focus.line",
            #[cfg(feature = "graphite-extensions")]
            Self::TextMutedOnFocus => "text.muted.focus",
            #[cfg(feature = "graphite-extensions")]
            Self::OnDanger => "on.danger",
            #[cfg(feature = "graphite-extensions")]
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
            #[cfg(feature = "graphite-extensions")]
            Self::Border,
            #[cfg(feature = "graphite-extensions")]
            Self::TextMuted,
            #[cfg(feature = "graphite-extensions")]
            Self::TextStrong,
            #[cfg(feature = "graphite-extensions")]
            Self::SurfaceRaised,
            #[cfg(feature = "graphite-extensions")]
            Self::BorderTitle,
            #[cfg(feature = "graphite-extensions")]
            Self::HoverOutline,
            #[cfg(feature = "graphite-extensions")]
            Self::TextIdle,
            #[cfg(feature = "graphite-extensions")]
            Self::FocusFill,
            #[cfg(feature = "graphite-extensions")]
            Self::FocusLine,
            #[cfg(feature = "graphite-extensions")]
            Self::TextMutedOnFocus,
            #[cfg(feature = "graphite-extensions")]
            Self::OnDanger,
            #[cfg(feature = "graphite-extensions")]
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
            #[cfg(feature = "graphite-extensions")]
            "border" => Some(Self::Border),
            #[cfg(feature = "graphite-extensions")]
            "text.muted" => Some(Self::TextMuted),
            #[cfg(feature = "graphite-extensions")]
            "text.strong" => Some(Self::TextStrong),
            #[cfg(feature = "graphite-extensions")]
            "surface.raised" => Some(Self::SurfaceRaised),
            #[cfg(feature = "graphite-extensions")]
            "border.title" => Some(Self::BorderTitle),
            #[cfg(feature = "graphite-extensions")]
            "hover.outline" => Some(Self::HoverOutline),
            #[cfg(feature = "graphite-extensions")]
            "text.idle" => Some(Self::TextIdle),
            #[cfg(feature = "graphite-extensions")]
            "focus.fill" => Some(Self::FocusFill),
            #[cfg(feature = "graphite-extensions")]
            "focus.line" => Some(Self::FocusLine),
            #[cfg(feature = "graphite-extensions")]
            "text.muted.focus" => Some(Self::TextMutedOnFocus),
            #[cfg(feature = "graphite-extensions")]
            "on.danger" => Some(Self::OnDanger),
            #[cfg(feature = "graphite-extensions")]
            "motion.head" => Some(Self::MotionHead),
            _ => None,
        }
    }
}
