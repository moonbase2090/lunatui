// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Color;
use lunatui_theme::{
    BUNDLED_COUNT, ThemeLoadError, Token, bundled_catalog, load_bundled, theme_from_json_strict,
};

#[test]
fn bundled_catalog_matches_prismattyc_host_count() {
    assert_eq!(BUNDLED_COUNT, 29);
    assert_eq!(bundled_catalog().len(), BUNDLED_COUNT);
}

#[test]
fn every_bundled_theme_parses_and_resolves_all_tokens() -> Result<(), ThemeLoadError> {
    for entry in bundled_catalog() {
        let theme = load_bundled(entry.id)?;
        for token in Token::all() {
            let color = theme.resolve(token);
            assert!(
                matches!(color, Color::Rgb(_, _, _)),
                "{} token {:?} did not resolve to rgb",
                entry.id,
                token
            );
        }
    }
    Ok(())
}

#[test]
fn bundled_prismattyc_dark_loads_from_json_strict() -> Result<(), ThemeLoadError> {
    let toml = load_bundled("prismattyc-dark")?;
    let json = r#"{
      "variant": "dark",
      "tokens": {
        "surface.0": "181b21",
        "surface.1": "1a1d23",
        "text.primary": "d9dde4",
        "accent": "2a3a55",
        "ok": "4cc98a",
        "warn": "f2b84b",
        "fail": "ff7a6b",
        "focus.ring": "e6e9ee"
      }
    }"#;
    let from_json = theme_from_json_strict(json)?;
    assert_eq!(toml.palette(), from_json.palette());
    Ok(())
}
