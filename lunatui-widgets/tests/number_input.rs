// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{NumberInput, RenderCx, Widget, WidgetState};

fn draw(input: &NumberInput<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    input.render(area, &mut cx);
    (grid, clusters)
}

fn row(input: &NumberInput<'_>, width: u16) -> String {
    let (grid, clusters) = draw(input, Rect::new(0, 0, width, 1), width, 1);
    grid.row_display(0, &clusters)
}

fn style_at(grid: &Grid, x: u16) -> Style {
    grid.get(x, 0).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn styles(input: &NumberInput<'_>, width: u16) -> Grid {
    draw(input, Rect::new(0, 0, width, 1), width, 1).0
}

#[test]
fn the_value_is_right_aligned_between_the_buttons() {
    assert_eq!(row(&NumberInput::new(42.0), 12), " −    42  + ");
    assert_eq!(row(&NumberInput::new(7.0), 12), " −     7  + ");
    assert_eq!(row(&NumberInput::new(1234.0), 12), " −  1234  + ");
}

#[test]
fn decimals_prefix_and_suffix_surround_the_number() {
    let input = NumberInput::new(3.1449)
        .decimals(2)
        .prefix("$")
        .suffix(" kg");
    assert_eq!(row(&input, 16), " −  $3.14 kg  + ");
    assert_eq!(row(&NumberInput::new(2.5).decimals(0), 12), " −     3  + ");
}

#[test]
fn a_negative_value_that_rounds_to_zero_shows_no_sign() {
    let input = NumberInput::new(-0.004).decimals(2);
    assert_eq!(input.format(-0.004), "0.00");
    assert_eq!(input.format(-0.4), "-0.40");
    assert_eq!(input.format(-3.0), "-3.00");
    assert_eq!(row(&NumberInput::new(-0.2).decimals(0), 12), " −     0  + ");
}

#[test]
fn format_rounds_and_clamps_to_the_range() {
    let input = NumberInput::new(0.0).range(0.0, 10.0).decimals(1);
    assert_eq!(input.format(2.25), "2.3");
    assert_eq!(input.format(99.0), "10.0");
    assert_eq!(input.format(-5.0), "0.0");
    assert_eq!(input.format(f64::NAN), "0.0");
}

#[test]
fn nudge_moves_by_steps_and_stays_inside_the_range() {
    let input = NumberInput::new(5.0).range(0.0, 10.0).step(2.0);
    assert_eq!(input.nudge(1), 7.0);
    assert_eq!(input.nudge(-1), 3.0);
    assert_eq!(input.nudge(3), 10.0);
    assert_eq!(input.nudge(-9), 0.0);
    assert_eq!(input.nudge(0), 5.0);
    let free = NumberInput::new(5.0);
    assert_eq!(free.nudge(-12), -7.0);
}

#[test]
fn nudge_does_not_drift_on_decimal_steps() {
    let input = NumberInput::new(0.1).decimals(1).step(0.1);
    assert_eq!(input.nudge(2), 0.3);
    assert_eq!(input.nudge(-1), 0.0);
    let money = NumberInput::new(19.99).decimals(2).step(0.01);
    assert_eq!(money.nudge(1), 20.0);
    assert_eq!(money.nudge(-1), 19.98);
}

#[test]
fn nudge_pulls_an_out_of_range_value_back_in() {
    let input = NumberInput::new(50.0).range(0.0, 10.0);
    assert_eq!(input.nudge(0), 10.0);
    assert_eq!(input.nudge(-1), 9.0);
    let below = NumberInput::new(-5.0).range(2.0, 10.0);
    assert_eq!(below.nudge(1), 3.0);
}

#[test]
fn a_nan_value_counts_as_zero_inside_the_range() {
    assert_eq!(NumberInput::new(f64::NAN).nudge(1), 1.0);
    assert_eq!(NumberInput::new(f64::NAN).range(5.0, 9.0).nudge(0), 5.0);
    assert_eq!(NumberInput::new(f64::NAN).range(-9.0, -5.0).nudge(0), -5.0);
}

#[test]
fn a_bad_step_counts_as_one_and_a_backwards_range_is_put_in_order() {
    assert_eq!(NumberInput::new(5.0).step(0.0).nudge(1), 6.0);
    assert_eq!(NumberInput::new(5.0).step(-2.0).nudge(1), 6.0);
    assert_eq!(NumberInput::new(5.0).step(f64::NAN).nudge(1), 6.0);
    let input = NumberInput::new(5.0).range(10.0, 0.0);
    assert_eq!(input.nudge(100), 10.0);
    assert_eq!(input.nudge(-100), 0.0);
}

#[test]
fn accepts_allows_partial_entries_while_typing() {
    let input = NumberInput::new(0.0).range(-10.0, 10.0).decimals(2);
    for text in ["", "-", "1", "-1", "12.", "12.3", "12.34", ".5", "-.", "."] {
        assert!(input.accepts(text), "{text}");
    }
    for text in ["12.345", "1.2.3", "--1", "1-", "a", "1e3", "+1", " 1", "1 "] {
        assert!(!input.accepts(text), "{text}");
    }
}

#[test]
fn accepts_needs_a_negative_bound_for_a_sign_and_decimals_for_a_point() {
    let unsigned = NumberInput::new(0.0).range(0.0, 10.0);
    assert!(!unsigned.accepts("-"));
    assert!(!unsigned.accepts("-1"));
    assert!(!unsigned.accepts("1."));
    assert!(!unsigned.accepts("."));
    assert!(unsigned.accepts("10"));
    let signed = NumberInput::new(0.0).range(-1.0, 1.0);
    assert!(signed.accepts("-1"));
    let unbounded = NumberInput::new(0.0);
    assert!(unbounded.accepts("-1"));
}

#[test]
fn accepts_limits_the_length_of_the_text() {
    let input = NumberInput::new(0.0);
    assert!(input.accepts(&"1".repeat(20)));
    assert!(!input.accepts(&"1".repeat(21)));
}

#[test]
fn parse_reads_a_number_rounds_it_and_clamps_it() {
    let input = NumberInput::new(0.0).range(-5.0, 100.0).decimals(1);
    assert_eq!(input.parse("42"), Some(42.0));
    assert_eq!(input.parse("-3.5"), Some(-3.5));
    assert_eq!(input.parse("7."), Some(7.0));
    assert_eq!(input.parse(".5"), Some(0.5));
    assert_eq!(input.parse("500"), Some(100.0));
    assert_eq!(input.parse("-50"), Some(-5.0));
    assert_eq!(input.parse("0.25"), None);
    assert_eq!(input.parse("0.2"), Some(0.2));
}

#[test]
fn parse_rejects_text_without_digits_or_with_bad_characters() {
    let input = NumberInput::new(0.0).range(-5.0, 100.0).decimals(1);
    for text in ["", "-", ".", "-.", "x", "1x", "1.25"] {
        assert_eq!(input.parse(text), None, "{text}");
    }
}

#[test]
fn the_bounds_report_when_a_button_is_spent() {
    let input = NumberInput::new(0.0).range(0.0, 10.0);
    assert!(input.at_min());
    assert!(!input.at_max());
    let top = NumberInput::new(10.0).range(0.0, 10.0);
    assert!(top.at_max());
    assert!(!top.at_min());
    let over = NumberInput::new(99.0).range(0.0, 10.0);
    assert!(over.at_max());
    let free = NumberInput::new(5.0);
    assert!(!free.at_min() && !free.at_max());
}

#[test]
fn the_buttons_and_field_split_the_area() {
    let input = NumberInput::new(1.0);
    let area = Rect::new(2, 3, 14, 2);
    assert_eq!(input.minus_area(area), Rect::new(2, 3, 3, 1));
    assert_eq!(input.field_area(area), Rect::new(5, 3, 8, 1));
    assert_eq!(input.plus_area(area), Rect::new(13, 3, 3, 1));
    let tight = Rect::new(0, 0, 9, 1);
    assert_eq!(input.field_area(tight), Rect::new(3, 0, 3, 1));
    assert_eq!(input.plus_area(tight), Rect::new(6, 0, 3, 1));
}

#[test]
fn narrow_areas_drop_the_buttons() {
    let input = NumberInput::new(42.0);
    let area = Rect::new(1, 0, 8, 1);
    assert_eq!(input.minus_area(area).width, 0);
    assert_eq!(input.plus_area(area), Rect::new(9, 0, 0, 1));
    assert_eq!(input.field_area(area), Rect::new(1, 0, 8, 1));
    assert_eq!(row(&input, 8), "     42 ");
}

#[test]
fn editing_shows_the_typed_text_and_an_inverted_caret_when_focused() {
    let input = NumberInput::new(5.0)
        .decimals(1)
        .edit("12.5", 2)
        .state(WidgetState::Focused);
    assert_eq!(row(&input, 14), " −    12.5  + ");
    let grid = styles(&input, 14);
    assert_eq!(style_at(&grid, 7).fg, color(Token::Accent));
    assert_eq!(style_at(&grid, 7).bg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 8).bg, color(Token::Accent));
    assert_eq!(style_at(&grid, 8).fg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 9).bg, color(Token::Surface1));
}

#[test]
fn the_caret_at_the_end_sits_on_a_blank_cell_after_the_text() {
    let input = NumberInput::new(5.0)
        .edit("12", 2)
        .state(WidgetState::Focused);
    let grid = styles(&input, 12);
    assert_eq!(style_at(&grid, 8).bg, color(Token::Accent));
    assert_eq!(style_at(&grid, 7).bg, color(Token::Surface1));
}

#[test]
fn the_caret_counts_the_prefix_and_is_not_drawn_unfocused() {
    let focused = NumberInput::new(5.0)
        .prefix("$")
        .edit("12", 0)
        .state(WidgetState::Focused);
    let grid = styles(&focused, 14);
    assert_eq!(style_at(&grid, 8).bg, color(Token::Accent));
    let plain = NumberInput::new(5.0).prefix("$").edit("12", 0);
    let grid = styles(&plain, 14);
    for x in 0..14 {
        assert_ne!(style_at(&grid, x).bg, color(Token::TextPrimary));
        assert_ne!(style_at(&grid, x).bg, color(Token::Accent));
    }
}

#[test]
fn text_that_cannot_be_committed_is_drawn_in_the_fail_color() {
    let bad = NumberInput::new(5.0).edit("-", 1);
    assert_eq!(style_at(&styles(&bad, 12), 7).fg, color(Token::Fail));
    let empty = NumberInput::new(5.0).edit("", 0);
    assert_eq!(style_at(&styles(&empty, 12), 7).fg, color(Token::Fail));
    let good = NumberInput::new(5.0).edit("12", 2);
    assert_eq!(
        style_at(&styles(&good, 12), 6).fg,
        color(Token::TextPrimary)
    );
    let disabled = NumberInput::new(5.0)
        .edit("-", 1)
        .state(WidgetState::Disabled);
    assert_eq!(
        style_at(&styles(&disabled, 12), 7).fg,
        color(Token::TextPrimary)
    );
}

#[test]
fn colors_follow_the_state_and_spent_buttons_are_dimmed() {
    let input = NumberInput::new(10.0).range(0.0, 10.0);
    let grid = styles(&input, 12);
    assert_eq!(style_at(&grid, 6).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 6).attrs.bold());
    assert_eq!(style_at(&grid, 1).fg, color(Token::Accent));
    assert!(!style_at(&grid, 1).attrs.dim());
    assert!(style_at(&grid, 10).attrs.dim());
    assert_eq!(style_at(&grid, 0).bg, color(Token::Surface1));
    let focused = styles(&input.clone().state(WidgetState::Focused), 12);
    assert_eq!(style_at(&focused, 6).fg, color(Token::Accent));
    assert!(style_at(&focused, 6).attrs.bold());
    let disabled = styles(&input.state(WidgetState::Disabled), 12);
    for x in [1, 6, 10] {
        assert!(style_at(&disabled, x).attrs.dim());
    }
    assert!(!style_at(&disabled, 6).attrs.bold());
}

#[test]
fn an_explicit_style_colors_the_text() {
    let chosen = Color::Rgb(1, 2, 3);
    let input = NumberInput::new(42.0).style(Style::new().fg(chosen));
    let grid = styles(&input, 12);
    assert_eq!(style_at(&grid, 6).fg, chosen);
    assert_eq!(style_at(&grid, 1).fg, color(Token::Accent));
}

#[test]
fn long_text_is_clipped_to_the_field() {
    let input = NumberInput::new(123_456_789.0).suffix(" units");
    assert_eq!(row(&input, 12), " −  1234  + ");
}

#[test]
fn nothing_is_drawn_for_empty_areas_or_outside_the_area() {
    let input = NumberInput::new(42.0);
    let (grid, clusters) = draw(&input, Rect::new(0, 0, 0, 1), 12, 2);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = draw(&input, Rect::new(0, 0, 12, 0), 12, 2);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = draw(&input, Rect::new(2, 1, 10, 3), 14, 4);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(grid.row_display(1, &clusters), "   −  42  +   ");
    assert_eq!(grid.row_display(2, &clusters).trim(), "");
}

#[test]
fn measure_fits_the_longest_number_between_the_buttons() {
    let input = NumberInput::new(5.0).range(0.0, 1000.0).suffix("%");
    assert_eq!(input.measure(Size::new(40, 5)), Size::new(13, 1));
    assert_eq!(input.measure(Size::new(10, 5)), Size::new(10, 1));
    assert_eq!(input.measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(input.measure(Size::new(5, 0)), Size::new(0, 0));
    let decimals = NumberInput::new(1.5).decimals(2).prefix("$");
    assert_eq!(decimals.measure(Size::new(40, 1)), Size::new(13, 1));
    let unbounded = NumberInput::new(f64::NAN);
    assert_eq!(unbounded.measure(Size::new(40, 1)), Size::new(9, 1));
}
