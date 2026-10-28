// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, SearchBox, SearchMatch, Widget, WidgetState};

fn draw(search: &SearchBox<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    search.render(area, &mut cx);
    (grid, clusters)
}

fn row(search: &SearchBox<'_>, width: u16) -> String {
    let (grid, clusters) = draw(search, Rect::new(0, 0, width, 1), width, 1);
    grid.row_display(0, &clusters)
}

fn styles(search: &SearchBox<'_>, width: u16) -> Grid {
    draw(search, Rect::new(0, 0, width, 1), width, 1).0
}

fn style_at(grid: &Grid, x: u16) -> Style {
    grid.get(x, 0).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn found(items: &[&str], query: &str) -> Vec<(usize, Vec<(usize, usize)>)> {
    SearchBox::filter(items, query)
        .into_iter()
        .map(|SearchMatch { index, ranges }| {
            (
                index,
                ranges.into_iter().map(|r| (r.start, r.end)).collect(),
            )
        })
        .collect()
}

#[test]
fn the_row_shows_a_prompt_the_query_and_a_clear_button() {
    assert_eq!(row(&SearchBox::new("table", 5), 20), " / table          × ");
    assert_eq!(row(&SearchBox::new("", 0), 20), " /                  ");
}

#[test]
fn a_placeholder_shows_only_while_the_query_is_empty() {
    let empty = SearchBox::new("", 0).placeholder("Filter widgets");
    assert_eq!(row(&empty, 20), " / Filter widgets   ");
    let typed = SearchBox::new("a", 1).placeholder("Filter widgets");
    assert_eq!(row(&typed, 20), " / a              × ");
    let grid = styles(&empty, 20);
    assert!(style_at(&grid, 5).attrs.dim());
}

#[test]
fn the_count_is_right_aligned_before_the_clear_button() {
    let some = SearchBox::new("ta", 2).matches(Some(2), 12);
    assert_eq!(row(&some, 30), " / ta              3 of 12  × ");
    let total = SearchBox::new("ta", 2).matches(None, 12);
    assert_eq!(row(&total, 30), " / ta           12 matches  × ");
    let one = SearchBox::new("ta", 2).matches(None, 1);
    assert_eq!(row(&one, 30), " / ta              1 match  × ");
    let none = SearchBox::new("zz", 2).matches(None, 0);
    assert_eq!(row(&none, 30), " / zz           No matches  × ");
}

#[test]
fn the_count_shows_without_a_clear_button_while_the_query_is_empty() {
    let empty = SearchBox::new("", 0).matches(None, 7);
    assert_eq!(row(&empty, 24), " /            7 matches ");
}

#[test]
fn the_count_is_dropped_when_it_would_crowd_the_query() {
    let tight = SearchBox::new("table", 5).matches(Some(0), 3);
    assert_eq!(row(&tight, 14), " / table    × ");
    let roomy = SearchBox::new("table", 5).matches(Some(0), 3);
    assert_eq!(row(&roomy, 24), " / table      1 of 3  × ");
}

#[test]
fn narrow_rows_drop_the_clear_button() {
    let search = SearchBox::new("table", 5);
    assert_eq!(search.clear_area(Rect::new(0, 0, 11, 1)).width, 0);
    assert_eq!(row(&search, 11), " / table   ");
    assert_eq!(search.clear_area(Rect::new(0, 0, 12, 1)).width, 3);
}

#[test]
fn the_areas_split_the_row() {
    let search = SearchBox::new("table", 5).matches(Some(0), 3);
    let area = Rect::new(2, 4, 30, 2);
    assert_eq!(search.clear_area(area), Rect::new(29, 4, 3, 1));
    assert_eq!(search.query_area(area), Rect::new(5, 4, 16, 1));
    let empty = SearchBox::new("", 0);
    assert_eq!(empty.clear_area(area), Rect::new(32, 4, 0, 1));
    assert_eq!(empty.query_area(area), Rect::new(5, 4, 27, 1));
}

#[test]
fn long_queries_are_clipped_to_the_field() {
    let search = SearchBox::new("a very long query indeed", 24).matches(Some(0), 3);
    assert_eq!(row(&search, 24), " / a very lon 1 of 3  × ");
}

#[test]
fn colors_follow_the_state() {
    let search = SearchBox::new("ab", 2).matches(None, 0);
    let grid = styles(&search, 30);
    assert_eq!(style_at(&grid, 0).bg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 1).fg, color(Token::Accent));
    assert!(style_at(&grid, 1).attrs.dim());
    assert_eq!(style_at(&grid, 3).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 16).fg, color(Token::Fail));
    assert_eq!(style_at(&grid, 28).fg, color(Token::Accent));
    let focused = styles(&search.clone().state(WidgetState::Focused), 30);
    assert!(!style_at(&focused, 1).attrs.dim());
    assert!(style_at(&focused, 1).attrs.bold());
    let disabled = styles(&search.state(WidgetState::Disabled), 30);
    for x in [1, 3, 16, 28] {
        assert!(style_at(&disabled, x).attrs.dim(), "{x}");
    }
    assert_eq!(style_at(&disabled, 16).fg, color(Token::TextPrimary));
}

#[test]
fn a_positive_count_is_dimmed_not_failing() {
    let search = SearchBox::new("ab", 2).matches(Some(0), 4);
    let grid = styles(&search, 30);
    assert_eq!(style_at(&grid, 21).fg, color(Token::TextPrimary));
    assert!(style_at(&grid, 21).attrs.dim());
}

#[test]
fn an_explicit_style_colors_the_query() {
    let chosen = Color::Rgb(1, 2, 3);
    let search = SearchBox::new("ab", 2).style(Style::new().fg(chosen));
    let grid = styles(&search, 20);
    assert_eq!(style_at(&grid, 3).fg, chosen);
    assert_eq!(style_at(&grid, 1).fg, color(Token::Accent));
}

#[test]
fn the_caret_is_an_inverted_cell_when_focused() {
    let search = SearchBox::new("table", 2).state(WidgetState::Focused);
    let grid = styles(&search, 20);
    assert_eq!(style_at(&grid, 5).bg, color(Token::Accent));
    assert_eq!(style_at(&grid, 5).fg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 4).bg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 6).bg, color(Token::Surface1));
    let plain = styles(&SearchBox::new("table", 2), 20);
    assert_eq!(style_at(&plain, 5).bg, color(Token::Surface1));
}

#[test]
fn the_caret_at_the_end_or_in_an_empty_field_sits_on_a_blank_cell() {
    let end = styles(&SearchBox::new("ab", 2).state(WidgetState::Focused), 20);
    assert_eq!(style_at(&end, 5).bg, color(Token::Accent));
    let empty = SearchBox::new("", 0)
        .placeholder("Filter")
        .state(WidgetState::Focused);
    let grid = styles(&empty, 20);
    assert_eq!(style_at(&grid, 3).bg, color(Token::Accent));
    let past = styles(&SearchBox::new("ab", 99).state(WidgetState::Focused), 20);
    assert_eq!(style_at(&past, 5).bg, color(Token::Accent));
}

#[test]
fn the_caret_moves_back_to_a_character_boundary() {
    let search = SearchBox::new("é!", 1).state(WidgetState::Focused);
    let grid = styles(&search, 20);
    assert_eq!(style_at(&grid, 3).bg, color(Token::Accent));
    assert_eq!(style_at(&grid, 4).bg, color(Token::Surface1));
}

#[test]
fn filter_matches_terms_in_any_order_and_reports_ranges() {
    let items = ["Slider widget", "Table", "Toggle switch", "slider track"];
    assert_eq!(found(&items, "sl"), [(0, vec![(0, 2)]), (3, vec![(0, 2)])]);
    assert_eq!(found(&items, "wid sli"), [(0, vec![(0, 3), (7, 10)])]);
    assert_eq!(found(&items, "zzz"), []);
}

#[test]
fn filter_uses_smart_case() {
    let items = ["Table", "table", "TABLE"];
    assert_eq!(found(&items, "table").len(), 3);
    assert_eq!(found(&items, "Table"), [(0, vec![(0, 5)])]);
    assert_eq!(found(&items, "TABLE"), [(2, vec![(0, 5)])]);
    assert_eq!(found(&items, "tAble"), []);
}

#[test]
fn filter_returns_every_item_for_an_empty_query() {
    let items = ["a", "b"];
    assert_eq!(found(&items, ""), [(0, vec![]), (1, vec![])]);
    assert_eq!(found(&items, "   "), [(0, vec![]), (1, vec![])]);
    assert_eq!(found(&Vec::<&str>::new(), "a"), []);
}

#[test]
fn filter_finds_every_occurrence_and_merges_overlaps() {
    let items = ["abab", "aaaa"];
    assert_eq!(found(&items, "ab"), [(0, vec![(0, 4)])]);
    assert_eq!(found(&items, "aa"), [(1, vec![(0, 4)])]);
    assert_eq!(found(&["xaxaxa"], "a"), [(0, vec![(1, 2), (3, 4), (5, 6)])]);
    assert_eq!(found(&["abcd"], "bc ab"), [(0, vec![(0, 3)])]);
    assert_eq!(found(&["abcd"], "cd ab"), [(0, vec![(0, 4)])]);
}

#[test]
fn filter_handles_non_ascii_text() {
    let items = ["Crème brûlée", "creme"];
    assert_eq!(found(&items, "brûl"), [(0, vec![(7, 12)])]);
    assert_eq!(found(&items, "crème"), [(0, vec![(0, 6)])]);
    assert_eq!(found(&items, "creme"), [(1, vec![(0, 5)])]);
}

#[test]
fn filter_works_on_owned_strings() {
    let items = vec![String::from("alpha"), String::from("beta")];
    assert_eq!(SearchBox::filter(&items, "et")[0].index, 1);
}

#[test]
fn step_wraps_through_the_matches() {
    assert_eq!(SearchBox::step(None, 3, true), Some(0));
    assert_eq!(SearchBox::step(None, 3, false), Some(2));
    assert_eq!(SearchBox::step(Some(0), 3, true), Some(1));
    assert_eq!(SearchBox::step(Some(2), 3, true), Some(0));
    assert_eq!(SearchBox::step(Some(0), 3, false), Some(2));
    assert_eq!(SearchBox::step(Some(2), 3, false), Some(1));
    assert_eq!(SearchBox::step(Some(3), 3, true), Some(0));
    assert_eq!(SearchBox::step(Some(3), 3, false), Some(2));
    assert_eq!(SearchBox::step(Some(9), 3, true), Some(0));
    assert_eq!(SearchBox::step(Some(9), 3, false), Some(2));
    assert_eq!(SearchBox::step(Some(0), 0, true), None);
    assert_eq!(SearchBox::step(None, 0, false), None);
    assert_eq!(SearchBox::step(Some(0), 1, true), Some(0));
}

fn highlighted(text: &str, ranges: &[std::ops::Range<usize>], width: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(14, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let base = Style::new().fg(Color::Rgb(1, 1, 1));
    let hit = Style::new().fg(Color::Rgb(2, 2, 2));
    SearchBox::put_highlighted(&mut cx, (1, 0), text, ranges, (base, hit), width);
    (grid, clusters)
}

#[test]
fn highlighted_text_draws_the_ranges_in_the_hit_style() {
    let (grid, clusters) = highlighted("slider track", &[0..2, 7..9], 12);
    assert_eq!(grid.row_display(0, &clusters), " slider track ");
    let fg = |x| style_at(&grid, x).fg;
    assert_eq!(fg(1), Color::Rgb(2, 2, 2));
    assert_eq!(fg(2), Color::Rgb(2, 2, 2));
    assert_eq!(fg(3), Color::Rgb(1, 1, 1));
    assert_eq!(fg(7), Color::Rgb(1, 1, 1));
    assert_eq!(fg(8), Color::Rgb(2, 2, 2));
    assert_eq!(fg(9), Color::Rgb(2, 2, 2));
    assert_eq!(fg(10), Color::Rgb(1, 1, 1));
}

#[test]
fn highlighted_text_is_cut_to_the_width_and_skips_bad_ranges() {
    let (grid, clusters) = highlighted("slider track", &[0..2, 7..9], 5);
    assert_eq!(grid.row_display(0, &clusters), " slide        ");
    let (grid, clusters) = highlighted("slider", &[4..99, 3..3, 1..2, 0..1], 12);
    assert_eq!(grid.row_display(0, &clusters), " slider       ");
    assert_eq!(style_at(&grid, 5).fg, Color::Rgb(2, 2, 2));
    assert_eq!(style_at(&grid, 6).fg, Color::Rgb(2, 2, 2));
    assert_eq!(style_at(&grid, 4).fg, Color::Rgb(1, 1, 1));
    assert_eq!(style_at(&grid, 2).fg, Color::Rgb(1, 1, 1));
    let (grid, clusters) = highlighted("日本語", &[3..6, 3..6], 5);
    assert_eq!(grid.row_display(0, &clusters), " 日本         ");
    assert_eq!(style_at(&grid, 3).fg, Color::Rgb(2, 2, 2));
}

#[test]
fn nothing_is_drawn_for_empty_areas_or_outside_the_area() {
    let search = SearchBox::new("ab", 2);
    let (grid, clusters) = draw(&search, Rect::new(0, 0, 0, 1), 12, 2);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = draw(&search, Rect::new(0, 0, 12, 0), 12, 2);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = draw(&search, Rect::new(2, 1, 8, 3), 12, 4);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(grid.row_display(1, &clusters), "   / ab     ");
    assert_eq!(grid.row_display(2, &clusters).trim(), "");
}

#[test]
fn a_one_column_row_still_draws_the_prompt() {
    assert_eq!(row(&SearchBox::new("ab", 2), 1), "/");
    assert_eq!(row(&SearchBox::new("ab", 2), 2), " /");
}

#[test]
fn measure_is_the_width_on_one_row() {
    let search = SearchBox::new("a", 1);
    assert_eq!(search.measure(Size::new(30, 5)), Size::new(30, 1));
    assert_eq!(search.measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(search.measure(Size::new(5, 0)), Size::new(0, 0));
}

#[test]
fn the_query_field_gives_up_the_count_when_it_would_leave_no_room() {
    let search = SearchBox::new("table", 5).matches(Some(0), 3);
    let width_at = |width| search.query_area(Rect::new(0, 0, width, 1)).width;
    assert_eq!(width_at(14), 8);
    assert_eq!(width_at(15), 1);
    assert_eq!(width_at(16), 2);
    let empty = SearchBox::new("", 0).matches(None, 7);
    let width_at = |width| empty.query_area(Rect::new(0, 0, width, 1)).width;
    assert_eq!(width_at(14), 11);
    assert_eq!(width_at(15), 1);
    assert_eq!(width_at(2), 0);
}

#[test]
fn the_caret_is_not_drawn_past_the_end_of_the_field() {
    let full = SearchBox::new("abcdef", 6).state(WidgetState::Focused);
    let grid = styles(&full, 12);
    assert_eq!(row(&full, 12), " / abcdef × ");
    assert_eq!(style_at(&grid, 9).bg, color(Token::Surface1));
    let inside = SearchBox::new("abcdef", 5).state(WidgetState::Focused);
    assert_eq!(style_at(&styles(&inside, 12), 8).bg, color(Token::Accent));
}
