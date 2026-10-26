// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Color;
use lunatui_theme::{Profile, Theme, Token, Variant, load_bundled, theme_from_json_strict};

#[test]
fn canonical_dark_and_light_match_literal_graphite_tokens() {
    for (variant, expected) in [
        (
            Variant::Dark,
            [
                0x101216, 0x181b21, 0xe6e9ee, 0x62a8ff, 0x4cc98a, 0xf2b84b, 0xff7a6b, 0x62a8ff,
                0x262a32, 0xa3abb8, 0xf2f4f7, 0x252a33, 0x22262d, 0x3d5f8f, 0x6b7380, 0x1c2330,
                0x2a3140, 0xb4bcc9, 0x1a0d0b, 0xd6e8ff,
            ],
        ),
        (
            Variant::Light,
            [
                0xe9ecf0, 0xffffff, 0x1f2329, 0x4f89d1, 0x1f8a55, 0xb87a0a, 0xc2392b, 0x4f89d1,
                0xd5d9df, 0x5b6472, 0x1f2329, 0xe2e6eb, 0xe3e6ea, 0x9dbbe8, 0x8a929e, 0xeaf1fc,
                0xcddcf3, 0x4a525e, 0xffffff, 0x163f80,
            ],
        ),
    ] {
        let theme = Theme::builtin(variant);
        for (token, hex) in Token::all().into_iter().zip(expected) {
            let bytes = u32::to_be_bytes(hex);
            assert_eq!(
                theme.resolve(token),
                Color::Rgb(bytes[1], bytes[2], bytes[3]),
                "{token:?}"
            );
        }
    }
}

#[test]
fn ansi16_uses_semantic_indices_even_after_rgb_overrides() {
    for (variant, expected) in [
        (
            Variant::Dark,
            [0, 0, 7, 4, 2, 3, 1, 4, 8, 8, 15, 8, 8, 4, 8, 4, 4, 8, 0, 15],
        ),
        (
            Variant::Light,
            [7, 15, 0, 4, 2, 3, 1, 4, 8, 8, 0, 7, 8, 4, 8, 4, 4, 8, 7, 4],
        ),
    ] {
        let mut theme = Theme::builtin(variant);
        theme.set_token(Token::Accent, Color::Rgb(255, 0, 0));
        let palette = theme.palette_for(Profile::Ansi16);
        assert_eq!(palette, expected.map(Color::Indexed));
        assert_eq!(
            theme.palette_for(Profile::TrueColor)[3],
            Color::Rgb(255, 0, 0)
        );
    }
}

#[test]
fn token_ids_and_names_round_trip_including_extensions() {
    assert_eq!(Token::COUNT, 20);
    assert_eq!(Token::FocusRing.id(), 7);
    assert_eq!(Token::Border.id(), 8);
    assert_eq!(Token::MotionHead.id(), 19);
    assert_eq!(Token::from_id(20), None);
    for token in Token::all() {
        assert_eq!(Token::from_id(token.id()), Some(token));
        assert_eq!(Token::from_name(token.name()), Some(token));
    }
}

#[test]
fn base_file_tokens_are_required_and_extensions_inherit_or_override()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"{"variant":"dark","tokens":{"surface.0":"010203","surface.1":"040506","text.primary":"070809","accent":"101112","ok":"131415","warn":"161718","fail":"192021","focus.ring":"222324","border":"aabbcc"}}"#;
    let theme = theme_from_json_strict(source)?;
    assert_eq!(theme.resolve(Token::Surface0), Color::Rgb(1, 2, 3));
    assert_eq!(theme.resolve(Token::Border), Color::Rgb(170, 187, 204));
    assert_eq!(theme.resolve(Token::TextMuted), Color::Rgb(163, 171, 184));
    assert!(theme_from_json_strict(&source.replace(",\"warn\":\"161718\"", "")).is_err());
    Ok(())
}

#[test]
fn bundled_prismattyc_variants_use_canonical_graphite() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        load_bundled("prismattyc")?.resolve(Token::Surface0),
        Color::Rgb(16, 18, 22)
    );
    assert_eq!(
        load_bundled("prismattyc-dark")?.resolve(Token::Border),
        Color::Rgb(38, 42, 50)
    );
    assert_eq!(
        load_bundled("prismattyc-light")?.resolve(Token::Accent),
        Color::Rgb(79, 137, 209)
    );
    Ok(())
}

#[test]
fn prepared_palette_keeps_colors_until_explicitly_rebuilt() {
    let mut theme = Theme::builtin(Variant::Dark);
    let palette = theme.prepare(Profile::TrueColor);
    assert_eq!(palette.profile(), Profile::TrueColor);
    assert_eq!(palette.colors().len(), 20);
    theme.set_token(Token::Accent, Color::Rgb(1, 2, 3));
    assert_eq!(palette.resolve(Token::Accent), Color::Rgb(98, 168, 255));
    assert_eq!(
        theme.prepare(Profile::TrueColor).resolve(Token::Accent),
        Color::Rgb(1, 2, 3)
    );
    assert_eq!(
        theme.prepare(Profile::Ansi16).resolve(Token::Accent),
        Color::Indexed(4)
    );
}
