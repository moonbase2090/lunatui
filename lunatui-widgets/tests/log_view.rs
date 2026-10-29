// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{LogBuffer, LogEntry, LogLevel, LogView, RenderCx, Widget, WidgetState};

fn draw(view: &LogView<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    view.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(view: &LogView<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(view, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn padded<const N: usize>(width: usize, rows: [&str; N]) -> Vec<String> {
    rows.map(|row| format!("{row:<width$}")).to_vec()
}

fn sample() -> LogBuffer {
    let mut buffer = LogBuffer::new(100);
    buffer.push(LogEntry::new(LogLevel::Info, "net", "connected"));
    buffer.push(LogEntry::new(LogLevel::Debug, "db", "pool ready"));
    buffer.push(LogEntry::new(LogLevel::Warn, "net", "slow reply"));
    buffer.push(LogEntry::new(LogLevel::Error, "db", "query failed"));
    buffer
}

#[test]
fn rows_show_the_level_and_message_oldest_first() {
    let buffer = sample();
    assert_eq!(
        rows(&LogView::new(&buffer), 22, 4),
        [
            " INFO connected      ",
            "DEBUG pool ready     ",
            " WARN slow reply     ",
            "ERROR query failed   ",
        ]
        .map(|row| format!("{row:<22}"))
    );
}

#[test]
fn a_target_column_is_clipped_and_padded() {
    let buffer = sample();
    let view = LogView::new(&buffer).target_width(4);
    assert_eq!(
        rows(&view, 26, 2),
        padded(26, [" WARN net  slow reply", "ERROR db   query failed"])
    );
    let tight = LogView::new(&buffer).target_width(1);
    assert_eq!(rows(&tight, 20, 1)[0], "ERROR d query failed");
}

#[test]
fn a_stamp_comes_first_and_a_blank_one_is_skipped() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Info, "app", "start").stamp("12:04:31"));
    buffer.push(LogEntry::new(LogLevel::Info, "app", "ready"));
    assert_eq!(
        rows(&LogView::new(&buffer), 24, 2),
        ["12:04:31  INFO start    ", " INFO ready            "].map(|row| format!("{row:<24}"))
    );
}

#[test]
fn a_short_log_sits_at_the_bottom() {
    let buffer = sample();
    let view = LogView::new(&buffer).min_level(LogLevel::Warn);
    assert_eq!(
        rows(&view, 20, 3),
        [
            "                    ",
            " WARN slow reply    ",
            "ERROR query failed  "
        ]
    );
}

#[test]
fn following_shows_the_newest_rows() {
    let buffer = sample();
    assert_eq!(
        rows(&LogView::new(&buffer), 20, 2),
        [" WARN slow reply    ", "ERROR query failed  "]
    );
}

#[test]
fn scrolling_leaves_entries_out_below_the_view() {
    let buffer = sample();
    let view = LogView::new(&buffer).follow(false).scroll(1);
    assert_eq!(
        rows(&view, 20, 2),
        padded(20, ["DEBUG pool ready   ", " WARN slow reply    "])
    );
    let view = LogView::new(&buffer).follow(false).scroll(2);
    assert_eq!(
        rows(&view, 20, 2),
        padded(20, [" INFO connected    ", "DEBUG pool ready   "])
    );
}

#[test]
fn scrolling_stops_at_the_oldest_entry_and_zero_follows() {
    let buffer = sample();
    let past = LogView::new(&buffer).follow(false).scroll(99);
    let top = LogView::new(&buffer).follow(false).scroll(2);
    assert_eq!(rows(&past, 20, 2), rows(&top, 20, 2));
    let zero = LogView::new(&buffer).follow(false).scroll(0);
    assert_eq!(rows(&zero, 20, 2), rows(&LogView::new(&buffer), 20, 2));
}

#[test]
fn following_ignores_the_scroll_offset() {
    let buffer = sample();
    let view = LogView::new(&buffer).scroll(3);
    assert_eq!(rows(&view, 20, 2), rows(&LogView::new(&buffer), 20, 2));
}

#[test]
fn a_view_taller_than_the_log_still_scrolls_to_nothing() {
    let buffer = sample();
    let view = LogView::new(&buffer).follow(false).scroll(1);
    assert_eq!(view.max_scroll(10), 0);
    assert_eq!(rows(&view, 20, 5)[1..].len(), 4);
    assert_eq!(rows(&view, 20, 5)[1], " INFO connected     ");
}

#[test]
fn the_level_filter_hides_quieter_entries() {
    let buffer = sample();
    let view = LogView::new(&buffer).min_level(LogLevel::Info);
    assert_eq!(view.matching(), 3);
    assert_eq!(
        rows(&view, 20, 3),
        padded(
            20,
            [
                " INFO connected    ",
                " WARN slow reply    ",
                "ERROR query failed  "
            ]
        )
    );
    assert_eq!(
        LogView::new(&buffer).min_level(LogLevel::Error).matching(),
        1
    );
}

#[test]
fn the_target_filter_matches_a_prefix() {
    let buffer = sample();
    let view = LogView::new(&buffer).target("db");
    assert_eq!(view.matching(), 2);
    assert_eq!(
        rows(&view, 20, 2),
        padded(20, ["DEBUG pool ready   ", "ERROR query failed  "])
    );
    assert_eq!(LogView::new(&buffer).target("b").matching(), 0);
}

#[test]
fn the_query_keeps_matching_messages_and_targets() {
    let buffer = sample();
    assert_eq!(LogView::new(&buffer).query("QUERY").matching(), 0);
    assert_eq!(LogView::new(&buffer).query("query").matching(), 1);
    assert_eq!(LogView::new(&buffer).query("net").matching(), 2);
    assert_eq!(LogView::new(&buffer).query("").matching(), 4);
}

#[test]
fn a_query_with_a_capital_matches_case_exactly() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Info, "app", "Opened file"));
    buffer.push(LogEntry::new(LogLevel::Info, "app", "opened file"));
    assert_eq!(LogView::new(&buffer).query("opened").matching(), 2);
    assert_eq!(LogView::new(&buffer).query("Opened").matching(), 1);
}

#[test]
fn matches_in_the_message_are_highlighted() {
    let buffer = sample();
    let view = LogView::new(&buffer).query("ow");
    let (grid, _) = draw(&view, 20, 1);
    let hit = style_at(&grid, 8, 0);
    assert_eq!(hit.bg, color(Token::Accent));
    assert_eq!(hit.fg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 9, 0), hit);
    assert_eq!(style_at(&grid, 7, 0).bg, color(Token::Surface0));
    assert_eq!(style_at(&grid, 10, 0).bg, color(Token::Surface0));
}

#[test]
fn every_match_in_a_message_is_highlighted() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Info, "a", "abcabc"));
    let (grid, _) = draw(&LogView::new(&buffer).query("bc"), 16, 1);
    let accent = color(Token::Accent);
    let lit: Vec<bool> = (6..12)
        .map(|x| style_at(&grid, x, 0).bg == accent)
        .collect();
    assert_eq!(lit, [false, true, true, false, true, true]);
}

#[test]
fn only_the_first_line_of_a_message_is_drawn() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Error, "app", "boom\nstack frame"));
    assert_eq!(rows(&LogView::new(&buffer), 14, 1)[0], "ERROR boom    ");
}

#[test]
fn long_messages_are_clipped_to_the_width() {
    let buffer = sample();
    assert_eq!(rows(&LogView::new(&buffer), 14, 1)[0], "ERROR query fa");
    assert_eq!(rows(&LogView::new(&buffer), 4, 1)[0], &"ERROR"[..4]);
}

#[test]
fn wide_characters_are_not_split() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Info, "app", "漢字漢字"));
    assert_eq!(rows(&LogView::new(&buffer), 12, 1)[0], " INFO 漢字漢");
}

#[test]
fn each_level_has_its_own_label_color() {
    let buffer = sample();
    let (grid, _) = draw(&LogView::new(&buffer), 20, 4);
    let info = style_at(&grid, 1, 0);
    assert_eq!(info.fg, color(Token::Ok));
    let debug = style_at(&grid, 0, 1);
    assert_eq!(debug.fg, color(Token::Accent));
    let warn = style_at(&grid, 1, 2);
    assert_eq!(warn.fg, color(Token::Warn));
    assert!(warn.attrs.bold());
    let error = style_at(&grid, 0, 3);
    assert_eq!(error.fg, color(Token::Fail));
    assert!(error.attrs.bold());
    assert!(!info.attrs.bold());
}

#[test]
fn trace_is_dim_and_messages_use_the_text_color() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Trace, "app", "tick"));
    let (grid, _) = draw(&LogView::new(&buffer), 14, 1);
    assert!(style_at(&grid, 0, 0).attrs.dim());
    let message = style_at(&grid, 6, 0);
    assert_eq!(message.fg, color(Token::TextPrimary));
    assert!(!message.attrs.dim());
}

#[test]
fn stamps_and_targets_are_dim() {
    let mut buffer = LogBuffer::new(10);
    buffer.push(LogEntry::new(LogLevel::Info, "app", "up").stamp("09:00"));
    let (grid, _) = draw(&LogView::new(&buffer).target_width(3), 24, 1);
    assert!(style_at(&grid, 0, 0).attrs.dim());
    assert!(style_at(&grid, 12, 0).attrs.dim());
    assert!(!style_at(&grid, 16, 0).attrs.dim());
}

#[test]
fn the_background_fills_the_area() {
    let buffer = sample();
    let (grid, _) = draw(&LogView::new(&buffer).min_level(LogLevel::Error), 10, 3);
    for y in 0..3 {
        for x in 0..10 {
            assert_eq!(style_at(&grid, x, y).bg, color(Token::Surface0), "{x},{y}");
        }
    }
}

#[test]
fn disabled_dims_every_entry() {
    let buffer = sample();
    let view = LogView::new(&buffer).state(WidgetState::Disabled);
    let (grid, _) = draw(&view, 20, 4);
    for y in 0..4 {
        assert!(style_at(&grid, 1, y).attrs.dim(), "label {y}");
        assert!(style_at(&grid, 8, y).attrs.dim(), "message {y}");
    }
}

#[test]
fn the_buffer_drops_the_oldest_entry_when_full() {
    let mut buffer = LogBuffer::new(2);
    for text in ["a", "b", "c"] {
        buffer.push(LogEntry::new(LogLevel::Info, "t", text));
    }
    let kept: Vec<&str> = buffer.iter().map(|entry| entry.message.as_str()).collect();
    assert_eq!(kept, ["b", "c"]);
    assert_eq!((buffer.len(), buffer.dropped()), (2, 1));
    assert!(!buffer.is_empty());
    buffer.push(LogEntry::new(LogLevel::Info, "t", "d"));
    buffer.push(LogEntry::new(LogLevel::Info, "t", "e"));
    assert_eq!((buffer.len(), buffer.dropped()), (2, 3));
}

#[test]
fn a_zero_capacity_buffer_keeps_one_entry() {
    let mut buffer = LogBuffer::new(0);
    buffer.push(LogEntry::new(LogLevel::Info, "t", "a"));
    buffer.push(LogEntry::new(LogLevel::Info, "t", "b"));
    assert_eq!(buffer.len(), 1);
    assert_eq!(buffer.dropped(), 1);
}

#[test]
fn clearing_empties_the_buffer_but_keeps_the_dropped_count() {
    let mut buffer = LogBuffer::new(1);
    buffer.push(LogEntry::new(LogLevel::Info, "t", "a"));
    buffer.push(LogEntry::new(LogLevel::Info, "t", "b"));
    buffer.clear();
    assert!(buffer.is_empty());
    assert_eq!(buffer.dropped(), 1);
}

#[test]
fn counts_group_entries_by_level() {
    let mut buffer = sample();
    buffer.push(LogEntry::new(LogLevel::Warn, "net", "again"));
    buffer.push(LogEntry::new(LogLevel::Trace, "net", "poll"));
    assert_eq!(buffer.counts(), [1, 1, 1, 2, 1]);
}

#[test]
fn levels_order_from_trace_to_error() {
    assert!(LogLevel::Trace < LogLevel::Debug);
    assert!(LogLevel::Debug < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Error);
}

#[test]
fn max_scroll_counts_entries_beyond_the_rows() {
    let buffer = sample();
    let view = LogView::new(&buffer);
    assert_eq!(view.max_scroll(1), 3);
    assert_eq!(view.max_scroll(4), 0);
    assert_eq!(view.min_level(LogLevel::Warn).max_scroll(1), 1);
}

#[test]
fn measure_is_one_row_per_matching_entry_up_to_the_space() {
    let buffer = sample();
    let view = LogView::new(&buffer);
    assert_eq!(view.measure(Size::new(30, 10)), Size::new(30, 4));
    assert_eq!(view.measure(Size::new(30, 2)), Size::new(30, 2));
}

#[test]
fn an_empty_area_draws_nothing() {
    let buffer = sample();
    let (grid, clusters) = draw(&LogView::new(&buffer), 6, 2);
    let theme = Theme::builtin(Variant::Dark);
    let mut again = Grid::new(6, 2);
    let mut table = ClusterTable::new();
    let mut cx = RenderCx::new(&mut again, &mut table, &theme);
    LogView::new(&buffer).render(Rect::new(0, 0, 0, 2), &mut cx);
    LogView::new(&buffer).render(Rect::new(0, 0, 6, 0), &mut cx);
    assert_eq!(again.row_display(0, &table), "      ");
    assert_eq!(grid.row_display(0, &clusters).len(), 6);
}

#[test]
fn a_scrolled_view_does_not_draw_below_its_area() {
    let buffer = sample();
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 4);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    LogView::new(&buffer)
        .follow(false)
        .scroll(2)
        .render(Rect::new(0, 0, 20, 2), &mut cx);
    assert_eq!(grid.row_display(1, &clusters), "DEBUG pool ready    ");
    assert_eq!(grid.row_display(2, &clusters), " ".repeat(20));
    assert_eq!(grid.row_display(3, &clusters), " ".repeat(20));
}
