// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Color;

#[cfg(feature = "color-profiles")]
use crate::profile::Profile;
use crate::token::Token;

/// Light or dark palette variant (OSC 11 probe selects the default).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Light background defaults.
    Light,
    /// Dark background defaults.
    Dark,
}

/// Resolved token colors for one variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    variant: Variant,
    colors: [Color; Token::COUNT],
}

impl Theme {
    /// Built-in palette for `variant`.
    #[must_use]
    pub fn builtin(variant: Variant) -> Self {
        #[cfg(feature = "graphite-tokens")]
        let colors = crate::graphite::colors(variant);
        #[cfg(not(feature = "graphite-tokens"))]
        let colors = match variant {
            Variant::Light => [
                Color::Rgb(0xf5, 0xf5, 0xf7),
                Color::Rgb(0xe8, 0xe8, 0xed),
                Color::Rgb(0x1a, 0x1a, 0x2e),
                Color::Rgb(0x7a, 0xa2, 0xf7),
                Color::Rgb(0x9e, 0xce, 0x6a),
                Color::Rgb(0xe6, 0x9f, 0x66),
                Color::Rgb(0xf3, 0x8b, 0xa8),
                Color::Rgb(0x89, 0xb4, 0xfa),
            ],
            Variant::Dark => [
                Color::Rgb(0x1e, 0x1e, 0x2e),
                Color::Rgb(0x31, 0x32, 0x44),
                Color::Rgb(0xcd, 0xd6, 0xf4),
                Color::Rgb(0x9e, 0xce, 0x6a),
                Color::Rgb(0xa6, 0xe3, 0xa1),
                Color::Rgb(0xfa, 0xbd, 0x2f),
                Color::Rgb(0xf3, 0x8b, 0xa8),
                Color::Rgb(0xb4, 0xbc, 0xfa),
            ],
        };
        Self { variant, colors }
    }

    /// Active variant.
    #[must_use]
    pub const fn variant(&self) -> Variant {
        self.variant
    }

    /// Maps OSC 11 background darkness to a variant.
    #[must_use]
    pub const fn variant_from_background_dark(background_dark: Option<bool>) -> Variant {
        match background_dark {
            Some(false) => Variant::Light,
            Some(true) | None => Variant::Dark,
        }
    }

    /// Resolves a semantic token.
    #[must_use]
    pub fn resolve(&self, token: Token) -> Color {
        self.colors[token.id() as usize]
    }

    /// Resolves a packed token id.
    #[must_use]
    pub fn resolve_id(&self, id: u32) -> Color {
        Token::from_id(id).map_or(Color::Default, |token| self.resolve(token))
    }

    /// Colors indexed by token id for the encoder.
    #[must_use]
    pub fn palette(&self) -> Vec<Color> {
        self.colors.to_vec()
    }

    /// Token palette after downsampling each color into `profile`.
    #[cfg(feature = "color-profiles")]
    #[must_use]
    pub fn palette_for(&self, profile: Profile) -> Vec<Color> {
        #[cfg(feature = "graphite-tokens")]
        if profile == Profile::Ansi16 {
            return crate::graphite::ansi16(self.variant).to_vec();
        }
        self.colors
            .iter()
            .map(|&color| profile.downsample(color))
            .collect()
    }

    /// Overrides one token color.
    pub fn set_token(&mut self, token: Token, color: Color) {
        self.colors[token.id() as usize] = color;
    }

    /// Builds a theme from `variant` and optional per-token overrides.
    #[must_use]
    pub fn with_overrides(variant: Variant, overrides: &[(Token, Color)]) -> Self {
        let mut theme = Self::builtin(variant);
        for (token, color) in overrides {
            theme.set_token(*token, *color);
        }
        theme
    }
}
