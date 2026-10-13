// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Semantic theme token (PRD §2.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Token {
    /// Base surface layer.
    Surface0 = 0,
    /// Raised surface.
    Surface1 = 1,
    /// Primary text.
    TextPrimary = 2,
    /// Accent highlight.
    Accent = 3,
    /// Success state.
    Ok = 4,
    /// Warning state.
    Warn = 5,
    /// Error state.
    Fail = 6,
    /// Focus ring.
    FocusRing = 7,
}

impl Token {
    /// Number of built-in tokens.
    pub const COUNT: usize = 8;

    /// Stable id stored in [`lunatui_core::Color::Token`].
    #[must_use]
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// Parses a built-in token from `id`.
    #[must_use]
    pub const fn from_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Surface0),
            1 => Some(Self::Surface1),
            2 => Some(Self::TextPrimary),
            3 => Some(Self::Accent),
            4 => Some(Self::Ok),
            5 => Some(Self::Warn),
            6 => Some(Self::Fail),
            7 => Some(Self::FocusRing),
            _ => None,
        }
    }

    /// Serde / file name for this token.
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
        }
    }

    /// Every built-in token in stable id order.
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
        ]
    }

    /// Parses a token name from theme files.
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
            _ => None,
        }
    }
}
