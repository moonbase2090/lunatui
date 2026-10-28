// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Slider, Widget, WidgetState};

fn draw(slider: &Slider<'_>, area: Rect, grid_width: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(grid_width, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    slider.render(area, &mut cx);
    (grid, clusters)
}

fn row(slider: &Slider<'_>, width: u16) -> String {
    let (grid, clusters) = draw(slider, Rect::new(0, 0, width, 1), width);
    grid.row_display(0, &clusters)
}

fn style_at(grid: &Grid, x: u16) -> Style {
    grid.get(x, 0).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn ten() -> Slider<'static> {
    Slider::new(0.0, 10.0)
}

#[test]
fn the_filled_part_runs_from_the_minimum_to_the_value() {
    assert_eq!(row(&ten().value(0.0), 10), "──────────");
    assert_eq!(row(&ten().value(5.0), 10), "━━━━━─────");
    assert_eq!(row(&ten().value(10.0), 10), "━━━━━━━━━━");
    assert_eq!(row(&ten(), 10), "──────────");
}

#[test]
fn the_end_of_the_fill_can_sit_on_a_half_cell() {
    assert_eq!(row(&ten().value(5.5), 10), "━━━━━╸────");
    assert_eq!(row(&ten().value(0.5), 10), "╸─────────");
    assert_eq!(row(&ten().value(9.5), 10), "━━━━━━━━━╸");
    assert_eq!(row(&ten().value(1.0), 10), "━─────────");
}

#[test]
fn a_range_fills_between_its_ends_with_half_cells_on_either_side() {
    assert_eq!(row(&ten().range(2.0, 6.0), 10), "──━━━━────");
    assert_eq!(row(&ten().range(2.5, 6.5), 10), "──╺━━━╸───");
    assert_eq!(row(&ten().range(6.0, 2.0), 10), "──━━━━────");
    assert_eq!(row(&ten().range(0.0, 10.0), 10), "━━━━━━━━━━");
    assert_eq!(row(&ten().range(5.0, 5.0), 10), "──────────");
}

#[test]
fn values_are_clamped_and_nan_counts_as_the_minimum() {
    assert_eq!(row(&ten().value(99.0), 10), "━━━━━━━━━━");
    assert_eq!(row(&ten().value(-4.0), 10), "──────────");
    assert_eq!(row(&ten().value(f64::NAN), 10), "──────────");
    assert_eq!(row(&ten().range(-3.0, 99.0), 10), "━━━━━━━━━━");
}

#[test]
fn bounds_the_wrong_way_round_are_swapped_and_a_flat_span_fills_nothing() {
    assert_eq!(row(&Slider::new(10.0, 0.0).value(5.0), 10), "━━━━━─────");
    assert_eq!(row(&Slider::new(3.0, 3.0).value(3.0), 6), "──────");
    assert_eq!(
        row(&Slider::new(f64::NAN, 10.0).value(5.0), 10),
        "━━━━━─────"
    );
}

#[test]
fn a_knob_marks_the_cell_of_each_end() {
    assert_eq!(row(&ten().value(5.0).knob(true), 10), "━━━━●─────");
    assert_eq!(row(&ten().value(0.0).knob(true), 10), "●─────────");
    assert_eq!(row(&ten().value(10.0).knob(true), 10), "━━━━━━━━━●");
    assert_eq!(row(&ten().range(2.0, 6.0).knob(true), 10), "──●━━●────");
    assert_eq!(row(&ten().range(5.0, 5.0).knob(true), 10), "─────●────");
}

#[test]
fn a_label_comes_before_the_track_and_the_value_after_it() {
    let volume = Slider::new(0.0, 100.0).label("Volume").value(50.0);
    assert_eq!(row(&volume, 20), "Volume ━━━━━━╸──────");
    let shown = volume.clone().show_value(true, 0);
    assert_eq!(row(&shown, 20), "Volume ━━━━╸──── 50 ");
}

#[test]
fn the_value_text_follows_the_precision_and_ranges_show_both_ends() {
    let precise = Slider::new(0.0, 1.0).value(0.3).show_value(true, 2);
    assert_eq!(row(&precise, 17), "━━━╸──────── 0.30");
    let range = Slider::new(0.0, 100.0)
        .range(20.0, 60.0)
        .show_value(true, 0);
    assert_eq!(row(&range, 24), "───━━━━━━╸────── 20–60  ");
}

#[test]
fn the_track_area_leaves_room_for_the_label_and_the_widest_value() {
    let volume = Slider::new(0.0, 100.0).label("Volume");
    assert_eq!(
        volume.track_area(Rect::new(2, 3, 20, 1)),
        Rect::new(9, 3, 13, 1)
    );
    let shown = volume.clone().show_value(true, 0);
    assert_eq!(
        shown.track_area(Rect::new(2, 3, 20, 1)),
        Rect::new(9, 3, 9, 1)
    );
    assert_eq!(
        ten().track_area(Rect::new(0, 0, 8, 4)),
        Rect::new(0, 0, 8, 1)
    );
    assert_eq!(volume.track_area(Rect::new(0, 0, 5, 1)).width, 0);
    assert_eq!(ten().track_area(Rect::new(0, 0, 0, 1)).width, 0);
    assert_eq!(ten().track_area(Rect::new(0, 0, 5, 0)).width, 0);
}

#[test]
fn value_at_maps_a_column_to_a_value_across_the_track() {
    let track = Rect::new(4, 0, 11, 1);
    let slider = ten();
    assert!((slider.value_at(track, 4) - 0.0).abs() < 1e-9);
    assert!((slider.value_at(track, 9) - 5.0).abs() < 1e-9);
    assert!((slider.value_at(track, 14) - 10.0).abs() < 1e-9);
    assert!((slider.value_at(track, 0) - 0.0).abs() < 1e-9);
    assert!((slider.value_at(track, 40) - 10.0).abs() < 1e-9);
    assert!((slider.value_at(Rect::new(0, 0, 1, 1), 0) - 0.0).abs() < 1e-9);
    assert!((slider.value_at(Rect::new(0, 0, 0, 1), 3) - 0.0).abs() < 1e-9);
}

#[test]
fn value_at_snaps_to_the_step() {
    let track = Rect::new(0, 0, 11, 1);
    let stepped = Slider::new(0.0, 100.0).step(25.0);
    assert!((stepped.value_at(track, 1) - 0.0).abs() < 1e-9);
    assert!((stepped.value_at(track, 2) - 25.0).abs() < 1e-9);
    assert!((stepped.value_at(track, 5) - 50.0).abs() < 1e-9);
    assert!((stepped.value_at(track, 9) - 100.0).abs() < 1e-9);
    let offset = Slider::new(10.0, 20.0).step(4.0);
    assert!((offset.value_at(track, 10) - 20.0).abs() < 1e-9);
    assert!((offset.value_at(track, 5) - 14.0).abs() < 1e-9);
}

#[test]
fn nudge_moves_by_whole_steps_and_stays_inside_the_bounds() {
    let slider = Slider::new(0.0, 100.0).step(5.0);
    assert!((slider.nudge(50.0, 1) - 55.0).abs() < 1e-9);
    assert!((slider.nudge(50.0, -3) - 35.0).abs() < 1e-9);
    assert!((slider.nudge(98.0, 1) - 100.0).abs() < 1e-9);
    assert!((slider.nudge(2.0, -5) - 0.0).abs() < 1e-9);
    assert!((slider.nudge(f64::NAN, 2) - 10.0).abs() < 1e-9);
    let offset = Slider::new(10.0, 60.0);
    assert!((offset.nudge(30.0, 4) - 32.0).abs() < 1e-9);
    let plain = Slider::new(0.0, 50.0);
    assert!((plain.nudge(10.0, 2) - 11.0).abs() < 1e-9);
    assert!((plain.nudge(10.0, -2) - 9.0).abs() < 1e-9);
}

#[test]
fn invalid_steps_are_ignored() {
    let track = Rect::new(0, 0, 11, 1);
    for step in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let slider = Slider::new(0.0, 10.0).step(step);
        assert!((slider.value_at(track, 3) - 3.0).abs() < 1e-9, "{step}");
    }
}

#[test]
fn thumb_areas_are_the_cells_of_the_ends() {
    let single = ten().value(5.0);
    assert_eq!(
        single.thumb_areas(Rect::new(3, 1, 10, 1)),
        [(0, Rect::new(7, 1, 1, 1))]
    );
    let range = ten().range(2.0, 6.0);
    assert_eq!(
        range.thumb_areas(Rect::new(0, 0, 10, 1)),
        [(0, Rect::new(2, 0, 1, 1)), (1, Rect::new(5, 0, 1, 1))]
    );
    assert_eq!(
        ten().value(0.0).thumb_areas(Rect::new(0, 0, 10, 1)),
        [(0, Rect::new(0, 0, 1, 1))]
    );
    assert_eq!(
        ten().value(10.0).thumb_areas(Rect::new(0, 0, 10, 1)),
        [(0, Rect::new(9, 0, 1, 1))]
    );
    assert_eq!(ten().thumb_areas(Rect::new(0, 0, 0, 1)), []);
}

#[test]
fn colors_follow_the_state_and_the_theme() {
    let slider = ten().value(5.0).knob(true).label("V");
    let (grid, _) = draw(&slider, Rect::new(0, 0, 12, 1), 12);
    assert_eq!(style_at(&grid, 0).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 2).fg, color(Token::Accent));
    assert_eq!(style_at(&grid, 5).fg, color(Token::Accent));
    assert_eq!(style_at(&grid, 7).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 9).fg, color(Token::TextPrimary));
    assert!(style_at(&grid, 9).attrs.dim());
    assert!(!style_at(&grid, 2).attrs.dim());

    let focused = slider.clone().state(WidgetState::Focused);
    let (grid, _) = draw(&focused, Rect::new(0, 0, 12, 1), 12);
    assert_eq!(style_at(&grid, 6).fg, color(Token::FocusRing));
    assert!(style_at(&grid, 6).attrs.bold());
    assert!(style_at(&grid, 0).attrs.bold());
    assert_eq!(style_at(&grid, 2).fg, color(Token::Accent));
    assert!(!style_at(&grid, 2).attrs.bold());
}

#[test]
fn a_disabled_slider_dims_the_label_the_fill_and_the_knob() {
    let slider = ten()
        .value(5.0)
        .knob(true)
        .label("V")
        .state(WidgetState::Disabled);
    let (grid, _) = draw(&slider, Rect::new(0, 0, 12, 1), 12);
    for x in [0, 2, 6, 9] {
        assert!(style_at(&grid, x).attrs.dim(), "column {x}");
    }
}

#[test]
fn an_explicit_style_replaces_the_theme_colors() {
    let style = Style::new().fg(Color::Rgb(1, 2, 3));
    let slider = ten()
        .value(5.0)
        .knob(true)
        .label("V")
        .show_value(true, 0)
        .style(style);
    let (grid, _) = draw(&slider, Rect::new(0, 0, 16, 1), 16);
    for x in [0, 2, 6, 12, 14] {
        assert_eq!(style_at(&grid, x), style, "column {x}");
    }
}

#[test]
fn nothing_is_drawn_outside_the_area_or_for_empty_areas() {
    let (grid, clusters) = draw(&ten().value(5.0), Rect::new(2, 0, 4, 1), 12);
    assert_eq!(grid.row_display(0, &clusters), "  ━━──      ");
    let (grid, clusters) = draw(&ten().value(5.0), Rect::new(0, 0, 0, 1), 6);
    assert_eq!(grid.row_display(0, &clusters), "      ");
    let (grid, clusters) = draw(&ten().value(5.0), Rect::new(0, 0, 6, 0), 6);
    assert_eq!(grid.row_display(0, &clusters), "      ");
    let (grid, clusters) = draw(&ten().value(5.0), Rect::new(0, 0, 6, 2), 6);
    assert_eq!(grid.row_display(1, &clusters), "      ");
}

#[test]
fn a_narrow_area_clips_the_label_and_leaves_no_track() {
    let slider = Slider::new(0.0, 10.0).label("Brightness").value(5.0);
    assert_eq!(row(&slider, 5), "Brigh");
    assert_eq!(row(&slider, 11), "Brightness ");
}

#[test]
fn measure_is_the_width_on_one_row() {
    assert_eq!(ten().measure(Size::new(20, 5)), Size::new(20, 1));
    assert_eq!(ten().measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(ten().measure(Size::new(5, 0)), Size::new(0, 0));
}

#[test]
fn a_range_that_starts_above_zero_positions_values_from_its_minimum() {
    let slider = Slider::new(10.0, 20.0);
    assert_eq!(row(&slider.clone().value(15.0), 10), "━━━━━─────");
    assert_eq!(row(&slider.clone().value(10.0), 10), "──────────");
    assert_eq!(row(&slider.clone().value(20.0), 10), "━━━━━━━━━━");
    assert_eq!(row(&slider.clone().range(12.0, 18.0), 10), "──━━━━━━──");
    let negative = Slider::new(-10.0, 10.0);
    assert_eq!(row(&negative.clone().value(0.0), 10), "━━━━━─────");
    assert_eq!(row(&negative.value(-10.0), 10), "──────────");
}

#[test]
fn a_range_with_both_ends_at_the_maximum_keeps_its_thumbs_on_the_last_cell() {
    let both = ten().range(10.0, 10.0);
    assert_eq!(
        both.thumb_areas(Rect::new(0, 0, 10, 1)),
        [(0, Rect::new(9, 0, 1, 1)), (1, Rect::new(9, 0, 1, 1))]
    );
    assert_eq!(row(&both.knob(true), 10), "─────────●");
}

#[test]
fn a_label_is_not_drawn_into_an_empty_area() {
    let slider = ten().label("Volume").value(5.0);
    let (grid, clusters) = draw(&slider, Rect::new(0, 0, 6, 0), 6);
    assert_eq!(grid.row_display(0, &clusters), "      ");
    let (grid, clusters) = draw(&slider, Rect::new(0, 0, 0, 1), 6);
    assert_eq!(grid.row_display(0, &clusters), "      ");
}
