// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Token mapping and resolution checks.

use lunatui_core::{ClusterTable, Grid};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_ui_tokens::{Tone, UiToken};
use lunatui_widgets::RenderCx;

#[test]
fn roles_map_onto_theme_tokens() {
    assert_eq!(UiToken::Background.token(), Token::Surface0);
    assert_eq!(UiToken::Foreground.token(), Token::TextPrimary);
    assert_eq!(UiToken::Destructive.token(), Token::Fail);
    assert_eq!(UiToken::Ring.token(), Token::FocusRing);
}

#[test]
fn tones_map_onto_status_tokens() {
    assert_eq!(Tone::Success.token(), Token::Ok);
    assert_eq!(Tone::Warning.token(), Token::Warn);
    assert_eq!(Tone::Destructive.token(), Token::Fail);
    assert_eq!(Tone::default(), Tone::Neutral);
}

#[test]
fn colors_resolve_through_the_frame_theme() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    assert_eq!(UiToken::Primary.color(&cx), theme.resolve(Token::Accent));
    assert_eq!(Tone::Warning.fill(&cx), theme.resolve(Token::Warn));
    assert_eq!(
        Tone::Neutral.on_fill(&cx),
        theme.resolve(Token::TextPrimary)
    );
    assert_eq!(Tone::Success.on_fill(&cx), theme.resolve(Token::Surface0));
}
