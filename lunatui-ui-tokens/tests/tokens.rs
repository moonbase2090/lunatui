// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_ui_tokens::{Role, Spacing, Tone, surface, text};
use lunatui_widgets::RenderCx;

#[test]
fn roles_map_to_core_tokens() {
    assert_eq!(Role::Card.token(), Token::SurfaceRaised);
    assert_eq!(Role::Border.token(), Token::Border);
    assert_eq!(Role::Destructive.token(), Token::Fail);
    assert_eq!(Role::Ring.token(), Token::FocusRing);
}

#[test]
fn tone_style_resolves_through_the_theme() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let style = Tone::Danger.style(&cx);
    assert_eq!(style.bg, cx.token(Token::Fail));
    assert_eq!(style.fg, cx.token(Token::OnDanger));
    assert_eq!(text(&cx, Role::Muted).fg, cx.token(Token::TextMuted));
    assert_eq!(surface(&cx, Role::Card).bg, cx.token(Token::SurfaceRaised));
}

#[test]
fn spacing_doubles_horizontally() {
    assert_eq!((Spacing::S2.y(), Spacing::S2.x()), (2, 4));
}
