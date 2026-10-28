// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RadioFlow, RadioGroup, RenderCx, Widget, WidgetState};

fn sizes() -> RadioGroup {
    RadioGroup::new(["Small", "Medium", "Large"])
}

fn grid_for(group: &RadioGroup, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    group.render(area, &mut cx);
    (grid, clusters)
}

fn rows(group: &RadioGroup, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = grid_for(group, Rect::new(0, 0, width, height), width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn options_stack_one_per_row_with_the_selected_one_filled() {
    assert_eq!(
        rows(&sizes().select(Some(1)), 10, 4),
        ["○ Small   ", "◉ Medium  ", "○ Large   ", "          "]
    );
    assert_eq!(
        rows(&sizes(), 10, 3),
        ["○ Small   ", "○ Medium  ", "○ Large   "]
    );
}

#[test]
fn a_gap_leaves_blank_rows_between_options() {
    assert_eq!(
        rows(&sizes().select(Some(2)).gap(1), 9, 5),
        [
            "○ Small  ",
            "         ",
            "○ Medium ",
            "         ",
            "◉ Large  "
        ]
    );
}

#[test]
fn horizontal_options_sit_side_by_side_separated_by_the_gap() {
    let group = sizes().select(Some(1)).flow(RadioFlow::Horizontal).gap(2);
    assert_eq!(rows(&group, 28, 1), ["○ Small  ◉ Medium  ○ Large  "]);
    let tight = sizes().flow(RadioFlow::Horizontal);
    assert_eq!(rows(&tight, 22, 1), ["○ Small○ Medium○ Large"]);
}

#[test]
fn a_selection_past_the_last_option_selects_nothing() {
    assert_eq!(
        rows(&sizes().select(Some(7)), 10, 3),
        ["○ Small   ", "○ Medium  ", "○ Large   "]
    );
}

#[test]
fn options_that_do_not_fit_are_left_out_and_labels_are_clipped() {
    assert_eq!(rows(&sizes(), 10, 2), ["○ Small   ", "○ Medium  "]);
    assert_eq!(rows(&sizes(), 5, 3), ["○ Sma", "○ Med", "○ Lar"]);
    assert_eq!(rows(&sizes(), 2, 1), ["○ "]);
    assert_eq!(rows(&sizes(), 1, 1), ["○"]);
    let group = sizes().flow(RadioFlow::Horizontal).gap(1);
    assert_eq!(rows(&group, 11, 1), ["○ Small ○ M"]);
    assert_eq!(rows(&group, 8, 1), ["○ Small "]);
}

#[test]
fn option_areas_cover_the_glyph_the_blank_and_the_label() {
    assert_eq!(
        sizes().option_areas(Rect::new(2, 1, 20, 5)),
        [
            (0, Rect::new(2, 1, 7, 1)),
            (1, Rect::new(2, 2, 8, 1)),
            (2, Rect::new(2, 3, 7, 1)),
        ]
    );
    let across = sizes().flow(RadioFlow::Horizontal).gap(3);
    assert_eq!(
        across.option_areas(Rect::new(0, 4, 40, 2)),
        [
            (0, Rect::new(0, 4, 7, 1)),
            (1, Rect::new(10, 4, 8, 1)),
            (2, Rect::new(21, 4, 7, 1)),
        ]
    );
}

#[test]
fn option_areas_follow_the_gap_and_the_edges() {
    let gapped = sizes().gap(1);
    assert_eq!(
        gapped.option_areas(Rect::new(0, 0, 20, 4)),
        [(0, Rect::new(0, 0, 7, 1)), (1, Rect::new(0, 2, 8, 1))]
    );
    let across = sizes().flow(RadioFlow::Horizontal);
    assert_eq!(
        across.option_areas(Rect::new(0, 0, 10, 1)),
        [(0, Rect::new(0, 0, 7, 1)), (1, Rect::new(7, 0, 3, 1))]
    );
    assert_eq!(sizes().option_areas(Rect::new(0, 0, 0, 3)), []);
    assert_eq!(sizes().option_areas(Rect::new(0, 0, 5, 0)), []);
    assert_eq!(
        RadioGroup::new(Vec::<String>::new()).option_areas(Rect::new(0, 0, 5, 5)),
        []
    );
}

#[test]
fn wide_labels_count_by_display_width() {
    let group = RadioGroup::new(["日本語", "ab"]);
    assert_eq!(
        group.option_areas(Rect::new(0, 0, 20, 2))[0].1,
        Rect::new(0, 0, 8, 1)
    );
    assert_eq!(
        rows(&group.select(Some(1)), 10, 2),
        ["○ 日本語  ", "◉ ab      "]
    );
}

#[test]
fn step_moves_forward_and_back_and_wraps() {
    let group = sizes();
    assert_eq!(group.step(Some(0), true), Some(1));
    assert_eq!(group.step(Some(1), true), Some(2));
    assert_eq!(group.step(Some(2), true), Some(0));
    assert_eq!(group.step(Some(2), false), Some(1));
    assert_eq!(group.step(Some(0), false), Some(2));
    assert_eq!(group.step(None, true), Some(0));
    assert_eq!(group.step(None, false), Some(2));
    assert_eq!(group.step(Some(3), true), Some(0));
    assert_eq!(group.step(Some(3), false), Some(2));
    assert_eq!(group.step(Some(9), true), Some(0));
    assert_eq!(group.step(Some(9), false), Some(2));
}

#[test]
fn step_skips_disabled_options_and_gives_none_when_nothing_can_be_chosen() {
    let group = sizes().disabled_options([1]);
    assert_eq!(group.step(Some(0), true), Some(2));
    assert_eq!(group.step(Some(2), false), Some(0));
    let only_one = sizes().disabled_options([0, 2]);
    assert_eq!(only_one.step(Some(1), true), Some(1));
    assert_eq!(only_one.step(None, false), Some(1));
    let none = sizes().disabled_options([0, 1, 2]);
    assert_eq!(none.step(Some(0), true), None);
    assert_eq!(none.step(None, false), None);
    assert_eq!(RadioGroup::new(Vec::<String>::new()).step(None, true), None);
}

#[test]
fn len_and_is_empty_count_the_options() {
    assert_eq!(sizes().len(), 3);
    assert!(!sizes().is_empty());
    assert!(RadioGroup::new(Vec::<String>::new()).is_empty());
}

#[test]
fn colors_follow_selection_focus_and_the_theme() {
    let group = sizes().select(Some(1)).focus(Some(2));
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 0, 1).fg, color(Token::Accent));
    assert_eq!(style_at(&grid, 2, 1).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 0, 2).fg, color(Token::FocusRing));
    assert!(style_at(&grid, 0, 2).attrs.bold());
    assert!(style_at(&grid, 4, 2).attrs.bold());
    assert!(!style_at(&grid, 0, 1).attrs.bold());
    assert!(!style_at(&grid, 4, 0).attrs.bold());
}

#[test]
fn a_focused_selected_option_shows_the_focus_ring() {
    let group = sizes().select(Some(0)).focus(Some(0));
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::FocusRing));
}

#[test]
fn disabled_options_and_groups_are_dimmed_and_lose_focus() {
    let group = sizes().disabled_options([1]).focus(Some(1));
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    assert!(style_at(&grid, 0, 1).attrs.dim());
    assert!(style_at(&grid, 4, 1).attrs.dim());
    assert!(!style_at(&grid, 0, 1).attrs.bold());
    assert_eq!(style_at(&grid, 0, 1).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 0, 0).attrs.dim());

    let all = sizes().state(WidgetState::Disabled);
    let (grid, _) = grid_for(&all, Rect::new(0, 0, 10, 3), 10, 3);
    for y in 0..3 {
        assert!(style_at(&grid, 0, y).attrs.dim(), "row {y}");
        assert!(style_at(&grid, 4, y).attrs.dim(), "row {y}");
    }
}

#[test]
fn an_explicit_style_replaces_the_theme_colors() {
    let style = Style::new().fg(Color::Rgb(1, 2, 3));
    let group = sizes().select(Some(0)).focus(Some(1)).style(style);
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    for (x, y) in [(0, 0), (4, 0), (0, 1), (4, 1), (0, 2)] {
        assert_eq!(style_at(&grid, x, y), style, "({x}, {y})");
    }
}

#[test]
fn nothing_is_drawn_outside_the_area() {
    let group = sizes().select(Some(0));
    let (grid, clusters) = grid_for(&group, Rect::new(2, 1, 6, 2), 12, 5);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(grid.row_display(1, &clusters), "  ◉ Smal    ");
    assert_eq!(grid.row_display(2, &clusters), "  ○ Medi    ");
    assert_eq!(grid.row_display(3, &clusters).trim(), "");
}

#[test]
fn empty_areas_draw_nothing() {
    let (grid, clusters) = grid_for(&sizes(), Rect::new(0, 0, 0, 3), 8, 3);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = grid_for(&sizes(), Rect::new(0, 0, 8, 0), 8, 3);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
}

#[test]
fn measure_is_the_bounding_box_of_the_options() {
    assert_eq!(sizes().measure(Size::new(20, 10)), Size::new(8, 3));
    assert_eq!(sizes().gap(1).measure(Size::new(20, 10)), Size::new(8, 5));
    assert_eq!(sizes().measure(Size::new(20, 2)), Size::new(8, 2));
    assert_eq!(sizes().measure(Size::new(5, 10)), Size::new(5, 3));
    let across = sizes().flow(RadioFlow::Horizontal).gap(2);
    assert_eq!(across.measure(Size::new(40, 10)), Size::new(26, 1));
    assert_eq!(across.measure(Size::new(10, 10)), Size::new(10, 1));
    assert_eq!(sizes().measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(sizes().measure(Size::new(5, 0)), Size::new(0, 0));
    assert_eq!(
        RadioGroup::new(Vec::<String>::new()).measure(Size::new(5, 5)),
        Size::new(0, 0)
    );
}
