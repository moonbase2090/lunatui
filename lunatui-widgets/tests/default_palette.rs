// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(not(feature = "graphite-render"))]

use lunatui_core::Color;
use lunatui_theme::{Theme, Token, Variant};

#[test]
fn turning_on_every_widget_keeps_the_default_dark_palette() {
    let theme = Theme::builtin(Variant::Dark);
    let base = [
        (Token::Surface0, Color::Rgb(0x1e, 0x1e, 0x2e)),
        (Token::Surface1, Color::Rgb(0x31, 0x32, 0x44)),
        (Token::TextPrimary, Color::Rgb(0xcd, 0xd6, 0xf4)),
        (Token::Accent, Color::Rgb(0x9e, 0xce, 0x6a)),
        (Token::Ok, Color::Rgb(0xa6, 0xe3, 0xa1)),
        (Token::Warn, Color::Rgb(0xfa, 0xbd, 0x2f)),
        (Token::Fail, Color::Rgb(0xf3, 0x8b, 0xa8)),
        (Token::FocusRing, Color::Rgb(0xb4, 0xbc, 0xfa)),
    ];
    for (token, color) in base {
        assert_eq!(theme.resolve(token), color, "{token:?}");
    }
}

#[test]
fn the_nested_menu_takes_graphite_colors_only_for_its_extra_tokens() {
    let theme = Theme::builtin(Variant::Dark);
    assert_eq!(theme.resolve(Token::SurfaceRaised), Color::Rgb(37, 42, 51));
    assert_eq!(theme.resolve(Token::FocusFill), Color::Rgb(28, 35, 48));
    assert_eq!(
        theme.resolve(Token::TextMutedOnFocus),
        Color::Rgb(180, 188, 201)
    );
    assert_eq!(theme.resolve(Token::TextMuted), Color::Rgb(163, 171, 184));
}
