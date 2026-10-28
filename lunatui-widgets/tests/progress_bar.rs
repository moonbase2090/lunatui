// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{ProgressBar, ProgressTone, RenderCx, Widget, WidgetState};

fn theme() -> Theme {
    Theme::builtin(Variant::Dark)
}

fn draw(
    bar: &ProgressBar<'_>,
    area: Rect,
    grid_width: u16,
    grid_height: u16,
) -> (Grid, ClusterTable) {
    let theme = theme();
    let mut grid = Grid::new(grid_width, grid_height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    bar.render(area, &mut cx);
    (grid, clusters)
}

fn tag(color: Color) -> char {
    let theme = theme();
    [
        (Token::Accent, 'a'),
        (Token::Ok, 'o'),
        (Token::Warn, 'w'),
        (Token::Fail, 'f'),
        (Token::Surface1, '.'),
    ]
    .into_iter()
    .find(|(token, _)| theme.resolve(*token) == color)
    .map_or('?', |(_, tag)| tag)
}

fn cells(bar: &ProgressBar<'_>, width: u16) -> Vec<String> {
    let (grid, clusters) = draw(bar, Rect::new(0, 0, width, 1), width, 1);
    (0..width)
        .map(|x| {
            let cell = grid.get(x, 0).unwrap_or_default();
            let style = cell.style();
            let glyph = grid
                .row_display(0, &clusters)
                .chars()
                .nth(usize::from(x))
                .unwrap_or(' ');
            let _ = cell;
            if glyph == '█' {
                format!("{}", tag(style.fg))
            } else {
                format!("{glyph}{}{}", tag(style.fg), tag(style.bg))
            }
        })
        .collect()
}

fn shape(bar: &ProgressBar<'_>, width: u16) -> String {
    cells(bar, width).join(" ")
}

#[test]
fn a_ratio_fills_whole_cells_from_the_left() {
    assert_eq!(
        shape(&ProgressBar::new().ratio(0.5), 10),
        "a a a a a . . . . ."
    );
    assert_eq!(shape(&ProgressBar::new().ratio(0.0), 4), ". . . .");
    assert_eq!(shape(&ProgressBar::new().ratio(1.0), 4), "a a a a");
    assert_eq!(shape(&ProgressBar::new(), 4), ". . . .");
}

#[test]
fn the_fill_ends_on_an_eighth_of_a_cell() {
    assert_eq!(
        shape(&ProgressBar::new().ratio(0.55), 10),
        "a a a a a ▌a. . . . ."
    );
    assert_eq!(shape(&ProgressBar::new().ratio(0.125), 4), "▌a. . . .");
    assert_eq!(shape(&ProgressBar::new().ratio(0.03125), 4), "▏a. . . .");
    assert_eq!(shape(&ProgressBar::new().ratio(0.4375), 4), "a ▊a. . .");
}

#[test]
fn ratios_are_clamped_and_nan_counts_as_zero() {
    assert_eq!(shape(&ProgressBar::new().ratio(7.0), 3), "a a a");
    assert_eq!(shape(&ProgressBar::new().ratio(-1.0), 3), ". . .");
    assert_eq!(shape(&ProgressBar::new().ratio(f64::NAN), 3), ". . .");
}

#[test]
fn stacked_segments_follow_each_other_and_share_the_cell_where_they_meet() {
    let bar = ProgressBar::new()
        .segment(0.4, ProgressTone::Ok)
        .segment(0.3, ProgressTone::Warn)
        .segment(0.1, ProgressTone::Fail);
    assert_eq!(shape(&bar, 10), "o o o o w w w f . .");
    let split = ProgressBar::new()
        .segment(0.25, ProgressTone::Ok)
        .segment(0.25, ProgressTone::Fail);
    assert_eq!(shape(&split, 8), "o o f f . . . .");
    let mixed = ProgressBar::new()
        .segment(0.35, ProgressTone::Ok)
        .segment(0.35, ProgressTone::Fail);
    assert_eq!(shape(&mixed, 10), "o o o ▌of f f f . . .");
}

#[test]
fn segments_past_the_end_are_cut_off_and_bad_amounts_count_as_zero() {
    let over = ProgressBar::new()
        .segment(0.7, ProgressTone::Ok)
        .segment(0.7, ProgressTone::Fail);
    assert_eq!(shape(&over, 10), "o o o o o o o f f f");
    let odd = ProgressBar::new()
        .segment(-0.5, ProgressTone::Ok)
        .segment(f64::NAN, ProgressTone::Warn)
        .segment(0.5, ProgressTone::Fail);
    assert_eq!(shape(&odd, 4), "f f . .");
}

#[test]
fn a_new_ratio_replaces_segments_and_segments_replace_a_ratio_mode() {
    let replaced = ProgressBar::new()
        .segment(0.9, ProgressTone::Fail)
        .ratio(0.25);
    assert_eq!(shape(&replaced, 4), "a . . .");
    let stacked = ProgressBar::new()
        .ratio(0.25)
        .segment(0.25, ProgressTone::Ok);
    assert_eq!(shape(&stacked, 4), "a o . .");
}

#[test]
fn an_indeterminate_bar_slides_a_quarter_wide_block_along_the_track() {
    let at = |tick| shape(&ProgressBar::new().indeterminate(tick), 16);
    assert_eq!(at(0), "a a a a . . . . . . . . . . . .");
    assert_eq!(at(8), ". a a a a . . . . . . . . . . .");
    assert_eq!(at(4), "▌.a a a a ▌a. . . . . . . . . . . .");
}

#[test]
fn the_indeterminate_block_bounces_off_both_ends() {
    let at = |tick| shape(&ProgressBar::new().indeterminate(tick), 8);
    assert_eq!(at(0), "a a . . . . . .");
    assert_eq!(at(48), ". . . . . . a a");
    assert_eq!(at(96), "a a . . . . . .");
    assert_eq!(at(72), ". . . a a . . .");
}

#[test]
fn a_one_cell_indeterminate_bar_is_a_single_block() {
    let bar = ProgressBar::new().indeterminate(5);
    let still = ProgressBar::new().indeterminate(0);
    assert_eq!(shape(&still, 1), "a");
    assert_eq!(shape(&still, 2), "a .");
    assert_eq!(shape(&bar, 1), "a");
}

#[test]
fn the_percentage_is_the_filled_share_and_absent_when_indeterminate() {
    assert_eq!(ProgressBar::new().ratio(0.426).percent(), Some(43));
    assert_eq!(ProgressBar::new().percent(), Some(0));
    assert_eq!(
        ProgressBar::new()
            .segment(0.3, ProgressTone::Ok)
            .segment(0.25, ProgressTone::Warn)
            .percent(),
        Some(55)
    );
    assert_eq!(ProgressBar::new().ratio(2.0).percent(), Some(100));
    assert_eq!(ProgressBar::new().indeterminate(3).percent(), None);
}

fn text_row(bar: &ProgressBar<'_>, width: u16) -> String {
    let (grid, clusters) = draw(bar, Rect::new(0, 0, width, 1), width, 1);
    grid.row_display(0, &clusters)
}

#[test]
fn a_label_comes_before_the_track_and_the_percentage_after_it() {
    let bar = ProgressBar::new()
        .ratio(0.5)
        .label("Build")
        .show_percent(true);
    assert_eq!(text_row(&bar, 24), "Build ██████▌██████ 50% ");
    let plain = ProgressBar::new().ratio(0.5).show_percent(true);
    assert_eq!(text_row(&plain, 10), "██▌██ 50% ");
}

#[test]
fn the_track_area_leaves_room_for_the_label_and_the_widest_percentage() {
    let bar = ProgressBar::new().label("Build").show_percent(true);
    assert_eq!(
        bar.track_area(Rect::new(2, 3, 24, 2)),
        Rect::new(8, 3, 13, 2)
    );
    let bare = ProgressBar::new();
    assert_eq!(
        bare.track_area(Rect::new(2, 3, 24, 1)),
        Rect::new(2, 3, 24, 1)
    );
    let sweeping = ProgressBar::new().indeterminate(0).show_percent(true);
    assert_eq!(sweeping.track_area(Rect::new(0, 0, 10, 1)).width, 10);
    assert_eq!(bar.track_area(Rect::new(0, 0, 8, 1)).width, 0);
}

#[test]
fn the_bar_fills_every_row_of_its_area() {
    let (grid, clusters) = draw(&ProgressBar::new().ratio(1.0), Rect::new(1, 1, 3, 2), 6, 4);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(grid.row_display(1, &clusters), " ███  ");
    assert_eq!(grid.row_display(2, &clusters), " ███  ");
    assert_eq!(grid.row_display(3, &clusters).trim(), "");
}

#[test]
fn colors_follow_the_theme_and_a_disabled_bar_is_dimmed() {
    let bar = ProgressBar::new().ratio(0.5).label("L").show_percent(true);
    let (grid, _) = draw(&bar, Rect::new(0, 0, 20, 1), 20, 1);
    let style = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default();
    assert_eq!(style(0).fg, theme().resolve(Token::TextPrimary));
    assert_eq!(style(2).fg, theme().resolve(Token::Accent));
    assert!(!style(2).attrs.dim());
    let disabled = bar.clone().state(WidgetState::Disabled);
    let (grid, _) = draw(&disabled, Rect::new(0, 0, 20, 1), 20, 1);
    let style = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default();
    assert!(style(0).attrs.dim());
    assert!(style(2).attrs.dim());
    assert!(style(17).attrs.dim());
    let focused = bar.state(WidgetState::Focused);
    let (grid, _) = draw(&focused, Rect::new(0, 0, 20, 1), 20, 1);
    assert!(
        grid.get(0, 0)
            .map(Cell::style)
            .unwrap_or_default()
            .attrs
            .bold()
    );
}

#[test]
fn an_explicit_style_replaces_the_segment_colors() {
    let chosen = Color::Rgb(1, 2, 3);
    let bar = ProgressBar::new()
        .segment(0.5, ProgressTone::Fail)
        .style(lunatui_core::Style::new().fg(chosen));
    let (grid, _) = draw(&bar, Rect::new(0, 0, 4, 1), 4, 1);
    assert_eq!(
        grid.get(0, 0).map(Cell::style).unwrap_or_default().fg,
        chosen
    );
    assert_eq!(
        grid.get(3, 0).map(Cell::style).unwrap_or_default().fg,
        theme().resolve(Token::Surface1)
    );
}

#[test]
fn nothing_is_drawn_for_empty_areas_or_outside_the_area() {
    let bar = ProgressBar::new().ratio(1.0).label("L").show_percent(true);
    let (grid, clusters) = draw(&bar, Rect::new(0, 0, 0, 1), 8, 2);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = draw(&bar, Rect::new(0, 0, 8, 0), 8, 2);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = draw(&bar, Rect::new(2, 0, 6, 1), 12, 1);
    assert_eq!(grid.row_display(0, &clusters), "  L  100    ");
}

#[test]
fn measure_is_the_width_on_one_row() {
    assert_eq!(
        ProgressBar::new().measure(Size::new(30, 5)),
        Size::new(30, 1)
    );
    assert_eq!(ProgressBar::new().measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(ProgressBar::new().measure(Size::new(5, 0)), Size::new(0, 0));
}

#[test]
fn a_cell_with_several_segment_edges_shows_the_first_two_colors() {
    let bar = ProgressBar::new()
        .segment(0.125, ProgressTone::Ok)
        .segment(0.125, ProgressTone::Warn)
        .segment(0.5, ProgressTone::Fail);
    assert_eq!(shape(&bar, 1), "▏ow");
    assert_eq!(shape(&bar, 2), "▎ow ▌f.");
}
