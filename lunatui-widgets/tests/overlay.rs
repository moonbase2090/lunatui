// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
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
fn backdrop_blend_snapshot() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(6, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "hello", Style::new().bg(Color::Rgb(10, 20, 30)));
    cx.blend_backdrop(Rect::new(0, 0, 6, 2), 0.5, Color::Rgb(0, 0, 0));
    let snap = grid.styled_snapshot(&clusters);
    assert!(snap.contains("hello"));
    let cell = grid.get(0, 0).expect("cell");
    let bg = cell.style().bg;
    assert_eq!(bg, Color::Rgb(5, 10, 15));
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
