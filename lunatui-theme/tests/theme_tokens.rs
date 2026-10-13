// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Color;
use lunatui_theme::{Theme, ThemeLoadError, Token, Variant, theme_from_json, theme_from_toml};

#[test]
fn builtin_light_and_dark_resolve_text_primary_differently() {
    let light = Theme::builtin(Variant::Light);
    let dark = Theme::builtin(Variant::Dark);
    assert_ne!(
        light.resolve(Token::TextPrimary),
        dark.resolve(Token::TextPrimary)
    );
}

#[test]
fn json_theme_loads_token_colors() -> Result<(), ThemeLoadError> {
    let theme = theme_from_json(
        "{\"variant\":\"light\",\"tokens\":{\"text.primary\":\"112233\"}}",
    )?;
    assert_eq!(
        theme.resolve(Token::TextPrimary),
        Color::Rgb(0x11, 0x22, 0x33)
    );
    Ok(())
}

#[test]
fn toml_theme_loads_token_colors() -> Result<(), ThemeLoadError> {
    let theme = theme_from_toml("variant = \"dark\"\n[tokens]\n\"text.primary\" = \"aabbcc\"\n")?;
    assert_eq!(
        theme.resolve(Token::TextPrimary),
        Color::Rgb(0xaa, 0xbb, 0xcc)
    );
    Ok(())
}

#[test]
fn variant_from_osc11_background_probe() {
    assert_eq!(
        Theme::variant_from_background_dark(Some(true)),
        Variant::Dark
    );
    assert_eq!(
        Theme::variant_from_background_dark(Some(false)),
        Variant::Light
    );
}
