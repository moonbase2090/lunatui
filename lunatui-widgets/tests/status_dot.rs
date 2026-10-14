// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Size, StatusDot, StatusState, Widget};

fn snapshot_for(state: StatusState, variant: Variant) -> String {
    let theme = Theme::builtin(variant);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let dot = StatusDot::new(state);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    dot.render(Rect::new(0, 0, 1, 1), &mut cx);
    grid.styled_snapshot(&clusters)
}

#[test]
fn measure_is_one_cell() {
    let dot = StatusDot::new(StatusState::Ok);
    assert_eq!(dot.measure(Size::new(80, 24)), Size::new(1, 1));
}

#[test]
fn measure_clamps_to_available() {
    let dot = StatusDot::new(StatusState::Ok);
    assert_eq!(dot.measure(Size::new(0, 0)), Size::new(0, 0));
}

#[test]
fn renders_bullet_glyph() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(1, 0, 1, 1);
    let dot = StatusDot::new(StatusState::Ok);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    dot.render(area, &mut cx);
    let cell = grid.get(1, 0).ok_or("missing cell")?;
    let ch = char::from_u32(cell.glyph().bits()).ok_or("invalid glyph")?;
    assert_eq!(ch, '●');
    Ok(())
}

#[test]
fn ok_state_styled_snapshot_uses_ok_token_dark() {
    let theme = Theme::builtin(Variant::Dark);
    let snap = snapshot_for(StatusState::Ok, Variant::Dark);
    assert!(snap.contains('●'));
    assert!(snap.contains(format!("{:?}", theme.resolve(Token::Ok)).as_str()));
}

#[test]
fn fail_state_styled_snapshot_uses_fail_token_dark() {
    let theme = Theme::builtin(Variant::Dark);
    let snap = snapshot_for(StatusState::Fail, Variant::Dark);
    assert!(snap.contains(format!("{:?}", theme.resolve(Token::Fail)).as_str()));
}

#[test]
fn idle_state_styled_snapshot_uses_text_primary_dark() {
    let theme = Theme::builtin(Variant::Dark);
    let snap = snapshot_for(StatusState::Idle, Variant::Dark);
    assert!(snap.contains(format!("{:?}", theme.resolve(Token::TextPrimary)).as_str()));
}

#[test]
fn ok_state_styled_snapshot_uses_ok_token_light() {
    let theme = Theme::builtin(Variant::Light);
    let snap = snapshot_for(StatusState::Ok, Variant::Light);
    assert!(snap.contains(format!("{:?}", theme.resolve(Token::Ok)).as_str()));
}

#[test]
fn styled_snapshots_differ_across_states() {
    let ok = snapshot_for(StatusState::Ok, Variant::Dark);
    let fail = snapshot_for(StatusState::Fail, Variant::Dark);
    let idle = snapshot_for(StatusState::Idle, Variant::Dark);
    assert_ne!(ok, fail);
    assert_ne!(ok, idle);
    assert_ne!(fail, idle);
}

#[test]
fn renders_on_80_by_24_area() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(80, 24);
    let mut clusters = ClusterTable::new();
    let dot = StatusDot::new(StatusState::Ok);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    dot.render(Rect::new(79, 23, 1, 1), &mut cx);
    let cell = grid.get(79, 23).expect("cell");
    let ch = char::from_u32(cell.glyph().bits()).expect("glyph");
    assert_eq!(ch, '●');
    assert_eq!(cell.style().fg, theme.resolve(Token::Ok));
}
