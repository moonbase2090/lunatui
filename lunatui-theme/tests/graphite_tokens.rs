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
                0x0010_1216,
                0x0018_1b21,
                0x00e6_e9ee,
                0x0062_a8ff,
                0x004c_c98a,
                0x00f2_b84b,
                0x00ff_7a6b,
                0x0062_a8ff,
                0x0026_2a32,
                0x00a3_abb8,
                0x00f2_f4f7,
                0x0025_2a33,
                0x0022_262d,
                0x003d_5f8f,
                0x006b_7380,
                0x001c_2330,
                0x002a_3140,
                0x00b4_bcc9,
                0x001a_0d0b,
                0x00d6_e8ff,
            ],
        ),
        (
            Variant::Light,
            [
                0x00e9_ecf0,
                0x00ff_ffff,
                0x001f_2329,
                0x004f_89d1,
                0x001f_8a55,
                0x00b8_7a0a,
                0x00c2_392b,
                0x004f_89d1,
                0x00d5_d9df,
                0x005b_6472,
                0x001f_2329,
                0x00e2_e6eb,
                0x00e3_e6ea,
                0x009d_bbe8,
                0x008a_929e,
                0x00ea_f1fc,
                0x00cd_dcf3,
                0x004a_525e,
                0x00ff_ffff,
                0x0016_3f80,
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
