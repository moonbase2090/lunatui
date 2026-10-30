// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, TagInput, Widget, WidgetState};

const TAGS: [&str; 3] = ["rust", "tui", "async"];

fn draw(input: &TagInput<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    input.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(input: &TagInput<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(input, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn input<'a>(text: &'a str, caret: usize) -> TagInput<'a> {
    TagInput::new(&TAGS, text, caret)
}

#[test]
fn chips_and_the_field_share_a_row_while_they_fit() {
    assert_eq!(
        rows(&input("ru", 2), 40, 2),
        [" rust ×   tui ×   async ×  ru", ""]
    );
}

#[test]
fn chips_wrap_to_the_next_row_and_the_field_follows_the_last_chip() {
    assert_eq!(
        rows(&input("ru", 2), 24, 3),
        [" rust ×   tui ×", " async ×  ru", ""]
    );
}

#[test]
fn a_field_with_too_little_room_starts_its_own_row() {
    assert_eq!(
        rows(&input("ru", 2), 14, 5),
        [" rust ×", " tui ×", " async ×", "ru", ""]
    );
    assert_eq!(rows(&input("ru", 2), 17, 3)[1], " async ×");
    assert_eq!(rows(&input("ru", 2), 16, 3)[2], "ru");
}

#[test]
fn a_chip_wider_than_the_area_is_clipped() {
    assert_eq!(
        rows(&input("ru", 2), 6, 5),
        [" ru ×", " tu ×", " as ×", "ru", ""]
    );
}

#[test]
fn a_placeholder_shows_only_without_tags_and_text() {
    let none: [&str; 0] = [];
    let empty = TagInput::new(&none, "", 0).placeholder("Add a tag");
    assert_eq!(rows(&empty, 20, 1), ["Add a tag"]);
    let typed = TagInput::new(&none, "go", 2).placeholder("Add a tag");
    assert_eq!(rows(&typed, 20, 1), ["go"]);
    let tagged = input("", 0).placeholder("Add a tag");
    assert_eq!(rows(&tagged, 40, 1), [" rust ×   tui ×   async ×"]);
    assert_eq!(rows(&empty, 4, 1), ["Add"]);
}

#[test]
fn long_text_is_clipped_to_the_field() {
    assert_eq!(
        rows(&input("abcdefghijkl", 0), 40, 1)[0],
        " rust ×   tui ×   async ×  abcdefghijkl"
    );
    assert_eq!(
        rows(&input("abcdefghijkl", 0), 30, 2)[0],
        " rust ×   tui ×   async ×"
    );
    assert_eq!(rows(&input("abcdefghijkl", 0), 30, 2)[1], "abcdefghijkl");
    assert_eq!(rows(&input("abcdefghijkl", 0), 12, 4)[3], "abcdefghijkl");
    assert_eq!(rows(&input("abcdefghijkl", 0), 8, 5)[3], "abcdefgh");
}

#[test]
fn the_focused_field_draws_the_caret_inverted() {
    let focused = input("ru", 1).state(WidgetState::Focused);
    let (grid, _) = draw(&focused, 40, 1);
    let at = style_at(&grid, 28, 0);
    assert_eq!(at.fg, color(Token::Surface0));
    assert_eq!(at.bg, color(Token::Accent));
    assert_eq!(style_at(&grid, 27, 0).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 29, 0).bg, color(Token::Surface0));
    let end = input("ru", 2).state(WidgetState::Focused);
    let (grid, _) = draw(&end, 40, 1);
    assert_eq!(style_at(&grid, 29, 0).bg, color(Token::Accent));
    let past = input("ru", 99).state(WidgetState::Focused);
    let (grid, _) = draw(&past, 40, 1);
    assert_eq!(style_at(&grid, 29, 0).bg, color(Token::Accent));
    let normal = input("ru", 1);
    let (grid, _) = draw(&normal, 40, 1);
    assert_eq!(style_at(&grid, 28, 0).bg, color(Token::Surface0));
}

#[test]
fn the_caret_stays_on_a_character_boundary() {
    let focused = input("é", 1).state(WidgetState::Focused);
    let (grid, clusters) = draw(&focused, 40, 1);
    assert_eq!(
        grid.row_display(0, &clusters).trim_end(),
        " rust ×   tui ×   async ×  é"
    );
    assert_eq!(style_at(&grid, 27, 0).bg, color(Token::Accent));
}

#[test]
fn the_caret_is_not_drawn_past_the_end_of_the_field() {
    let none: [&str; 0] = [];
    let full = TagInput::new(&none, "abcdefgh", 8).state(WidgetState::Focused);
    let (grid, _) = draw(&full, 8, 1);
    for x in 0..8 {
        assert_eq!(style_at(&grid, x, 0).bg, color(Token::Surface0));
    }
}

#[test]
fn a_selected_chip_is_highlighted_and_hides_the_caret() {
    let chosen = input("ru", 2).selected(Some(1)).state(WidgetState::Focused);
    let (grid, _) = draw(&chosen, 40, 1);
    let accent = color(Token::Accent);
    for x in 9..16 {
        assert_eq!(style_at(&grid, x, 0).bg, accent, "column {x}");
    }
    assert_eq!(style_at(&grid, 10, 0).fg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 7, 0).bg, color(Token::Surface1));
    assert_eq!(style_at(&grid, 8, 0).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 16, 0).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 28, 0).bg, color(Token::Surface0));
}

#[test]
fn chips_use_the_raised_surface_and_a_dim_cross() {
    let (grid, _) = draw(&input("", 0), 40, 1);
    let text = style_at(&grid, 2, 0);
    assert_eq!(text.fg, color(Token::TextPrimary));
    assert_eq!(text.bg, color(Token::Surface1));
    assert!(!text.attrs.dim());
    let cross = style_at(&grid, 6, 0);
    assert_eq!(cross.bg, color(Token::Surface1));
    assert!(cross.attrs.dim());
    for x in 0..8 {
        assert_eq!(
            style_at(&grid, x, 0).bg,
            color(Token::Surface1),
            "column {x}"
        );
    }
    assert_eq!(style_at(&grid, 8, 0).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 20, 1).bg, Style::default().bg);
}

#[test]
fn a_disabled_input_dims_chips_and_the_field() {
    let (grid, _) = draw(&input("ru", 2).state(WidgetState::Disabled), 40, 1);
    assert!(style_at(&grid, 2, 0).attrs.dim());
    assert!(style_at(&grid, 27, 0).attrs.dim());
    assert!(!style_at(&grid, 27, 0).bg.eq(&color(Token::Accent)));
}

#[test]
fn rows_and_measure_report_what_the_chips_need() {
    let input = input("", 0);
    assert_eq!(input.rows(40), 1);
    assert_eq!(input.rows(24), 2);
    assert_eq!(input.rows(14), 4);
    assert_eq!(input.rows(0), 0);
    assert_eq!(input.measure(Size::new(24, 9)), Size::new(24, 2));
    assert_eq!(input.measure(Size::new(14, 3)), Size::new(14, 3));
    assert_eq!(input.measure(Size::new(0, 3)), Size::new(0, 0));
    assert_eq!(input.measure(Size::new(10, 0)), Size::new(0, 0));
}

#[test]
fn tag_at_and_remove_at_find_chips_and_their_crosses() {
    let input = input("", 0);
    let area = Rect::new(2, 1, 24, 3);
    assert_eq!(input.tag_at(area, 2, 1), Some(0));
    assert_eq!(input.tag_at(area, 9, 1), Some(0));
    assert_eq!(input.tag_at(area, 10, 1), None);
    assert_eq!(input.tag_at(area, 11, 1), Some(1));
    assert_eq!(input.tag_at(area, 1, 1), None);
    assert_eq!(input.tag_at(area, 2, 0), None);
    assert_eq!(input.tag_at(area, 4, 2), Some(2));
    assert_eq!(input.tag_at(area, 4, 3), None);
    assert_eq!(input.remove_at(area, 8, 1), Some(0));
    assert_eq!(input.remove_at(area, 7, 1), None);
    assert_eq!(input.remove_at(area, 9, 1), None);
    assert_eq!(input.remove_at(area, 16, 1), Some(1));
    assert_eq!(input.remove_at(area, 9, 2), Some(2));
    assert_eq!(input.tag_at(Rect::new(0, 0, 0, 3), 0, 0), None);
    assert_eq!(input.tag_at(Rect::new(0, 0, 20, 0), 0, 0), None);
}

#[test]
fn a_clipped_chip_keeps_its_cross() {
    let input = input("", 0);
    let area = Rect::new(0, 0, 6, 5);
    assert_eq!(input.remove_at(area, 4, 0), Some(0));
    assert_eq!(input.remove_at(area, 3, 0), None);
}

#[test]
fn field_area_is_the_rest_of_the_last_chip_row() {
    let input = input("", 0);
    assert_eq!(
        input.field_area(Rect::new(2, 1, 40, 2)),
        Rect::new(29, 1, 13, 1)
    );
    assert_eq!(
        input.field_area(Rect::new(2, 1, 24, 3)),
        Rect::new(12, 2, 14, 1)
    );
    assert_eq!(
        input.field_area(Rect::new(2, 1, 14, 5)),
        Rect::new(2, 4, 14, 1)
    );
    assert_eq!(
        input.field_area(Rect::new(2, 1, 0, 5)),
        Rect::new(2, 1, 0, 0)
    );
    assert_eq!(
        input.field_area(Rect::new(2, 1, 14, 0)),
        Rect::new(2, 1, 0, 0)
    );
}

#[test]
fn too_many_rows_show_the_last_ones_and_hit_testing_follows() {
    let input = input("ru", 2);
    assert_eq!(rows(&input, 14, 2), [" async ×", "ru"]);
    let area = Rect::new(0, 5, 14, 2);
    assert_eq!(input.tag_at(area, 2, 5), Some(2));
    assert_eq!(input.tag_at(area, 2, 6), None);
    assert_eq!(input.tag_at(area, 2, 4), None);
    assert_eq!(input.field_area(area), Rect::new(0, 6, 14, 1));
    assert_eq!(input.remove_at(area, 7, 5), Some(2));
    assert_eq!(rows(&input, 14, 1), ["ru"]);
    assert_eq!(
        input.field_area(Rect::new(0, 5, 14, 1)),
        Rect::new(0, 5, 14, 1)
    );
}

#[test]
fn an_empty_area_draws_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    input("ru", 2).render(Rect::new(0, 0, 0, 3), &mut cx);
    input("ru", 2).render(Rect::new(0, 0, 10, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
}

#[test]
fn step_moves_between_the_field_and_the_chips() {
    assert_eq!(TagInput::<&str>::step(None, 3, true), Some(2));
    assert_eq!(TagInput::<&str>::step(None, 0, true), None);
    assert_eq!(TagInput::<&str>::step(None, 3, false), None);
    assert_eq!(TagInput::<&str>::step(Some(2), 3, true), Some(1));
    assert_eq!(TagInput::<&str>::step(Some(0), 3, true), Some(0));
    assert_eq!(TagInput::<&str>::step(Some(0), 3, false), Some(1));
    assert_eq!(TagInput::<&str>::step(Some(1), 3, false), Some(2));
    assert_eq!(TagInput::<&str>::step(Some(2), 3, false), None);
}

#[test]
fn commit_adds_a_trimmed_new_tag_and_clears_the_text() {
    let mut tags = vec!["rust".to_owned()];
    let mut text = "  tui  ".to_owned();
    assert!(TagInput::<String>::commit(&mut tags, &mut text));
    assert_eq!(tags, ["rust", "tui"]);
    assert_eq!(text, "");
    text.push_str("rust");
    assert!(!TagInput::<String>::commit(&mut tags, &mut text));
    assert_eq!(text, "");
    text.push_str("   ");
    assert!(!TagInput::<String>::commit(&mut tags, &mut text));
    assert_eq!(tags, ["rust", "tui"]);
    assert_eq!(text, "");
}

#[test]
fn chips_and_the_field_may_end_exactly_at_the_edge() {
    assert_eq!(rows(&input("ru", 2), 16, 3)[0], " rust ×   tui ×");
    assert_eq!(input("", 0).rows(16), 3);
    assert_eq!(
        rows(&input("abcdefgh", 0), 35, 2),
        [" rust ×   tui ×   async ×  abcdefgh", ""]
    );
    assert_eq!(input("", 0).rows(35), 1);
    assert_eq!(input("", 0).rows(34), 2);
    let none: [&str; 0] = [];
    let empty = TagInput::new(&none, "", 0);
    assert_eq!(empty.rows(6), 1);
    assert_eq!(empty.rows(1), 1);
    assert_eq!(
        empty.field_area(Rect::new(0, 0, 6, 3)),
        Rect::new(0, 0, 6, 1)
    );
}
