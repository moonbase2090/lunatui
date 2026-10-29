// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(not(feature = "graphite-tokens"))]

use lunatui_core::Color;
use lunatui_theme::{Profile, Theme, Token, Variant};

#[test]
fn extensions_keep_the_default_base_palette() {
    let dark = Theme::builtin(Variant::Dark);
    assert_eq!(dark.resolve(Token::Surface0), Color::Rgb(0x1e, 0x1e, 0x2e));
    assert_eq!(dark.resolve(Token::Surface1), Color::Rgb(0x31, 0x32, 0x44));
    assert_eq!(
        dark.resolve(Token::TextPrimary),
        Color::Rgb(0xcd, 0xd6, 0xf4)
    );
    assert_eq!(dark.resolve(Token::Accent), Color::Rgb(0x9e, 0xce, 0x6a));
    assert_eq!(dark.resolve(Token::FocusRing), Color::Rgb(0xb4, 0xbc, 0xfa));
    let light = Theme::builtin(Variant::Light);
    assert_eq!(light.resolve(Token::Surface0), Color::Rgb(0xf5, 0xf5, 0xf7));
    assert_eq!(light.resolve(Token::Accent), Color::Rgb(0x7a, 0xa2, 0xf7));
}

#[test]
fn extension_tokens_take_their_graphite_colors() {
    let dark = Theme::builtin(Variant::Dark);
    assert_eq!(dark.resolve(Token::SurfaceRaised), Color::Rgb(37, 42, 51));
    assert_eq!(dark.resolve(Token::FocusFill), Color::Rgb(28, 35, 48));
    assert_eq!(dark.resolve(Token::TextMuted), Color::Rgb(163, 171, 184));
    assert_eq!(dark.resolve(Token::MotionHead), Color::Rgb(214, 232, 255));
    let light = Theme::builtin(Variant::Light);
    assert_eq!(
        light.resolve(Token::SurfaceRaised),
        Color::Rgb(226, 230, 235)
    );
    assert_eq!(light.resolve(Token::MotionHead), Color::Rgb(22, 63, 128));
}

#[test]
fn ansi16_downsamples_the_base_and_indexes_the_extensions() {
    let mut theme = Theme::builtin(Variant::Dark);
    theme.set_token(Token::Accent, Color::Rgb(255, 0, 0));
    let palette = theme.prepare(Profile::Ansi16);
    assert_eq!(
        palette.resolve(Token::Accent),
        Profile::Ansi16.downsample(Color::Rgb(255, 0, 0))
    );
    assert_eq!(
        palette.resolve(Token::Surface0),
        Profile::Ansi16.downsample(Color::Rgb(0x1e, 0x1e, 0x2e))
    );
    assert_eq!(palette.resolve(Token::SurfaceRaised), Color::Indexed(8));
    assert_eq!(palette.resolve(Token::FocusFill), Color::Indexed(4));
    assert_eq!(palette.resolve(Token::MotionHead), Color::Indexed(15));
}
