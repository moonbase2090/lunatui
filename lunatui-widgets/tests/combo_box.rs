// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{ComboBox, RenderCx, Widget, WidgetState};

const EMPTY_OPEN: &str = "                  ▴";
const FRUIT: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];

fn draw(combo: &ComboBox<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    combo.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(combo: &ComboBox<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(combo, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn grid(combo: &ComboBox<'_>, width: u16, height: u16) -> Grid {
    draw(combo, width, height).0
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn found(combo: &ComboBox<'_>) -> Vec<(usize, Vec<(usize, usize)>)> {
    combo
        .matches()
        .into_iter()
        .map(|hit| {
            (
                hit.index,
                hit.ranges.into_iter().map(|r| (r.start, r.end)).collect(),
            )
        })
        .collect()
}

#[test]
fn a_closed_box_draws_only_the_field_and_a_down_arrow() {
    let combo = ComboBox::new("ap", 2, &FRUIT);
    assert_eq!(rows(&combo, 20, 4), [" ap               ▾", "", "", ""]);
}

#[test]
fn an_open_box_lists_the_options_that_match() {
    let combo = ComboBox::new("a", 1, &FRUIT).open(true);
    assert_eq!(
        rows(&combo, 20, 5),
        [" a                ▴", " apple", " apricot", " banana", ""]
    );
    assert_eq!(found(&combo).len(), 3);
}

#[test]
fn matched_text_is_highlighted_in_the_list() {
    let combo = ComboBox::new("ap", 2, &FRUIT).open(true);
    let cells = grid(&combo, 20, 4);
    let mark = style_at(&cells, 1, 1);
    assert_eq!(mark.fg, color(Token::Surface0));
    assert_eq!(mark.bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 2, 1), mark);
    let plain = style_at(&cells, 3, 1);
    assert_eq!(plain.fg, color(Token::TextPrimary));
    assert_eq!(plain.bg, color(Token::Surface0));
}

#[test]
fn the_active_row_is_filled_with_the_accent_color() {
    let combo = ComboBox::new("ap", 2, &FRUIT).open(true).active(Some(1));
    let cells = grid(&combo, 20, 4);
    for x in [0, 5, 19] {
        let style = style_at(&cells, x, 2);
        assert_eq!(style.bg, color(Token::Accent), "{x}");
        assert_eq!(style.fg, color(Token::Surface0), "{x}");
    }
    let hit = style_at(&cells, 1, 2);
    let mark = style_at(&cells, 2, 2);
    assert_eq!(hit, mark);
    assert_eq!(style_at(&cells, 3, 1).bg, color(Token::Surface0));
    let inverted = ComboBox::new("ap", 2, &FRUIT).open(true).active(Some(0));
    let cells = grid(&inverted, 20, 4);
    assert_eq!(style_at(&cells, 1, 1).fg, color(Token::Accent));
    assert_eq!(style_at(&cells, 1, 1).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 3, 1).bg, color(Token::Accent));
}

#[test]
fn an_active_position_past_the_matches_highlights_nothing() {
    let combo = ComboBox::new("ap", 2, &FRUIT).open(true).active(Some(9));
    let cells = grid(&combo, 20, 4);
    for y in 1..3 {
        assert_eq!(style_at(&cells, 10, y).bg, color(Token::Surface0), "{y}");
    }
}

#[test]
fn the_list_scrolls_to_keep_the_active_row_in_view() {
    let combo = ComboBox::new("", 0, &FRUIT)
        .open(true)
        .max_rows(3)
        .active(Some(4));
    assert_eq!(
        rows(&combo, 20, 6),
        [EMPTY_OPEN, " banana", " blueberry", " cherry", "", ""]
    );
    let top = ComboBox::new("", 0, &FRUIT)
        .open(true)
        .max_rows(3)
        .active(Some(2));
    assert_eq!(rows(&top, 20, 4)[1..], [" apple", " apricot", " banana"]);
    let none = ComboBox::new("", 0, &FRUIT).open(true).max_rows(3);
    assert_eq!(rows(&none, 20, 4)[1..], [" apple", " apricot", " banana"]);
}

#[test]
fn the_list_is_cut_to_the_area_height() {
    let combo = ComboBox::new("", 0, &FRUIT).open(true);
    assert_eq!(rows(&combo, 20, 3)[1..], [" apple", " apricot"]);
    assert_eq!(rows(&combo, 20, 1), [EMPTY_OPEN]);
    let combo = combo.max_rows(0);
    assert_eq!(rows(&combo, 20, 4)[1..], [" apple", "", ""]);
}

#[test]
fn an_empty_result_says_no_matches() {
    let combo = ComboBox::new("zz", 2, &FRUIT).open(true).max_rows(3);
    let cells = grid(&combo, 20, 4);
    assert_eq!(rows(&combo, 20, 4)[1..], [" No matches", "", ""]);
    assert!(style_at(&cells, 1, 1).attrs.dim());
    assert_eq!(style_at(&cells, 15, 1).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 15, 2), Style::default());
}

#[test]
fn matching_follows_the_search_box_rules() {
    let combo = ComboBox::new("B e", 3, &FRUIT);
    assert_eq!(found(&combo), []);
    let lower = ComboBox::new("b e", 3, &FRUIT);
    assert_eq!(found(&lower), [(3, vec![(0, 1), (3, 6)])]);
    let all = ComboBox::new("", 0, &FRUIT);
    assert_eq!(found(&all).len(), 5);
}

#[test]
fn a_placeholder_shows_only_while_the_field_is_empty() {
    let empty = ComboBox::new("", 0, &FRUIT).placeholder("Pick a fruit");
    assert_eq!(rows(&empty, 20, 1), [" Pick a fruit     ▾"]);
    assert!(style_at(&grid(&empty, 20, 1), 1, 0).attrs.dim());
    let typed = ComboBox::new("b", 1, &FRUIT).placeholder("Pick a fruit");
    assert_eq!(rows(&typed, 20, 1), [" b                ▾"]);
}

#[test]
fn the_arrow_is_bold_and_accented_when_focused() {
    let idle = grid(&ComboBox::new("", 0, &FRUIT), 20, 1);
    assert!(style_at(&idle, 18, 0).attrs.dim());
    assert!(!style_at(&idle, 18, 0).attrs.bold());
    let focused = ComboBox::new("", 0, &FRUIT).state(WidgetState::Focused);
    let cells = grid(&focused, 20, 1);
    assert_eq!(style_at(&cells, 18, 0).fg, color(Token::Accent));
    assert!(style_at(&cells, 18, 0).attrs.bold());
    assert!(!style_at(&cells, 18, 0).attrs.dim());
}

#[test]
fn the_focused_caret_inverts_the_cell_under_it() {
    let combo = ComboBox::new("ap", 1, &FRUIT).state(WidgetState::Focused);
    let cells = grid(&combo, 20, 1);
    assert_eq!(style_at(&cells, 2, 0).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 2, 0).fg, color(Token::Surface1));
    assert_eq!(style_at(&cells, 1, 0).bg, color(Token::Surface1));
    let end = ComboBox::new("ap", 9, &FRUIT).state(WidgetState::Focused);
    let cells = grid(&end, 20, 1);
    assert_eq!(style_at(&cells, 3, 0).bg, color(Token::Accent));
    let unfocused = grid(&ComboBox::new("ap", 1, &FRUIT), 20, 1);
    assert_eq!(style_at(&unfocused, 2, 0).bg, color(Token::Surface1));
}

#[test]
fn the_caret_moves_back_to_a_character_boundary() {
    let words = ["éa"];
    let combo = ComboBox::new("éa", 1, &words).state(WidgetState::Focused);
    let cells = grid(&combo, 20, 1);
    assert_eq!(style_at(&cells, 1, 0).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 2, 0).bg, color(Token::Surface1));
}

#[test]
fn a_caret_beyond_the_field_is_not_drawn() {
    let combo = ComboBox::new("abcdefghijklmnop", 16, &FRUIT).state(WidgetState::Focused);
    let cells = grid(&combo, 12, 1);
    for x in 0..12 {
        assert_ne!(style_at(&cells, x, 0).bg, color(Token::Accent), "{x}");
    }
}

#[test]
fn long_text_is_clipped_before_the_arrow() {
    let combo = ComboBox::new("abcdefghijklmnop", 0, &FRUIT);
    assert_eq!(rows(&combo, 12, 1), [" abcdefgh ▾"]);
}

#[test]
fn narrow_rows_drop_the_arrow() {
    let combo = ComboBox::new("abcdefghijklmnop", 0, &FRUIT);
    assert_eq!(rows(&combo, 7, 1), [" abcde"]);
    assert_eq!(rows(&combo, 8, 1), [" abcd ▾"]);
}

#[test]
fn disabled_dims_the_field_and_the_list() {
    let combo = ComboBox::new("ap", 2, &FRUIT)
        .open(true)
        .state(WidgetState::Disabled);
    let cells = grid(&combo, 20, 3);
    for (x, y) in [(1, 0), (18, 0), (1, 1), (5, 2)] {
        assert!(style_at(&cells, x, y).attrs.dim(), "{x},{y}");
    }
    let normal = grid(&ComboBox::new("ap", 2, &FRUIT).open(true), 20, 3);
    assert!(!style_at(&normal, 5, 2).attrs.dim());
}

#[test]
fn the_areas_split_the_box() {
    let combo = ComboBox::new("ap", 2, &FRUIT).open(true);
    let area = Rect::new(2, 4, 30, 8);
    assert_eq!(combo.field_area(area), Rect::new(2, 4, 30, 1));
    assert_eq!(combo.list_area(area), Rect::new(2, 5, 30, 2));
    let closed = ComboBox::new("ap", 2, &FRUIT);
    assert_eq!(closed.list_area(area), Rect::new(2, 5, 30, 0));
    let none = ComboBox::new("zz", 2, &FRUIT).open(true);
    assert_eq!(none.list_area(area), Rect::new(2, 5, 30, 1));
}

#[test]
fn option_at_maps_cells_to_matches() {
    let combo = ComboBox::new("a", 1, &FRUIT).open(true);
    let area = Rect::new(2, 4, 30, 8);
    assert_eq!(combo.option_at(area, 2, 5), Some(0));
    assert_eq!(combo.option_at(area, 31, 7), Some(2));
    assert_eq!(combo.option_at(area, 10, 6), Some(1));
    assert_eq!(combo.option_at(area, 10, 4), None);
    assert_eq!(combo.option_at(area, 10, 8), None);
    assert_eq!(combo.option_at(area, 1, 5), None);
    assert_eq!(combo.option_at(area, 32, 5), None);
    let closed = ComboBox::new("a", 1, &FRUIT);
    assert_eq!(closed.option_at(area, 10, 5), None);
}

#[test]
fn option_at_accounts_for_scrolling() {
    let combo = ComboBox::new("", 0, &FRUIT)
        .open(true)
        .max_rows(2)
        .active(Some(3));
    let area = Rect::new(0, 0, 20, 6);
    assert_eq!(combo.option_at(area, 3, 1), Some(2));
    assert_eq!(combo.option_at(area, 3, 2), Some(3));
}

#[test]
fn measure_counts_the_field_and_the_list() {
    let avail = Size::new(30, 10);
    assert_eq!(
        ComboBox::new("", 0, &FRUIT).measure(avail),
        Size::new(30, 1)
    );
    let open = ComboBox::new("", 0, &FRUIT).open(true);
    assert_eq!(open.measure(avail), Size::new(30, 6));
    let few = ComboBox::new("ch", 2, &FRUIT).open(true);
    assert_eq!(few.measure(avail), Size::new(30, 2));
    let none = ComboBox::new("zz", 2, &FRUIT).open(true);
    assert_eq!(none.measure(avail), Size::new(30, 2));
    assert_eq!(open.measure(Size::new(30, 4)), Size::new(30, 4));
    assert_eq!(open.measure(Size::new(0, 4)), Size::new(0, 0));
    assert_eq!(open.measure(Size::new(4, 0)), Size::new(0, 0));
}

#[test]
fn an_empty_area_draws_nothing() {
    let combo = ComboBox::new("ap", 2, &FRUIT).open(true);
    let theme = Theme::builtin(Variant::Dark);
    let mut cells = Grid::new(10, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut cells, &mut clusters, &theme);
    combo.render(Rect::new(0, 0, 0, 3), &mut cx);
    combo.render(Rect::new(0, 0, 10, 0), &mut cx);
    assert_eq!(cells, Grid::new(10, 3));
}
