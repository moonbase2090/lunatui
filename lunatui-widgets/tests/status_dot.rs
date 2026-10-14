// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Size, StatusDot, StatusState, Widget};

fn snapshot_at(
    state: StatusState,
    variant: Variant,
    width: u16,
    height: u16,
    x: u16,
    y: u16,
) -> String {
    let theme = Theme::builtin(variant);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let dot = StatusDot::new(state);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    dot.render(Rect::new(x, y, 1, 1), &mut cx);
    grid.styled_snapshot(&clusters)
}

fn token_for(state: StatusState) -> Token {
    match state {
        StatusState::Ok => Token::Ok,
        StatusState::Fail => Token::Fail,
        StatusState::Idle => Token::TextPrimary,
    }
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
fn styled_snapshots_differ_across_states() {
    let ok = snapshot_at(StatusState::Ok, Variant::Dark, 1, 1, 0, 0);
    let fail = snapshot_at(StatusState::Fail, Variant::Dark, 1, 1, 0, 0);
    let idle = snapshot_at(StatusState::Idle, Variant::Dark, 1, 1, 0, 0);
    assert_ne!(ok, fail);
    assert_ne!(ok, idle);
    assert_ne!(fail, idle);
}

#[test]
fn every_state_truecolor_at_two_sizes() {
    for state in [StatusState::Ok, StatusState::Fail, StatusState::Idle] {
        for variant in [Variant::Dark, Variant::Light] {
            let theme = Theme::builtin(variant);
            let expected = format!("{:?}", theme.resolve(token_for(state)));
            for (width, height, x, y) in [(1_u16, 1_u16, 0, 0), (80, 24, 79, 23)] {
                let snap = snapshot_at(state, variant, width, height, x, y);
                assert!(snap.contains('●'), "missing glyph at {width}x{height}");
                assert!(
                    snap.contains(expected.as_str()),
                    "state {state:?} variant {variant:?} at {width}x{height}"
                );
            }
        }
    }
}

#[test]
fn renders_on_80_by_24_area() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(80, 24);
    let mut clusters = ClusterTable::new();
    let dot = StatusDot::new(StatusState::Ok);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    dot.render(Rect::new(79, 23, 1, 1), &mut cx);
    let cell = grid.get(79, 23).ok_or("missing cell")?;
    let ch = char::from_u32(cell.glyph().bits()).ok_or("invalid glyph")?;
    assert_eq!(ch, '●');
    assert_eq!(cell.style().fg, theme.resolve(Token::Ok));
    Ok(())
}
