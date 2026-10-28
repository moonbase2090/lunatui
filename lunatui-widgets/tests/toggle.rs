// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Toggle, Widget, WidgetState};

fn draw(toggle: &Toggle, width: u16) -> (Grid, ClusterTable) {
    draw_in(toggle, Rect::new(0, 0, width, 1), width)
}

fn draw_in(toggle: &Toggle, area: Rect, grid_width: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(grid_width, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    toggle.render(area, &mut cx);
    (grid, clusters)
}

fn row(toggle: &Toggle, width: u16) -> String {
    let (grid, clusters) = draw(toggle, width);
    grid.row_display(0, &clusters)
}

fn style_at(grid: &Grid, x: u16) -> Style {
    grid.get(x, 0).map(Cell::style).unwrap_or_default()
}

fn token(theme: &Theme, token: Token) -> Color {
    theme.resolve(token)
}

#[test]
fn an_off_switch_has_its_knob_on_the_left() {
    assert_eq!(row(&Toggle::new(false), 6), "●───  ");
}

#[test]
fn an_on_switch_has_its_knob_on_the_right() {
    assert_eq!(row(&Toggle::new(true), 6), "━━━●  ");
}

#[test]
fn the_label_follows_one_cell_after_the_track() {
    assert_eq!(
        row(&Toggle::new(true).label("Dark mode"), 16),
        "━━━● Dark mode  "
    );
    assert_eq!(row(&Toggle::new(false).label("Sync"), 10), "●─── Sync ");
}

#[test]
fn the_track_width_is_configurable_with_a_minimum_of_two() {
    assert_eq!(row(&Toggle::new(true).track_width(6), 8), "━━━━━●  ");
    assert_eq!(row(&Toggle::new(false).track_width(2), 4), "●─  ");
    assert_eq!(row(&Toggle::new(true).track_width(0), 4), "━●  ");
    assert_eq!(row(&Toggle::new(false).track_width(1), 4), "●─  ");
}

#[test]
fn toggled_flips_the_switch_and_is_on_reports_it() {
    let off = Toggle::new(false);
    assert!(!off.is_on());
    let on = off.clone().toggled();
    assert!(on.is_on());
    assert_eq!(row(&on, 4), "━━━●");
    assert_eq!(on.toggled(), off);
}

#[test]
fn the_switch_and_label_are_clipped_to_the_area() {
    assert_eq!(row(&Toggle::new(true).label("Long label"), 3), "━━━");
    assert_eq!(row(&Toggle::new(true).label("Long label"), 4), "━━━●");
    assert_eq!(row(&Toggle::new(true).label("Long label"), 5), "━━━● ");
    assert_eq!(row(&Toggle::new(true).label("Long label"), 7), "━━━● Lo");
    let (grid, clusters) = draw_in(
        &Toggle::new(true).label("Long label"),
        Rect::new(1, 0, 6, 1),
        12,
    );
    assert_eq!(grid.row_display(0, &clusters), " ━━━● L     ");
}

#[test]
fn a_narrow_area_cuts_the_track() {
    assert_eq!(row(&Toggle::new(true), 2), "━━");
    assert_eq!(row(&Toggle::new(false), 1), "●");
}

#[test]
fn only_the_first_row_of_a_tall_area_is_used() {
    let (grid, clusters) = draw_in(&Toggle::new(true), Rect::new(0, 0, 6, 2), 6);
    assert_eq!(grid.row_display(0, &clusters), "━━━●  ");
    assert_eq!(grid.row_display(1, &clusters), "      ");
}

#[test]
fn empty_areas_draw_nothing_and_measure_zero() {
    let toggle = Toggle::new(true).label("x");
    let (grid, clusters) = draw_in(&toggle, Rect::new(0, 0, 0, 1), 6);
    assert_eq!(grid.row_display(0, &clusters), "      ");
    let (grid, clusters) = draw_in(&toggle, Rect::new(0, 0, 6, 0), 6);
    assert_eq!(grid.row_display(0, &clusters), "      ");
    assert_eq!(toggle.measure(Size::new(0, 3)), Size::new(0, 0));
    assert_eq!(toggle.measure(Size::new(9, 0)), Size::new(0, 0));
}

#[test]
fn measure_is_the_switch_and_label_on_one_row() {
    assert_eq!(Toggle::new(true).measure(Size::new(20, 5)), Size::new(4, 1));
    assert_eq!(
        Toggle::new(true).label("Sync").measure(Size::new(20, 5)),
        Size::new(9, 1)
    );
    assert_eq!(
        Toggle::new(true).label("Sync").measure(Size::new(6, 5)),
        Size::new(6, 1)
    );
}

#[test]
fn the_hit_area_covers_the_switch_and_its_label() {
    let toggle = Toggle::new(false).label("Sync");
    assert_eq!(
        toggle.hit_area(Rect::new(3, 2, 20, 4)),
        Rect::new(3, 2, 9, 1)
    );
    assert_eq!(
        Toggle::new(false).hit_area(Rect::new(3, 2, 20, 4)),
        Rect::new(3, 2, 4, 1)
    );
    assert_eq!(
        toggle.hit_area(Rect::new(3, 2, 6, 4)),
        Rect::new(3, 2, 6, 1)
    );
    assert_eq!(toggle.hit_area(Rect::new(3, 2, 0, 4)).width, 0);
    assert_eq!(toggle.hit_area(Rect::new(3, 2, 5, 0)).height, 0);
    let wide = Toggle::new(true).label("日本");
    assert_eq!(wide.hit_area(Rect::new(0, 0, 20, 1)), Rect::new(0, 0, 9, 1));
}

#[test]
fn colors_follow_the_state_and_the_theme() {
    let theme = Theme::builtin(Variant::Dark);
    let accent = token(&theme, Token::Accent);
    let ok = token(&theme, Token::Ok);
    let ring = token(&theme, Token::FocusRing);
    let text = token(&theme, Token::TextPrimary);

    let (grid, _) = draw(&Toggle::new(true).label("ab"), 8);
    assert_eq!(style_at(&grid, 0).fg, accent);
    assert_eq!(style_at(&grid, 2).fg, accent);
    assert_eq!(style_at(&grid, 3).fg, ok);
    assert_eq!(style_at(&grid, 5).fg, text);

    let (grid, _) = draw(&Toggle::new(false).label("ab"), 8);
    assert_eq!(style_at(&grid, 0).fg, text);
    assert_eq!(style_at(&grid, 1).fg, text);
    assert!(style_at(&grid, 1).attrs.dim());
    assert!(!style_at(&grid, 0).attrs.dim());
    assert!(!style_at(&grid, 5).attrs.dim());

    let focused = Toggle::new(true).label("ab").state(WidgetState::Focused);
    let (grid, _) = draw(&focused, 8);
    assert_eq!(style_at(&grid, 3).fg, ring);
    assert!(style_at(&grid, 3).attrs.bold());
    assert!(style_at(&grid, 5).attrs.bold());
    assert_eq!(style_at(&grid, 0).fg, accent);
    assert!(!style_at(&grid, 0).attrs.bold());
}

#[test]
fn a_disabled_switch_is_dimmed_everywhere() {
    let disabled = Toggle::new(true).label("ab").state(WidgetState::Disabled);
    let (grid, _) = draw(&disabled, 8);
    for x in [0, 2, 3, 5] {
        assert!(style_at(&grid, x).attrs.dim(), "column {x}");
    }
    let (grid, _) = draw(&Toggle::new(true).label("ab"), 8);
    for x in [0, 2, 3, 5] {
        assert!(!style_at(&grid, x).attrs.dim(), "column {x}");
    }
}

#[test]
fn an_explicit_style_replaces_the_theme_colors() {
    let style = Style::new().fg(Color::Rgb(1, 2, 3));
    let (grid, _) = draw(&Toggle::new(true).label("ab").style(style), 8);
    for x in [0, 3, 5] {
        assert_eq!(style_at(&grid, x), style, "column {x}");
    }
    let (grid, _) = draw(
        &Toggle::new(false)
            .label("ab")
            .style(style)
            .state(WidgetState::Disabled),
        8,
    );
    assert_eq!(style_at(&grid, 1), style);
}
