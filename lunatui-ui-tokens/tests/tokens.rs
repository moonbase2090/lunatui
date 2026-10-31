// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_ui_tokens::{Role, Spacing, Tone, surface, text};
use lunatui_widgets::RenderCx;

#[cfg(feature = "graphite")]
use Token::{Border as BORDER, OnDanger as ON_DANGER, SurfaceRaised as RAISED, TextMuted as MUTED};
#[cfg(not(feature = "graphite"))]
use Token::{FocusRing as BORDER, Surface0 as ON_DANGER, Surface1 as RAISED, TextPrimary as MUTED};

#[test]
fn roles_map_to_core_tokens() {
    assert_eq!(Role::Card.token(), RAISED);
    assert_eq!(Role::Border.token(), BORDER);
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
    assert_eq!(style.fg, cx.token(ON_DANGER));
    assert_eq!(text(&cx, Role::Muted).fg, cx.token(MUTED));
    assert_eq!(surface(&cx, Role::Card).bg, cx.token(RAISED));
}

#[test]
fn spacing_doubles_horizontally() {
    assert_eq!((Spacing::S2.y(), Spacing::S2.x()), (2, 4));
}
