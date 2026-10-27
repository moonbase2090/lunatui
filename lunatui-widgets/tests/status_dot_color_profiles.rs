// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid, Rect};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{RenderCx, StatusDot, StatusState, Widget};

#[test]
fn status_dot_renders_prepared_ansi16_for_every_state_and_size() -> Result<(), &'static str> {
    for state in [StatusState::Ok, StatusState::Fail, StatusState::Idle] {
        for variant in [Variant::Dark, Variant::Light] {
            let mut theme = Theme::builtin(variant);
            theme.set_token(Token::Ok, Color::Rgb(0, 205, 0));
            theme.set_token(Token::Fail, Color::Rgb(205, 0, 0));
            let text = if variant == Variant::Dark {
                Color::Rgb(229, 229, 229)
            } else {
                Color::Rgb(0, 0, 0)
            };
            theme.set_token(Token::TextPrimary, text);
            let index = if state == StatusState::Ok {
                2
            } else if state == StatusState::Fail {
                1
            } else if variant == Variant::Dark {
                7
            } else {
                0
            };
            let palette = theme.prepare(Profile::Ansi16);
            for (width, height, x, y) in [(1, 1, 0, 0), (80, 24, 79, 23)] {
                let mut grid = Grid::new(width, height);
                let mut clusters = ClusterTable::new();
                let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).palette(&palette);
                assert_eq!(cx.color_profile(), Some(Profile::Ansi16));
                StatusDot::new(state).render(Rect::new(x, y, 1, 1), &mut cx);
                let cell = grid.get(x, y).ok_or("missing dot")?;
                assert_eq!(cell.style().fg, Color::Indexed(index));
                assert_eq!(cell.glyph().decode(), DecodedGlyph::Inline('●'));
            }
        }
    }
    Ok(())
}
