// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Select, Widget, WidgetState};

const FRUIT: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];

fn draw(select: &Select<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    select.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(select: &Select<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(select, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn grid(select: &Select<'_>, width: u16, height: u16) -> Grid {
    draw(select, width, height).0
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn a_closed_select_draws_the_chosen_option_and_a_down_arrow() {
    let select = Select::new(&FRUIT, Some(2));
    assert_eq!(rows(&select, 20, 3), [" banana           ▾", "", ""]);
}

#[test]
fn no_choice_draws_the_placeholder_dim() {
    let select = Select::new(&FRUIT, None).placeholder("Pick a fruit");
    let cells = grid(&select, 20, 1);
    assert_eq!(rows(&select, 20, 1), [" Pick a fruit     ▾"]);
    assert!(style_at(&cells, 1, 0).attrs.dim());
    assert_eq!(style_at(&cells, 1, 0).fg, color(Token::TextPrimary));
}

#[test]
fn an_out_of_range_choice_draws_the_placeholder() {
    let select = Select::new(&FRUIT, Some(9)).placeholder("Pick a fruit");
    assert_eq!(rows(&select, 20, 1), [" Pick a fruit     ▾"]);
}

#[test]
fn an_open_select_lists_every_option_with_a_check_on_the_chosen_one() {
    let select = Select::new(&FRUIT, Some(2)).open(true);
    assert_eq!(
        rows(&select, 20, 6),
        [
            " banana           ▴",
            "   apple",
            "   apricot",
            " ✓ banana",
            "   blueberry",
            "   cherry",
        ]
    );
}

#[test]
fn the_active_row_is_drawn_on_the_accent_color() {
    let select = Select::new(&FRUIT, Some(2)).open(true).active(Some(1));
    let cells = grid(&select, 20, 6);
    assert_eq!(style_at(&cells, 3, 2).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 3, 2).fg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 3, 1).bg, color(Token::Surface0));
}

#[test]
fn an_out_of_range_active_row_highlights_nothing() {
    let select = Select::new(&FRUIT, Some(0)).open(true).active(Some(9));
    let cells = grid(&select, 20, 3);
    assert_eq!(style_at(&cells, 3, 1).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 3, 2).bg, color(Token::Surface0));
}

#[test]
fn the_list_scrolls_to_keep_the_active_row_in_view() {
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .active(Some(4))
        .max_rows(2);
    assert_eq!(rows(&select, 20, 3)[1..], ["   blueberry", "   cherry"]);
}

#[test]
fn max_rows_of_zero_still_shows_one_row() {
    let select = Select::new(&FRUIT, Some(0)).open(true).max_rows(0);
    assert_eq!(rows(&select, 20, 4)[1..], [" ✓ apple", "", ""]);
}

#[test]
fn an_open_select_with_no_options_says_so() {
    let none: [&str; 0] = [];
    let select = Select::new(&none, None).open(true);
    assert_eq!(rows(&select, 20, 3)[1], " No options");
    assert_eq!(select.option_at(Rect::new(0, 0, 20, 3), 3, 1), None);
}

#[test]
fn long_options_are_cut_to_the_field_and_list_width() {
    let words = ["abcdefghijklmnop"];
    let select = Select::new(&words, Some(0)).open(true);
    assert_eq!(rows(&select, 12, 2), [" abcdefgh ▴", " ✓ abcdefghi"]);
}

#[test]
fn narrow_fields_drop_the_arrow() {
    let select = Select::new(&FRUIT, Some(0));
    assert_eq!(rows(&select, 7, 1), [" apple"]);
    assert_eq!(rows(&select, 8, 1), [" appl ▾"]);
}

#[test]
fn disabled_dims_the_field_and_the_list() {
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .state(WidgetState::Disabled);
    let cells = grid(&select, 20, 3);
    assert!(style_at(&cells, 1, 0).attrs.dim());
    assert!(style_at(&cells, 3, 1).attrs.dim());
}

#[test]
fn disabled_dims_the_active_row_too() {
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .active(Some(0))
        .state(WidgetState::Disabled);
    let cells = grid(&select, 20, 3);
    let style = style_at(&cells, 3, 1);
    assert_eq!(style.bg, color(Token::Accent));
    assert!(style.attrs.dim());
}

#[test]
fn focused_draws_the_arrow_bold_in_the_accent_color() {
    let select = Select::new(&FRUIT, Some(0)).state(WidgetState::Focused);
    let cells = grid(&select, 20, 1);
    let arrow = style_at(&cells, 18, 0);
    assert!(arrow.attrs.bold());
    assert!(!arrow.attrs.dim());
    assert_eq!(arrow.fg, color(Token::Accent));
}

#[test]
fn field_and_list_areas_split_the_area() {
    let area = Rect::new(2, 4, 30, 4);
    let closed = Select::new(&FRUIT, Some(0));
    assert_eq!(closed.field_area(area), Rect::new(2, 4, 30, 1));
    assert_eq!(closed.list_area(area), Rect::new(2, 5, 30, 0));
    let open = closed.open(true);
    assert_eq!(open.list_area(area), Rect::new(2, 5, 30, 3));
}

#[test]
fn option_at_maps_a_click_to_the_option_under_it() {
    let area = Rect::new(2, 4, 30, 6);
    let select = Select::new(&FRUIT, Some(0)).open(true);
    assert_eq!(select.option_at(area, 2, 5), Some(0));
    assert_eq!(select.option_at(area, 31, 7), Some(2));
    assert_eq!(select.option_at(area, 10, 4), None);
    assert_eq!(select.option_at(area, 1, 5), None);
    assert_eq!(select.option_at(area, 32, 5), None);
    assert_eq!(Select::new(&FRUIT, Some(0)).option_at(area, 3, 5), None);
}

#[test]
fn option_at_accounts_for_scrolling() {
    let area = Rect::new(0, 0, 20, 3);
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .active(Some(4))
        .max_rows(2);
    assert_eq!(select.option_at(area, 3, 1), Some(3));
    assert_eq!(select.option_at(area, 3, 2), Some(4));
}

#[test]
fn measure_is_the_field_plus_the_visible_rows() {
    let closed = Select::new(&FRUIT, Some(0));
    assert_eq!(closed.measure(Size::new(20, 9)), Size::new(20, 1));
    assert_eq!(
        closed.clone().open(true).measure(Size::new(20, 9)),
        Size::new(20, 6)
    );
    assert_eq!(closed.measure(Size::new(0, 9)), Size::new(0, 0));
    assert_eq!(closed.measure(Size::new(20, 0)), Size::new(0, 0));
}

#[test]
fn step_moves_and_wraps() {
    assert_eq!(Select::<&str>::step(Some(1), 5, true), Some(2));
    assert_eq!(Select::<&str>::step(Some(4), 5, true), Some(0));
    assert_eq!(Select::<&str>::step(Some(0), 5, false), Some(4));
    assert_eq!(Select::<&str>::step(None, 5, true), Some(0));
    assert_eq!(Select::<&str>::step(None, 5, false), Some(4));
    assert_eq!(Select::<&str>::step(Some(9), 5, true), Some(0));
    assert_eq!(Select::<&str>::step(Some(0), 0, true), None);
}

#[test]
fn type_ahead_finds_the_next_option_with_the_prefix() {
    assert_eq!(Select::type_ahead(&FRUIT, None, "b"), Some(2));
    assert_eq!(Select::type_ahead(&FRUIT, Some(2), "b"), Some(3));
    assert_eq!(Select::type_ahead(&FRUIT, Some(3), "b"), Some(2));
    assert_eq!(Select::type_ahead(&FRUIT, Some(0), "ap"), Some(1));
    assert_eq!(Select::type_ahead(&FRUIT, Some(1), "ap"), Some(0));
}

#[test]
fn type_ahead_ignores_case_and_misses_cleanly() {
    assert_eq!(Select::type_ahead(&FRUIT, None, "CH"), Some(4));
    assert_eq!(Select::type_ahead(&FRUIT, None, "z"), None);
    assert_eq!(Select::type_ahead(&FRUIT, Some(2), ""), None);
    let words = ["Éclair", "eclair"];
    assert_eq!(Select::type_ahead(&words, None, "é"), Some(0));
    assert_eq!(Select::type_ahead(&words, Some(0), "e"), Some(1));
}

#[test]
fn zero_sized_areas_draw_nothing() {
    let select = Select::new(&FRUIT, Some(0)).open(true);
    assert_eq!(rows(&select, 0, 3), ["", "", ""]);
    assert_eq!(rows(&select, 20, 0), Vec::<String>::new());
    assert_eq!(select.option_at(Rect::new(0, 0, 0, 3), 0, 1), None);
}
