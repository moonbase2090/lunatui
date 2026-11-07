// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::{Constraint, Layout};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{LayoutDebug, RenderCx, Widget};

const PAGE: Color = Color::Rgb(10, 20, 30);

fn render(widget: &LayoutDebug, width: u16, rows: u16, area: Rect) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, rows);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.fill(Rect::new(0, 0, width, rows), Style::new().bg(PAGE), '.');
    widget.render(area, &mut cx);
    (grid, clusters)
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .unwrap_or_default()
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|y| grid.row_display(y, clusters))
        .collect()
}

#[test]
fn outlines_a_box_and_labels_its_size() {
    let overlay = LayoutDebug::new().rect(Rect::new(1, 1, 8, 4));
    let (grid, clusters) = render(&overlay, 10, 6, Rect::new(0, 0, 10, 6));
    assert_eq!(
        rows(&grid, &clusters),
        [
            "..........",
            ".┌8x4───┐.",
            ".│......│.",
            ".│......│.",
            ".└──────┘.",
            "..........",
        ]
    );
}

#[test]
fn names_come_before_the_size() {
    let overlay = LayoutDebug::new().named("nav", Rect::new(0, 0, 12, 3));
    let (grid, clusters) = render(&overlay, 12, 3, Rect::new(0, 0, 12, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌nav 12x3──┐");
}

#[test]
fn label_is_dropped_when_it_does_not_fit() {
    let overlay = LayoutDebug::new().rect(Rect::new(0, 0, 5, 3));
    let (grid, clusters) = render(&overlay, 5, 3, Rect::new(0, 0, 5, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌5x3┐");
    let overlay = LayoutDebug::new().rect(Rect::new(0, 0, 4, 3));
    let (grid, clusters) = render(&overlay, 4, 3, Rect::new(0, 0, 4, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌──┐");
    let overlay = LayoutDebug::new().rect(Rect::new(0, 0, 6, 3));
    let (grid, clusters) = render(&overlay, 6, 3, Rect::new(0, 0, 6, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌6x3─┐");
}

#[test]
fn outline_keeps_the_background_and_interior() {
    let overlay = LayoutDebug::new().rect(Rect::new(0, 0, 6, 3));
    let (grid, _) = render(&overlay, 6, 3, Rect::new(0, 0, 6, 3));
    for (x, y) in [(0, 0), (3, 0), (0, 1), (5, 2), (2, 2)] {
        assert_eq!(style_at(&grid, x, y).bg, PAGE, "({x},{y})");
    }
    assert_eq!(style_at(&grid, 2, 1).fg, Color::Default);
}

#[test]
fn neighbors_get_different_colors_from_theme_tokens() {
    let theme = Theme::builtin(Variant::Dark);
    let overlay = LayoutDebug::new()
        .rect(Rect::new(0, 0, 4, 3))
        .rect(Rect::new(4, 0, 4, 3))
        .rect(Rect::new(8, 0, 4, 3))
        .rect(Rect::new(12, 0, 4, 3))
        .rect(Rect::new(16, 0, 4, 3));
    let (grid, _) = render(&overlay, 20, 3, Rect::new(0, 0, 20, 3));
    let fg = |x: u16| style_at(&grid, x, 2).fg;
    assert_eq!(fg(0), theme.resolve(Token::Accent));
    assert_eq!(fg(4), theme.resolve(Token::Ok));
    assert_eq!(fg(8), theme.resolve(Token::Warn));
    assert_eq!(fg(12), theme.resolve(Token::Fail));
    assert_eq!(fg(16), theme.resolve(Token::Accent));
}

#[test]
fn enabled_switches_the_overlay_and_keeps_the_boxes() {
    let on = LayoutDebug::new().rect(Rect::new(0, 0, 6, 3));
    assert_ne!(on, on.clone().enabled(false));
    assert_eq!(on, on.clone().enabled(false).enabled(true));
}

#[test]
fn disabled_overlay_draws_nothing() {
    let overlay = LayoutDebug::new()
        .rect(Rect::new(0, 0, 6, 3))
        .enabled(false);
    let (grid, clusters) = render(&overlay, 6, 3, Rect::new(0, 0, 6, 3));
    assert_eq!(rows(&grid, &clusters), ["......", "......", "......"]);
}

#[test]
fn degenerate_boxes_draw_lines_and_a_dot() {
    let overlay = LayoutDebug::new()
        .rect(Rect::new(0, 0, 5, 1))
        .rect(Rect::new(0, 1, 1, 3))
        .rect(Rect::new(3, 2, 1, 1))
        .rect(Rect::new(4, 2, 0, 3))
        .rect(Rect::new(4, 2, 3, 0));
    let (grid, clusters) = render(&overlay, 7, 4, Rect::new(0, 0, 7, 4));
    assert_eq!(
        rows(&grid, &clusters),
        ["─────..", "│......", "│..▪...", "│......"]
    );
}

#[test]
fn nothing_is_drawn_outside_the_render_area() {
    let overlay = LayoutDebug::new().rect(Rect::new(2, 1, 10, 6));
    let (grid, clusters) = render(&overlay, 12, 6, Rect::new(2, 1, 5, 3));
    assert_eq!(
        rows(&grid, &clusters),
        [
            "............",
            "..┌10x6.....",
            "..│.........",
            "..│.........",
            "............",
            "............",
        ]
    );
}

#[test]
fn boxes_past_the_grid_edge_do_not_panic() {
    let overlay = LayoutDebug::new().rect(Rect::new(8, 4, 40, 40));
    let (grid, clusters) = render(&overlay, 10, 6, Rect::new(0, 0, 10, 6));
    assert_eq!(rows(&grid, &clusters)[4], "........┌4");
    let overlay = LayoutDebug::new().rect(Rect::new(u16::MAX - 1, u16::MAX - 1, 9, 9));
    let _ = render(&overlay, 10, 6, Rect::new(0, 0, 10, 6));
}

#[test]
fn empty_render_area_draws_nothing() {
    let overlay = LayoutDebug::new().rect(Rect::new(0, 0, 6, 3));
    let (grid, clusters) = render(&overlay, 6, 3, Rect::new(0, 0, 0, 3));
    assert_eq!(rows(&grid, &clusters), ["......", "......", "......"]);
}

#[test]
fn solved_segments_become_boxes_and_spacers_do_not() {
    let solved = Layout::horizontal([Constraint::Length(6), Constraint::Length(6)])
        .spacing(2)
        .split(Rect::new(0, 0, 14, 3));
    let overlay = LayoutDebug::new().solved(&solved);
    let (grid, clusters) = render(&overlay, 14, 3, Rect::new(0, 0, 14, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌6x3─┐..┌6x3─┐");
}

#[test]
fn rects_adds_each_in_order() {
    let overlay = LayoutDebug::new().rects([Rect::new(0, 0, 6, 3), Rect::new(6, 0, 6, 3)]);
    let (grid, clusters) = render(&overlay, 12, 3, Rect::new(0, 0, 12, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌6x3─┐┌6x3─┐");
}

#[test]
fn default_is_enabled_and_empty() {
    assert_eq!(LayoutDebug::default(), LayoutDebug::new());
    let (grid, clusters) = render(&LayoutDebug::new(), 4, 2, Rect::new(0, 0, 4, 2));
    assert_eq!(rows(&grid, &clusters), ["....", "...."]);
}

#[test]
fn takes_no_space() {
    let overlay = LayoutDebug::new().rect(Rect::new(0, 0, 6, 3));
    assert_eq!(
        overlay.measure(lunatui_core::Size::new(40, 10)),
        lunatui_core::Size::new(0, 0)
    );
}

#[test]
fn huge_boxes_cost_only_the_visible_edges() {
    let overlay = LayoutDebug::new()
        .rect(Rect::new(0, 0, u16::MAX, u16::MAX))
        .rect(Rect::new(3, 1, u16::MAX, u16::MAX));
    let (grid, clusters) = render(&overlay, 12, 5, Rect::new(2, 1, 8, 3));
    assert_eq!(rows(&grid, &clusters)[0], "............");
    assert_eq!(rows(&grid, &clusters)[1], "...┌65535x..");
}

#[test]
fn wide_names_are_sized_by_display_width() {
    let overlay = LayoutDebug::new().named("日本", Rect::new(0, 0, 11, 3));
    let (grid, clusters) = render(&overlay, 11, 3, Rect::new(0, 0, 11, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌日本 11x3┐");
    let overlay = LayoutDebug::new().named("日本", Rect::new(0, 0, 9, 3));
    let (grid, clusters) = render(&overlay, 9, 3, Rect::new(0, 0, 9, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌───────┐");
}

#[test]
fn wide_label_never_overwrites_the_border() {
    let overlay = LayoutDebug::new().named("日本語", Rect::new(0, 0, 13, 3));
    let (grid, clusters) = render(&overlay, 13, 3, Rect::new(0, 0, 13, 3));
    let top = rows(&grid, &clusters)[0].clone();
    assert!(top.starts_with('┌') && top.ends_with('┐'), "{top}");
    assert_eq!(top, "┌日本語 13x3┐");
}

#[test]
fn wide_character_cut_by_the_render_area_is_not_drawn() {
    let overlay = LayoutDebug::new().named("日本", Rect::new(0, 0, 14, 3));
    let (grid, clusters) = render(&overlay, 14, 3, Rect::new(0, 0, 4, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌日─..........");
}

#[test]
fn control_and_zero_width_characters_are_left_out_of_names() {
    let overlay = LayoutDebug::new().named("a\u{7}\u{301}b\n", Rect::new(0, 0, 10, 3));
    let (grid, clusters) = render(&overlay, 10, 3, Rect::new(0, 0, 10, 3));
    assert_eq!(rows(&grid, &clusters)[0], "┌ab 10x3─┐");
}

#[test]
fn empty_boxes_draw_nothing_even_next_to_other_boxes() {
    for rect in [
        Rect::new(2, 0, 0, 4),
        Rect::new(0, 1, 6, 0),
        Rect::new(0, 0, 0, 0),
    ] {
        let (grid, clusters) = render(&LayoutDebug::new().rect(rect), 6, 4, Rect::new(0, 0, 6, 4));
        assert_eq!(rows(&grid, &clusters), ["......"; 4], "{rect:?}");
    }
}

#[test]
fn two_row_boxes_are_labeled_and_one_row_boxes_are_not() {
    let (grid, clusters) = render(
        &LayoutDebug::new().rect(Rect::new(0, 0, 8, 2)),
        8,
        2,
        Rect::new(0, 0, 8, 2),
    );
    assert_eq!(rows(&grid, &clusters), ["┌8x2───┐", "└──────┘"]);
    let (grid, clusters) = render(
        &LayoutDebug::new().rect(Rect::new(0, 0, 8, 1)),
        8,
        1,
        Rect::new(0, 0, 8, 1),
    );
    assert_eq!(rows(&grid, &clusters), ["────────"]);
}

#[test]
fn narrow_boxes_never_underflow_the_label_room() {
    for width in 1..=4 {
        let rect = Rect::new(0, 0, width, 3);
        let (grid, clusters) = render(&LayoutDebug::new().rect(rect), 4, 3, Rect::new(0, 0, 4, 3));
        assert!(!rows(&grid, &clusters)[0].contains('x'), "width {width}");
    }
}
