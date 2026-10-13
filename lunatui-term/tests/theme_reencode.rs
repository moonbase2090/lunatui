// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};

#[test]
fn theme_switch_reencodes_without_grid_rerender() {
    let mut grid = Grid::new(8, 3);
    let mut clusters = ClusterTable::new();
    let style = Style::new().fg(Color::Token(Token::TextPrimary.id()));
    grid.put_str(1, 1, style, "x", &mut clusters);
    let prev = Grid::new(8, 3);
    let mut encoder = Encoder::new();
    encoder.set_token_palette(Theme::builtin(Variant::Light).palette());
    let light = encoder
        .encode_diff(&prev, &grid, &clusters, ColorMode::TrueColor)
        .to_vec();
    encoder.set_token_palette(Theme::builtin(Variant::Dark).palette());
    let dark = encoder
        .encode_diff(&prev, &grid, &clusters, ColorMode::TrueColor)
        .to_vec();
    assert_ne!(light.as_slice(), &[] as &[u8]);
    assert_ne!(dark.as_slice(), &[] as &[u8]);
    assert_ne!(light, dark);
    assert_eq!(
        grid.get(1, 1).map(|cell| cell.style().fg),
        Some(Color::Token(Token::TextPrimary.id()))
    );
}

#[test]
fn palette_change_requires_cleared_previous_grid() {
    let mut grid = Grid::new(8, 3);
    let mut clusters = ClusterTable::new();
    let style = Style::new().fg(Color::Token(Token::TextPrimary.id()));
    grid.put_str(1, 1, style, "x", &mut clusters);
    let prev = grid.clone();
    let mut encoder = Encoder::new();
    encoder.set_token_palette(Theme::builtin(Variant::Light).palette());
    let light = encoder
        .encode_diff(&prev, &grid, &clusters, ColorMode::TrueColor)
        .to_vec();
    assert_ne!(light.as_slice(), &[] as &[u8]);
    let fresh_prev = Grid::new(8, 3);
    encoder.set_token_palette(Theme::builtin(Variant::Dark).palette());
    let dark = encoder
        .encode_diff(&fresh_prev, &grid, &clusters, ColorMode::TrueColor)
        .to_vec();
    assert_ne!(dark, light);
}
