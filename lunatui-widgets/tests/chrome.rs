// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Gauge, RenderCx, Spinner, StatusBar, Tabs, TextInput, Widget};

#[test]
fn tabs_and_status_render() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Tabs::new(["One", "Two"])
        .select(1)
        .render(Rect::new(0, 0, 20, 1), &mut cx);
    StatusBar::new("ready").render(Rect::new(0, 2, 40, 1), &mut cx);
    assert!(grid.row_display(0, &clusters).contains("Two"));
    assert!(grid.row_display(2, &clusters).contains("ready"));
}

#[test]
fn gauge_and_spinner_render() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Gauge::new(0.5).render(Rect::new(0, 0, 10, 1), &mut cx);
    Spinner::new(3).render(Rect::new(10, 0, 1, 1), &mut cx);
    assert!(grid.row_display(0, &clusters).contains('█'));
}

#[test]
fn text_input_renders_cursor() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    TextInput::new("hi", 2).render(Rect::new(0, 0, 10, 1), &mut cx);
    assert!(grid.row_display(0, &clusters).contains('│'));
}
