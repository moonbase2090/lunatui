// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Color;
use lunatui_theme::{Profile, ThemeLoadError, Token, load_bundled};

#[test]
fn bundled_theme_profiles_downsample_tokens() -> Result<(), ThemeLoadError> {
    let theme = load_bundled("catppuccin-mocha")?;
    for profile in [Profile::Ansi256, Profile::Ansi16, Profile::Mono] {
        let palette = theme.palette_for(profile);
        assert_eq!(palette.len(), Token::COUNT);
        for color in palette {
            match profile {
                Profile::Ansi256 => assert!(matches!(color, Color::Indexed(_))),
                Profile::Ansi16 => assert!(matches!(color, Color::Indexed(index) if index < 16)),
                Profile::Mono => assert!(matches!(color, Color::Indexed(0 | 15))),
                Profile::TrueColor => {}
            }
        }
    }
    Ok(())
}
