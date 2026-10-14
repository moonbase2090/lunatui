// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Size, StatusDot, StatusState, Widget};

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
