// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::ops::Range;

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{CodeMark, CodeView, CodeViewState, RenderCx, Widget, WidgetState};

const MAIN: [&str; 3] = ["fn main() {", "    println!(\"hi\");", "}"];

fn draw(view: &impl Widget, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let dots = ".".repeat(usize::from(width));
    for y in 0..height {
        cx.put_str(0, y, &dots, Style::new().fg(Color::Rgb(200, 200, 200)));
    }
    view.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(view: &impl Widget, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(view, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn synced(lines: usize, rows: u16) -> CodeViewState {
    let text = vec![""; lines];
    let mut state = CodeViewState::new();
    CodeView::new(&text).sync(Rect::new(0, 0, 20, rows), &mut state);
    state
}

#[test]
fn numbers_each_line_and_blanks_rows_past_the_end() {
    assert_eq!(
        rows(&CodeView::new(&MAIN), 24, 4),
        [
            "1 fn main() {           ",
            "2     println!(\"hi\");   ",
            "3 }                     ",
            "                        ",
        ]
    );
    let (grid, _) = draw(&CodeView::new(&MAIN), 24, 4);
    let number = style_at(&grid, 0, 0);
    assert_eq!(number.fg, token(Token::TextPrimary));
    assert!(number.attrs.dim());
    assert_eq!(style_at(&grid, 2, 0).fg, token(Token::TextPrimary));
    assert!(!style_at(&grid, 2, 0).attrs.dim());
}

#[test]
fn the_gutter_grows_with_the_last_line_number() {
    let view = CodeView::new(&MAIN).first_line_number(98);
    assert_eq!(view.gutter_width(), 4);
    assert_eq!(
        rows(&view, 12, 3),
        [" 98 fn main(", " 99     prin", "100 }       "]
    );
    assert_eq!(CodeView::new(&MAIN).line_numbers(false).gutter_width(), 0);
    assert_eq!(
        rows(
            &CodeView::new(&MAIN).line_numbers(false).scrollbar(false),
            12,
            1
        ),
        ["fn main() { "]
    );
}

#[test]
fn marks_take_a_column_before_the_numbers() {
    let marks = [
        (0, CodeMark::Added),
        (2, CodeMark::Error),
        (2, CodeMark::Info),
    ];
    let view = CodeView::new(&MAIN).marks(&marks);
    assert_eq!(view.gutter_width(), 3);
    let (grid, clusters) = draw(&view, 14, 3);
    assert_eq!(grid.row_display(0, &clusters), "▎1 fn main() {");
    assert_eq!(grid.row_display(1, &clusters), " 2     println");
    assert_eq!(grid.row_display(2, &clusters), "●3 }          ");
    assert_eq!(style_at(&grid, 0, 0).fg, token(Token::Ok));
    assert_eq!(style_at(&grid, 0, 2).fg, token(Token::Fail));
    let every = [
        (0, CodeMark::Changed),
        (1, CodeMark::Removed),
        (2, CodeMark::Warning),
    ];
    let (grid, clusters) = draw(&CodeView::new(&MAIN).marks(&every), 6, 3);
    assert_eq!(grid.row_display(1, &clusters), "▁2    ");
    assert_eq!(style_at(&grid, 0, 0).fg, token(Token::Warn));
    assert_eq!(style_at(&grid, 0, 1).fg, token(Token::Fail));
    assert_eq!(style_at(&grid, 0, 2).fg, token(Token::Warn));
    let (grid, clusters) = draw(
        &CodeView::new(&MAIN)
            .marks(&[(1, CodeMark::Breakpoint), (0, CodeMark::Info)])
            .scrollbar(false),
        6,
        2,
    );
    assert_eq!(grid.row_display(1, &clusters), "◆2    ");
    assert_eq!(style_at(&grid, 0, 1).fg, token(Token::Fail));
    assert_eq!(style_at(&grid, 0, 0).fg, token(Token::Accent));
}

#[test]
fn the_cursor_line_is_raised_and_its_number_stands_out() {
    let mut state = synced(3, 3);
    state.set_cursor(Some(1));
    let (grid, _) = draw(&CodeView::new(&MAIN).view(&state), 24, 3);
    for x in [0, 1, 6, 23] {
        assert_eq!(style_at(&grid, x, 1).bg, token(Token::Surface1), "({x},1)");
    }
    assert_ne!(style_at(&grid, 0, 0).bg, token(Token::Surface1));
    let number = style_at(&grid, 0, 1);
    assert_eq!(number.fg, token(Token::Accent));
    assert!(number.attrs.bold());
    let (grid, _) = draw(
        &CodeView::new(&MAIN)
            .view(&state)
            .state(WidgetState::Focused),
        24,
        3,
    );
    assert_eq!(style_at(&grid, 0, 1).fg, token(Token::FocusRing));
}

#[test]
fn scrolls_down_and_right() {
    let mut state = synced(3, 2);
    state.down(1);
    assert_eq!(state.top(), 1);
    assert_eq!(
        rows(&CodeView::new(&MAIN).view(&state).scrollbar(false), 16, 2),
        ["2     println!(\"", "3 }             "]
    );
    state.scroll_right(4);
    assert_eq!(state.left(), 4);
    assert_eq!(
        rows(&CodeView::new(&MAIN).view(&state).scrollbar(false), 16, 2),
        ["2 println!(\"hi\")", "3               "]
    );
    state.scroll_left(10);
    assert_eq!(state.left(), 0);
}

#[test]
fn tabs_stop_at_the_tab_width() {
    let lines = ["\tx", "ab\ty", "\t\tz"];
    assert_eq!(
        rows(&CodeView::new(&lines).line_numbers(false), 10, 3),
        ["    x     ", "ab  y     ", "        z "]
    );
    assert_eq!(
        rows(
            &CodeView::new(&lines).line_numbers(false).tab_width(3),
            10,
            3
        ),
        ["   x      ", "ab y      ", "      z   "]
    );
    assert_eq!(
        rows(
            &CodeView::new(&lines)
                .line_numbers(false)
                .tab_width(0)
                .scrollbar(false),
            10,
            1
        ),
        [" x        "]
    );
}

#[test]
fn wide_characters_cut_at_either_edge_become_blanks() {
    let lines = ["ab漢字"];
    assert_eq!(
        rows(&CodeView::new(&lines).line_numbers(false), 3, 1),
        ["ab "]
    );
    let mut state = synced(1, 1);
    state.scroll_right(3);
    assert_eq!(
        rows(
            &CodeView::new(&lines).line_numbers(false).view(&state),
            5,
            1
        ),
        [" 字  "]
    );
}

#[test]
fn the_highlight_hook_styles_byte_ranges_over_the_line() {
    let keyword = |_: usize, text: &str, paint: &mut dyn FnMut(Range<usize>, Style)| {
        if let Some(at) = text.find("fn") {
            paint(
                at..at + 2,
                Style::new()
                    .fg(Color::Rgb(9, 9, 9))
                    .with_attrs(Attrs::new().set_bold(true)),
            );
        }
        if let Some(at) = text.find('\t') {
            paint(at..at + 2, Style::new().bg(Color::Rgb(1, 1, 1)));
        }
    };
    let lines = ["fn a", "\tfn"];
    let mut state = synced(2, 2);
    state.set_cursor(Some(0));
    let view = CodeView::new(&lines).view(&state).highlight(&keyword);
    let (grid, _) = draw(&view, 10, 2);
    let painted = style_at(&grid, 2, 0);
    assert_eq!(painted.fg, Color::Rgb(9, 9, 9));
    assert_eq!(painted.bg, token(Token::Surface1));
    assert!(painted.attrs.bold());
    assert_eq!(style_at(&grid, 3, 0).fg, Color::Rgb(9, 9, 9));
    assert_eq!(style_at(&grid, 4, 0).fg, token(Token::TextPrimary));
    assert!(!style_at(&grid, 4, 0).attrs.bold());
    for x in 2..7 {
        assert_eq!(style_at(&grid, x, 1).bg, Color::Rgb(1, 1, 1), "({x},1)");
    }
    assert_eq!(style_at(&grid, 7, 1).fg, Color::Rgb(9, 9, 9));
    assert_ne!(style_at(&grid, 8, 1).bg, Color::Rgb(1, 1, 1));

    let mut right = synced(2, 2);
    right.scroll_right(5);
    let (grid, _) = draw(
        &CodeView::new(&lines).view(&right).highlight(&keyword),
        6,
        2,
    );
    assert_eq!(style_at(&grid, 2, 1).fg, Color::Rgb(9, 9, 9));
    assert_eq!(style_at(&grid, 2, 1).bg, Color::Default);
}

#[test]
fn a_scrollbar_takes_the_right_column_when_lines_overflow() {
    let lines: Vec<String> = (0..10).map(|n| format!("line {n}")).collect();
    let view = CodeView::new(&lines);
    let area = Rect::new(0, 0, 12, 4);
    assert_eq!(view.text_area(area), Rect::new(3, 0, 8, 4));
    let drawn = rows(&view, 12, 4);
    assert_eq!(drawn[0], " 1 line 0  █");
    assert_eq!(drawn[3], " 4 line 3  │");
    let plain = CodeView::new(&lines).scrollbar(false);
    assert_eq!(plain.text_area(area), Rect::new(3, 0, 9, 4));
    assert_eq!(rows(&plain, 12, 1)[0], " 1 line 0   ");
    assert_eq!(CodeView::new(&MAIN).text_area(area), Rect::new(2, 0, 10, 4));
}

#[test]
fn line_at_maps_rows_to_lines() {
    let mut state = synced(3, 2);
    state.down(1);
    let view = CodeView::new(&MAIN).view(&state);
    let area = Rect::new(4, 2, 20, 3);
    assert_eq!(view.line_at(area, 2), Some(1));
    assert_eq!(view.line_at(area, 3), Some(2));
    assert_eq!(view.line_at(area, 4), None);
    assert_eq!(view.line_at(area, 1), None);
    assert_eq!(view.line_at(area, 5), None);
}

#[test]
fn the_cursor_stays_on_screen_and_in_range() {
    let mut state = synced(20, 5);
    state.set_cursor(Some(0));
    state.down(6);
    assert_eq!((state.cursor(), state.top()), (Some(6), 2));
    state.up(4);
    assert_eq!((state.cursor(), state.top()), (Some(2), 2));
    state.up(1);
    assert_eq!((state.cursor(), state.top()), (Some(1), 1));
    state.page_down();
    assert_eq!((state.cursor(), state.top()), (Some(6), 2));
    state.down(100);
    assert_eq!((state.cursor(), state.top()), (Some(19), 15));
    state.page_up();
    assert_eq!((state.cursor(), state.top()), (Some(14), 14));
    state.set_cursor(None);
    assert_eq!(state.cursor(), None);
    state.up(100);
    assert_eq!(state.top(), 0);
}

#[test]
fn scrolling_without_a_cursor_stops_at_the_last_screen() {
    let mut state = synced(20, 5);
    state.down(3);
    assert_eq!(state.top(), 3);
    state.page_down();
    assert_eq!(state.top(), 8);
    state.down(100);
    assert_eq!(state.top(), 15);
    state.scroll_to(40);
    assert_eq!(state.top(), 15);
    state.scroll_to(4);
    assert_eq!(state.top(), 4);
    state.page_up();
    assert_eq!(state.top(), 0);
    let short = synced(3, 5);
    let mut short = short;
    short.down(2);
    assert_eq!(short.top(), 0);
}

#[test]
fn sync_keeps_a_cursor_inside_shorter_text() {
    let mut state = synced(20, 5);
    state.set_cursor(Some(18));
    assert_eq!(state.top(), 14);
    CodeView::new(&MAIN).sync(Rect::new(0, 0, 20, 2), &mut state);
    assert_eq!((state.cursor(), state.top()), (Some(2), 1));
    let empty: [&str; 0] = [];
    CodeView::new(&empty).sync(Rect::new(0, 0, 20, 2), &mut state);
    assert_eq!((state.cursor(), state.top()), (None, 0));
    assert_eq!(rows(&CodeView::new(&empty), 6, 2), ["      ", "      "]);
}

#[test]
fn line_at_counts_rows_from_the_top_of_the_area() {
    let lines = ["a"; 10];
    let view = CodeView::new(&lines);
    let area = Rect::new(0, 4, 20, 3);
    assert_eq!(view.line_at(area, 6), Some(2));
    assert_eq!(view.line_at(area, 7), None);
    assert_eq!(view.line_at(area, 8), None);
}

#[test]
fn no_scrollbar_when_the_lines_exactly_fit() {
    let area = Rect::new(0, 0, 12, 3);
    assert_eq!(CodeView::new(&MAIN).text_area(area), Rect::new(2, 0, 10, 3));
    assert_eq!(rows(&CodeView::new(&MAIN), 12, 3)[0], "1 fn main() ");
}

#[test]
fn a_highlight_mid_line_covers_only_its_range() {
    let paint_fn = |_: usize, text: &str, paint: &mut dyn FnMut(Range<usize>, Style)| {
        if let Some(at) = text.find("fn") {
            paint(at..at + 2, Style::new().fg(Color::Rgb(9, 9, 9)));
        }
    };
    let lines = ["pub fn go"];
    let view = CodeView::new(&lines)
        .line_numbers(false)
        .highlight(&paint_fn);
    let (grid, _) = draw(&view, 12, 1);
    assert_eq!(style_at(&grid, 3, 0).fg, token(Token::TextPrimary));
    assert_eq!(style_at(&grid, 4, 0).fg, Color::Rgb(9, 9, 9));
    assert_eq!(style_at(&grid, 5, 0).fg, Color::Rgb(9, 9, 9));
    assert_eq!(style_at(&grid, 6, 0).fg, token(Token::TextPrimary));
    assert_eq!(style_at(&grid, 8, 0).fg, token(Token::TextPrimary));
}

#[test]
fn rows_past_the_end_are_blanked_inside_the_area_only() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(12, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    for y in 0..3 {
        cx.put_str(0, y, "............", Style::new());
    }
    let lines = ["ab"];
    CodeView::new(&lines)
        .line_numbers(false)
        .render(Rect::new(2, 0, 8, 3), &mut cx);
    let shown: Vec<String> = (0..3).map(|y| grid.row_display(y, &clusters)).collect();
    assert_eq!(shown, ["..ab      ..", "..        ..", "..        .."]);
}
