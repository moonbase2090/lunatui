// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style, UnderlineStyle};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Date, DatePicker, RenderCx, WeekStart, Widget, WidgetState};

fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_ymd(year, month, day).unwrap_or(Date::UNIX_EPOCH)
}

fn draw(picker: &DatePicker, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    picker.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(picker: &DatePicker, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(picker, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn grid(picker: &DatePicker) -> Grid {
    draw(picker, 21, 8).0
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn underlined(grid: &Grid, x: u16, y: u16) -> bool {
    style_at(grid, x, y).attrs.underline() != UnderlineStyle::None
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

const OCTOBER: [&str; 8] = [
    " ‹  October 2026   ›",
    " Su Mo Tu We Th Fr Sa",
    "              1  2  3",
    "  4  5  6  7  8  9 10",
    " 11 12 13 14 15 16 17",
    " 18 19 20 21 22 23 24",
    " 25 26 27 28 29 30 31",
    "",
];

#[test]
fn a_month_is_drawn_under_its_header_and_weekday_row() {
    let picker = DatePicker::new(date(2026, 10, 14));
    assert_eq!(rows(&picker, 21, 8), OCTOBER);
}

#[test]
fn a_week_can_start_on_monday() {
    let picker = DatePicker::new(date(2026, 10, 14)).week_start(WeekStart::Monday);
    assert_eq!(
        rows(&picker, 21, 8),
        [
            " ‹  October 2026   ›",
            " Mo Tu We Th Fr Sa Su",
            "           1  2  3  4",
            "  5  6  7  8  9 10 11",
            " 12 13 14 15 16 17 18",
            " 19 20 21 22 23 24 25",
            " 26 27 28 29 30 31",
            "",
        ]
    );
    let sunday_first = DatePicker::new(date(2026, 11, 1)).week_start(WeekStart::Monday);
    assert_eq!(rows(&sunday_first, 21, 8)[2], "                    1");
}

#[test]
fn month_lengths_and_leap_years_are_respected() {
    let february = DatePicker::new(date(2026, 2, 1));
    assert_eq!(
        rows(&february, 21, 8)[2..],
        [
            "  1  2  3  4  5  6  7",
            "  8  9 10 11 12 13 14",
            " 15 16 17 18 19 20 21",
            " 22 23 24 25 26 27 28",
            "",
            ""
        ]
    );
    let leap = DatePicker::new(date(2028, 2, 1));
    assert_eq!(rows(&leap, 21, 8)[0], " ‹  February 2028  ›");
    assert_eq!(rows(&leap, 21, 8)[6], " 27 28 29");
}

#[test]
fn a_month_that_needs_six_rows_fills_them() {
    let august = DatePicker::new(date(2026, 8, 3));
    assert_eq!(rows(&august, 21, 8)[2], "                    1");
    assert_eq!(rows(&august, 21, 8)[7], " 30 31");
}

#[test]
fn a_chosen_day_is_a_filled_chip() {
    let picker = DatePicker::new(date(2026, 10, 14)).selected(date(2026, 10, 14));
    let cells = grid(&picker);
    for x in [10, 11] {
        let style = style_at(&cells, x, 4);
        assert_eq!(style.bg, color(Token::Accent), "{x}");
        assert_eq!(style.fg, color(Token::Surface0), "{x}");
        assert!(style.attrs.bold(), "{x}");
    }
    assert_eq!(style_at(&cells, 9, 4).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 12, 4).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 13, 4).bg, color(Token::Surface0));
    assert!(!style_at(&cells, 13, 4).attrs.bold());
}

#[test]
fn a_range_has_filled_ends_and_a_band_between() {
    let picker = DatePicker::new(date(2026, 10, 14)).range(date(2026, 10, 12), date(2026, 10, 16));
    let cells = grid(&picker);
    for x in [4, 5, 16, 17] {
        assert_eq!(style_at(&cells, x, 4).bg, color(Token::Accent), "{x}");
    }
    for x in [7, 8, 10, 11, 13, 14] {
        assert_eq!(style_at(&cells, x, 4).bg, color(Token::Surface1), "{x}");
        assert_eq!(style_at(&cells, x, 4).fg, color(Token::TextPrimary), "{x}");
    }
    assert_eq!(style_at(&cells, 3, 4).bg, color(Token::Surface0));
    for gutter in [6, 9, 12, 15] {
        assert_eq!(
            style_at(&cells, gutter, 4).bg,
            color(Token::Surface1),
            "{gutter}"
        );
    }
    assert_eq!(style_at(&cells, 3, 4).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 18, 4).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 19, 4).bg, color(Token::Surface0));
    let reversed =
        DatePicker::new(date(2026, 10, 14)).range(date(2026, 10, 16), date(2026, 10, 12));
    assert_eq!(grid(&reversed), cells);
}

#[test]
fn a_range_wraps_across_rows() {
    let picker = DatePicker::new(date(2026, 10, 14)).range(date(2026, 10, 9), date(2026, 10, 11));
    let cells = grid(&picker);
    assert_eq!(style_at(&cells, 15, 3).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 16, 3).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 18, 3).bg, color(Token::Surface1));
    assert_eq!(style_at(&cells, 19, 3).bg, color(Token::Surface1));
    assert_eq!(style_at(&cells, 0, 4).bg, color(Token::Surface1));
    assert_eq!(style_at(&cells, 1, 4).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 3, 4).bg, color(Token::Surface0));
}

#[test]
fn days_outside_the_bounds_are_dimmed() {
    let picker = DatePicker::new(date(2026, 10, 14))
        .min(date(2026, 10, 10))
        .max(date(2026, 10, 20));
    let cells = grid(&picker);
    for (x, y) in [(11, 3), (2, 6), (11, 5)] {
        assert!(style_at(&cells, x, y).attrs.dim(), "{x},{y}");
    }
    for (x, y) in [(1, 4), (8, 5), (11, 4), (20, 3)] {
        assert!(!style_at(&cells, x, y).attrs.dim(), "{x},{y}");
    }
}

#[test]
fn the_month_arrows_dim_when_the_bounds_stop_navigation() {
    let open = grid(&DatePicker::new(date(2026, 10, 14)));
    for x in [1, 19] {
        assert_eq!(style_at(&open, x, 0).fg, color(Token::Accent), "{x}");
        assert!(style_at(&open, x, 0).attrs.bold(), "{x}");
        assert!(!style_at(&open, x, 0).attrs.dim(), "{x}");
    }
    let held = DatePicker::new(date(2026, 10, 14))
        .min(date(2026, 10, 1))
        .max(date(2026, 10, 31));
    let cells = grid(&held);
    for x in [1, 19] {
        assert!(style_at(&cells, x, 0).attrs.dim(), "{x}");
        assert!(!style_at(&cells, x, 0).attrs.bold(), "{x}");
    }
    let before = DatePicker::new(date(2026, 10, 14))
        .min(date(2026, 9, 30))
        .max(date(2026, 11, 1));
    let cells = grid(&before);
    for x in [1, 19] {
        assert!(!style_at(&cells, x, 0).attrs.dim(), "{x}");
    }
}

#[test]
fn today_is_accented_and_the_focused_cursor_is_underlined() {
    let picker = DatePicker::new(date(2026, 10, 14)).today(date(2026, 10, 20));
    let cells = grid(&picker);
    assert_eq!(style_at(&cells, 7, 5).fg, color(Token::Accent));
    assert!(style_at(&cells, 7, 5).attrs.bold());
    assert!(!underlined(&cells, 7, 5));
    assert!(!underlined(&cells, 10, 4));
    let focused = picker.state(WidgetState::Focused);
    let cells = grid(&focused);
    assert!(underlined(&cells, 10, 4));
    assert!(underlined(&cells, 11, 4));
    assert!(style_at(&cells, 10, 4).attrs.bold());
    assert!(!underlined(&cells, 13, 4));
    assert!(!underlined(&cells, 9, 4));
}

#[test]
fn disabled_dims_everything_and_shows_no_cursor() {
    let picker = DatePicker::new(date(2026, 10, 14))
        .selected(date(2026, 10, 14))
        .state(WidgetState::Disabled);
    let cells = grid(&picker);
    for (x, y) in [(5, 0), (1, 0), (2, 1), (10, 4), (3, 6)] {
        assert!(style_at(&cells, x, y).attrs.dim(), "{x},{y}");
    }
    assert!(!underlined(&cells, 10, 4));
}

#[test]
fn the_header_and_weekdays_are_styled() {
    let cells = grid(&DatePicker::new(date(2026, 10, 14)));
    assert!(style_at(&cells, 5, 0).attrs.bold());
    assert!(!style_at(&cells, 5, 0).attrs.dim());
    assert!(style_at(&cells, 2, 1).attrs.dim());
    assert!(!style_at(&cells, 2, 1).attrs.bold());
}

#[test]
fn bounds_decide_what_is_allowed_and_where_a_date_clamps() {
    let picker = DatePicker::new(date(2026, 10, 14))
        .min(date(2026, 10, 10))
        .max(date(2026, 10, 20));
    assert!(picker.allows(date(2026, 10, 10)));
    assert!(picker.allows(date(2026, 10, 20)));
    assert!(!picker.allows(date(2026, 10, 9)));
    assert!(!picker.allows(date(2026, 10, 21)));
    assert_eq!(picker.clamped(date(2026, 10, 9)), date(2026, 10, 10));
    assert_eq!(picker.clamped(date(2026, 10, 21)), date(2026, 10, 20));
    assert_eq!(picker.clamped(date(2026, 10, 15)), date(2026, 10, 15));
    let open = DatePicker::new(date(2026, 10, 14));
    assert!(open.allows(date(1900, 1, 1)));
    assert_eq!(open.clamped(date(2100, 1, 1)), date(2100, 1, 1));
    let above = DatePicker::new(date(2026, 10, 14)).min(date(2026, 10, 10));
    assert_eq!(above.clamped(date(2100, 1, 1)), date(2100, 1, 1));
    let below = DatePicker::new(date(2026, 10, 14)).max(date(2026, 10, 20));
    assert_eq!(below.clamped(date(1900, 1, 1)), date(1900, 1, 1));
}

#[test]
fn shifting_days_crosses_months_and_years() {
    assert_eq!(
        DatePicker::shift_days(date(2026, 10, 14), 0),
        date(2026, 10, 14)
    );
    assert_eq!(
        DatePicker::shift_days(date(2026, 10, 14), 7),
        date(2026, 10, 21)
    );
    assert_eq!(
        DatePicker::shift_days(date(2026, 10, 14), -14),
        date(2026, 9, 30)
    );
    assert_eq!(
        DatePicker::shift_days(date(2026, 12, 31), 1),
        date(2027, 1, 1)
    );
    assert_eq!(
        DatePicker::shift_days(date(2027, 1, 1), -1),
        date(2026, 12, 31)
    );
    assert_eq!(
        DatePicker::shift_days(date(2028, 2, 28), 2),
        date(2028, 3, 1)
    );
    assert_eq!(
        DatePicker::shift_days(date(2026, 2, 28), 1),
        date(2026, 3, 1)
    );
}

#[test]
fn shifting_months_keeps_the_day_when_it_can() {
    assert_eq!(
        DatePicker::shift_months(date(2026, 10, 14), 1),
        date(2026, 11, 14)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 10, 14), -1),
        date(2026, 9, 14)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 1, 14), -1),
        date(2025, 12, 14)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 12, 14), 1),
        date(2027, 1, 14)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 10, 14), 12),
        date(2027, 10, 14)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 10, 14), -25),
        date(2024, 9, 14)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 10, 14), 0),
        date(2026, 10, 14)
    );
}

#[test]
fn shifting_months_uses_the_last_day_of_a_shorter_month() {
    assert_eq!(
        DatePicker::shift_months(date(2026, 1, 31), 1),
        date(2026, 2, 28)
    );
    assert_eq!(
        DatePicker::shift_months(date(2028, 1, 31), 1),
        date(2028, 2, 29)
    );
    assert_eq!(
        DatePicker::shift_months(date(2028, 2, 29), 12),
        date(2029, 2, 28)
    );
    assert_eq!(
        DatePicker::shift_months(date(2026, 3, 31), -1),
        date(2026, 2, 28)
    );
}

#[test]
fn shifting_months_saturates_at_the_ends_of_the_year_range() {
    assert_eq!(
        DatePicker::shift_months(date(i32::MAX, 12, 31), 1),
        date(i32::MAX, 1, 31)
    );
    assert_eq!(
        DatePicker::shift_months(date(i32::MIN, 1, 1), -1),
        date(i32::MIN, 12, 1)
    );
}

#[test]
fn the_arrows_have_hit_areas_at_both_ends_of_the_header() {
    let picker = DatePicker::new(date(2026, 10, 14));
    let area = Rect::new(2, 4, 30, 10);
    assert_eq!(picker.prev_area(area), Rect::new(2, 4, 3, 1));
    assert_eq!(picker.next_area(area), Rect::new(20, 4, 3, 1));
    let narrow = Rect::new(2, 4, 2, 10);
    assert_eq!(picker.prev_area(narrow), Rect::new(2, 4, 2, 1));
    assert_eq!(picker.next_area(narrow), Rect::new(4, 4, 0, 1));
    let tight = Rect::new(0, 0, 20, 1);
    assert_eq!(picker.next_area(tight), Rect::new(18, 0, 2, 1));
}

#[test]
fn date_at_maps_cells_to_days() {
    let picker = DatePicker::new(date(2026, 10, 14));
    let area = Rect::new(2, 4, 30, 10);
    assert_eq!(
        picker.date_at(area, 2 + 10, 4 + 4),
        Some(date(2026, 10, 14))
    );
    assert_eq!(picker.date_at(area, 2 + 9, 4 + 4), Some(date(2026, 10, 14)));
    assert_eq!(
        picker.date_at(area, 2 + 11, 4 + 4),
        Some(date(2026, 10, 14))
    );
    assert_eq!(
        picker.date_at(area, 2 + 12, 4 + 4),
        Some(date(2026, 10, 15))
    );
    assert_eq!(picker.date_at(area, 2 + 12, 4 + 2), Some(date(2026, 10, 1)));
    assert_eq!(picker.date_at(area, 2 + 18, 4 + 2), Some(date(2026, 10, 3)));
    assert_eq!(
        picker.date_at(area, 2 + 20, 4 + 6),
        Some(date(2026, 10, 31))
    );
    assert_eq!(picker.date_at(area, 2, 4 + 3), Some(date(2026, 10, 4)));
}

#[test]
fn date_at_ignores_everything_that_is_not_a_day() {
    let picker = DatePicker::new(date(2026, 10, 14));
    let area = Rect::new(2, 4, 30, 10);
    assert_eq!(picker.date_at(area, 2 + 3, 4), None);
    assert_eq!(picker.date_at(area, 2 + 3, 4 + 1), None);
    assert_eq!(picker.date_at(area, 2, 4 + 2), None);
    assert_eq!(picker.date_at(area, 2 + 11, 4 + 2), None);
    assert_eq!(picker.date_at(area, 2 + 3, 4 + 7), None);
    assert_eq!(picker.date_at(area, 2 + 21, 4 + 4), None);
    assert_eq!(picker.date_at(area, 1, 4 + 4), None);
    assert_eq!(picker.date_at(area, 2 + 3, 3), None);
    assert_eq!(picker.date_at(area, 2 + 3, 4 + 8), None);
    assert_eq!(picker.date_at(Rect::new(2, 4, 30, 7), 2 + 3, 4 + 7), None);
    assert_eq!(picker.date_at(Rect::new(2, 4, 10, 10), 2 + 12, 4 + 4), None);
}

#[test]
fn date_at_follows_the_week_start_and_the_bounds() {
    let monday = DatePicker::new(date(2026, 10, 14)).week_start(WeekStart::Monday);
    let area = Rect::new(0, 0, 21, 8);
    assert_eq!(monday.date_at(area, 10, 2), Some(date(2026, 10, 1)));
    assert_eq!(monday.date_at(area, 7, 2), None);
    assert_eq!(monday.date_at(area, 19, 2), Some(date(2026, 10, 4)));
    let bounded = DatePicker::new(date(2026, 10, 14)).min(date(2026, 10, 10));
    assert_eq!(bounded.date_at(area, 1, 4), Some(date(2026, 10, 11)));
    assert_eq!(bounded.date_at(area, 1, 3), None);
}

#[test]
fn a_narrow_or_short_area_clips_instead_of_overflowing() {
    let picker = DatePicker::new(date(2026, 10, 14));
    assert_eq!(
        rows(&picker, 10, 4),
        [" ‹  Octobe", " Su Mo Tu", "", "  4  5  6"]
    );
}

#[test]
fn an_empty_area_draws_nothing() {
    let picker = DatePicker::new(date(2026, 10, 14));
    let theme = Theme::builtin(Variant::Dark);
    let mut cells = Grid::new(30, 10);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut cells, &mut clusters, &theme);
    picker.render(Rect::new(0, 0, 0, 5), &mut cx);
    picker.render(Rect::new(0, 0, 30, 0), &mut cx);
    assert_eq!(cells, Grid::new(30, 10));
}

#[test]
fn drawing_stays_inside_the_area() {
    let picker = DatePicker::new(date(2026, 10, 14)).selected(date(2026, 10, 14));
    let theme = Theme::builtin(Variant::Dark);
    let mut cells = Grid::new(40, 12);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut cells, &mut clusters, &theme);
    picker.render(Rect::new(5, 2, 21, 8), &mut cx);
    assert_eq!(cells.row_display(1, &clusters).trim_end(), "");
    assert_eq!(cells.row_display(10, &clusters).trim_end(), "");
    assert_eq!(
        cells.row_display(2, &clusters).trim_end(),
        "      ‹  October 2026   ›"
    );
    assert_eq!(style_at(&cells, 4, 4).bg, Style::default().bg);
    assert_eq!(style_at(&cells, 26, 4).bg, Style::default().bg);
}

#[test]
fn it_measures_its_fixed_size() {
    let picker = DatePicker::new(date(2026, 10, 14));
    assert_eq!(picker.measure(Size::new(40, 20)), Size::new(21, 8));
    assert_eq!(picker.measure(Size::new(10, 5)), Size::new(10, 5));
    assert_eq!((DatePicker::WIDTH, DatePicker::HEIGHT), (21, 8));
}
