// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_test::styled_snapshot;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, FocusTrap, Overlay, RenderCx, Widget, blend_rgb};

#[test]
fn focus_trap_filters_tab_order() {
    let trap = FocusTrap::enabled();
    let scope = [2_u32, 3];
    let order = [1, 2, 3, 4];
    assert_eq!(trap.filter_tab_order(&scope, &order), vec![2, 3]);
}

#[test]
fn blend_rgb_midpoint() {
    let mid = blend_rgb((0, 0, 0), (200, 100, 50), 0.5);
    assert_eq!(mid, (100, 50, 25));
}

#[test]
fn default_background_blends_with_theme_surface() -> Result<(), &'static str> {
    let mut theme = Theme::builtin(Variant::Light);
    theme.set_token(Token::Surface0, Color::Rgb(245, 245, 247));
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "x", Style::new().bg(Color::Default));
    cx.blend_backdrop(Rect::new(0, 0, 3, 1), 0.5, Color::Rgb(0, 0, 0));
    let cell = grid.get(0, 0).ok_or("missing cell")?;
    assert_eq!(cell.style().bg, Color::Rgb(123, 123, 124));
    Ok(())
}

#[test]
fn backdrop_blend_snapshot() -> Result<(), &'static str> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(6, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "hello", Style::new().bg(Color::Rgb(10, 20, 30)));
    cx.blend_backdrop(Rect::new(0, 0, 6, 2), 0.5, Color::Rgb(0, 0, 0));
    let snap = styled_snapshot(&grid, &clusters);
    assert!(snap.contains("hello"));
    let cell = grid.get(0, 0).ok_or("missing cell")?;
    let bg = cell.style().bg;
    assert_eq!(bg, Color::Rgb(5, 10, 15));
    Ok(())
}

#[test]
fn overlay_renders_modal_block() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 10);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "under", Style::new());
    let modal = Block::bordered().title("modal");
    Overlay::new(modal, Rect::new(4, 2, 12, 5)).render(Rect::new(0, 0, 20, 10), &mut cx);
    assert!(grid.row_display(2, &clusters).contains('┌'));
}

#[test]
fn shadow_sits_one_cell_right_and_down_behind_the_modal() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(8, 5);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    for y in 0..5 {
        cx.put_str(0, y, "........", Style::new());
    }
    let shadow = Color::Rgb(4, 5, 11);
    let panel = Color::Rgb(22, 28, 61);
    Overlay::new(
        Block::new().style(Style::new().bg(panel)),
        Rect::new(2, 1, 4, 2),
    )
    .backdrop(0.0, Color::Rgb(0, 0, 0))
    .shadow(Style::new().bg(shadow))
    .render(Rect::new(0, 0, 8, 5), &mut cx);
    let rows: Vec<String> = (0..5).map(|y| grid.row_display(y, &clusters)).collect();
    assert_eq!(
        rows,
        ["........", "..    ..", "..     .", "...    .", "........"]
    );
    let bg = |x, y| grid.get(x, y).map(|cell| cell.style().bg);
    assert_eq!(bg(2, 1), Some(panel));
    assert_eq!(bg(6, 2), Some(shadow), "right edge of the shadow");
    assert_eq!(bg(3, 3), Some(shadow), "bottom edge of the shadow");
    assert_eq!(bg(6, 3), Some(shadow), "shadow corner");
    assert_ne!(bg(6, 1), Some(shadow), "no shadow above the offset");
    assert_ne!(bg(2, 3), Some(shadow), "no shadow left of the offset");
}
