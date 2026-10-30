// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    NotificationCenter, NotificationId, NotificationLog, RenderCx, ToastKind, Widget, WidgetState,
};

fn log() -> NotificationLog {
    let mut log = NotificationLog::new(8);
    log.push(ToastKind::Success, "Build done", "12:04");
    log.push(ToastKind::Warning, "Disk almost full", "12:09");
    log.push(ToastKind::Error, "Deploy failed", "12:15");
    log
}

fn draw(center: &NotificationCenter<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    center.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(center: &NotificationCenter<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(center, width, height);
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

fn nth_id(log: &NotificationLog, n: usize) -> NotificationId {
    log.iter()
        .nth(n)
        .map_or(NotificationId(u32::MAX), |item| item.id())
}

fn center(log: &NotificationLog) -> NotificationCenter<'_> {
    NotificationCenter::new(log)
}

#[test]
fn entries_are_listed_newest_first_under_a_header_with_the_unread_count() {
    let log = log();
    assert_eq!(
        rows(&center(&log), 24, 5),
        [
            " Notifications 3 unread",
            " ● 12:15 Deploy faile ×",
            " ● 12:09 Disk almost  ×",
            " ● 12:04 Build done   ×",
            "",
        ]
    );
}

#[test]
fn read_entries_get_an_open_mark_and_the_header_says_so_when_all_are_read() {
    let mut log = log();
    let id = nth_id(&log, 1);
    assert!(log.mark_read(id));
    assert_eq!(
        rows(&center(&log), 24, 4),
        [
            " Notifications 2 unread",
            " ● 12:15 Deploy faile ×",
            " ○ 12:09 Disk almost  ×",
            " ● 12:04 Build done   ×",
        ]
    );
    log.mark_all_read();
    let shown = rows(&center(&log), 24, 4);
    assert_eq!(shown[0], " Notifications All read");
    assert_eq!(shown[3], " ○ 12:04 Build done   ×");
}

#[test]
fn the_unread_count_prints_every_digit() {
    let mut log = NotificationLog::new(200);
    for _ in 0..105 {
        log.push(ToastKind::Info, "x", "");
    }
    assert_eq!(
        rows(&center(&log), 30, 1)[0],
        " Notifications     105 unread"
    );
    log.mark_read(nth_id(&log, 0));
    assert_eq!(
        rows(&center(&log), 30, 1)[0],
        " Notifications     104 unread"
    );
    for _ in 0..95 {
        log.dismiss(nth_id(&log, 0));
    }
    assert_eq!(
        rows(&center(&log), 30, 1)[0],
        " Notifications      10 unread"
    );
}

#[test]
fn an_empty_log_says_so() {
    let log = NotificationLog::new(4);
    assert_eq!(
        rows(&center(&log), 24, 3),
        [" Notifications All read", " No notifications", ""]
    );
}

#[test]
fn entries_without_a_stamp_start_the_message_after_the_mark() {
    let mut log = NotificationLog::new(4);
    log.push(ToastKind::Info, "Saved", "");
    assert_eq!(rows(&center(&log), 20, 2)[1], " ● Saved          ×");
}

#[test]
fn long_text_and_stamps_are_clipped_before_the_dismiss_mark() {
    let mut log = NotificationLog::new(4);
    log.push(ToastKind::Info, "A very long message", "12:04:31");
    assert_eq!(rows(&center(&log), 16, 2)[1], " ● 12:04:31 A ×");
    assert_eq!(rows(&center(&log), 12, 2)[1], " ● 12:04:3×");
    assert_eq!(rows(&center(&log), 8, 2)[1], " ● 12:×");
}

#[test]
fn a_narrow_row_has_no_dismiss_mark() {
    let log = log();
    assert_eq!(rows(&center(&log), 5, 2), [" Not", " ● 12"]);
    assert_eq!(center(&log).dismiss_at(Rect::new(0, 0, 5, 4), 3, 1), None);
    assert_eq!(rows(&center(&log), 6, 2)[1], " ● 1×");
}

#[test]
fn the_header_drops_the_count_when_it_does_not_fit() {
    let log = log();
    assert_eq!(rows(&center(&log), 24, 1)[0], " Notifications 3 unread");
    assert_eq!(rows(&center(&log), 23, 1)[0], " Notifications");
    assert_eq!(rows(&center(&log), 9, 1)[0], " Notific");
}

#[test]
fn the_marks_take_the_kind_color_and_the_text_follows_the_read_state() {
    let mut log = log();
    let id = nth_id(&log, 2);
    log.mark_read(id);
    let (grid, _) = draw(&center(&log), 24, 4);
    let surface = color(Token::Surface0);
    let text = color(Token::TextPrimary);
    assert_eq!(style_at(&grid, 1, 1).fg, color(Token::Fail));
    assert_eq!(style_at(&grid, 1, 2).fg, color(Token::Warn));
    assert_eq!(style_at(&grid, 1, 3).fg, color(Token::Ok));
    assert_eq!(style_at(&grid, 1, 1).bg, surface);
    let unread = style_at(&grid, 9, 1);
    assert_eq!(unread.fg, text);
    assert!(unread.attrs.bold());
    assert!(!unread.attrs.dim());
    let read = style_at(&grid, 9, 3);
    assert!(read.attrs.dim());
    assert!(!read.attrs.bold());
    let stamp = style_at(&grid, 3, 1);
    assert!(stamp.attrs.dim());
    assert!(!stamp.attrs.bold());
    assert_eq!(stamp.fg, text);
    assert!(style_at(&grid, 22, 1).attrs.dim());
    assert_eq!(style_at(&grid, 22, 1).bg, surface);
    assert_eq!(style_at(&grid, 23, 1).bg, surface);
    assert_eq!(style_at(&grid, 0, 4).bg, Style::default().bg);
}

#[test]
fn the_header_is_a_bold_bar_with_an_accent_count() {
    let log = log();
    let (grid, _) = draw(&center(&log), 24, 3);
    let bar = color(Token::Surface1);
    for x in [0, 1, 13, 14, 23] {
        assert_eq!(style_at(&grid, x, 0).bg, bar);
    }
    let title = style_at(&grid, 1, 0);
    assert_eq!(title.fg, color(Token::TextPrimary));
    assert!(title.attrs.bold());
    assert!(!title.attrs.dim());
    assert_eq!(style_at(&grid, 15, 0).fg, color(Token::Accent));
    assert!(style_at(&grid, 16, 0).attrs.dim());
    assert_eq!(style_at(&grid, 16, 0).fg, color(Token::TextPrimary));
    let mut log = log;
    log.mark_all_read();
    let (grid, _) = draw(&center(&log), 24, 3);
    assert!(style_at(&grid, 15, 0).attrs.dim());
}

#[test]
fn the_selected_row_is_highlighted_across_its_width() {
    let log = log();
    let (grid, _) = draw(&center(&log).selected(Some(1)), 24, 4);
    let accent = color(Token::Accent);
    let surface = color(Token::Surface0);
    for x in 0..24 {
        let style = style_at(&grid, x, 2);
        assert_eq!(style.bg, accent, "column {x}");
    }
    assert_eq!(style_at(&grid, 1, 2).fg, surface);
    assert_eq!(style_at(&grid, 9, 2).fg, surface);
    assert_eq!(style_at(&grid, 1, 1).bg, surface);
    assert_eq!(style_at(&grid, 1, 3).bg, surface);
}

fn item_row(n: usize) -> String {
    format!(" ● {:<19}×", format!("item {n}"))
}

#[test]
fn the_list_scrolls_to_keep_the_selected_row_in_view() {
    let mut log = NotificationLog::new(8);
    for n in 0..5 {
        log.push(ToastKind::Info, format!("item {n}"), "");
    }
    let shown = |selected: usize| rows(&center(&log).selected(Some(selected)), 24, 3);
    assert_eq!(shown(0)[0], " Notifications 5 unread");
    assert_eq!(shown(0)[1..], [item_row(4), item_row(3)]);
    assert_eq!(shown(1)[1..], [item_row(4), item_row(3)]);
    assert_eq!(shown(2)[1..], [item_row(3), item_row(2)]);
    assert_eq!(shown(4)[1..], [item_row(1), item_row(0)]);
    assert_eq!(shown(9)[1..], [item_row(4), item_row(3)]);
}

#[test]
fn a_selection_past_the_end_highlights_nothing() {
    let log = log();
    let (grid, _) = draw(&center(&log).selected(Some(3)), 24, 4);
    for y in 1..4 {
        assert_eq!(style_at(&grid, 5, y).bg, color(Token::Surface0));
    }
}

#[test]
fn row_at_maps_cells_to_positions_and_follows_the_scroll() {
    let mut log = NotificationLog::new(8);
    for n in 0..5 {
        log.push(ToastKind::Info, format!("item {n}"), "");
    }
    let area = Rect::new(2, 3, 20, 3);
    let top = center(&log);
    assert_eq!(top.row_at(area, 2, 3), None);
    assert_eq!(top.row_at(area, 5, 4), Some(0));
    assert_eq!(top.row_at(area, 21, 5), Some(1));
    assert_eq!(top.row_at(area, 22, 5), None);
    assert_eq!(top.row_at(area, 1, 5), None);
    assert_eq!(top.row_at(area, 5, 6), None);
    assert_eq!(top.row_at(area, 5, 2), None);
    let low = center(&log).selected(Some(4));
    assert_eq!(low.row_at(area, 5, 4), Some(3));
    assert_eq!(low.row_at(area, 5, 5), Some(4));
    let short = NotificationLog::new(2);
    assert_eq!(center(&short).row_at(area, 5, 4), None);
}

#[test]
fn dismiss_at_finds_only_the_cross() {
    let log = log();
    let area = Rect::new(2, 1, 24, 4);
    let center = center(&log);
    assert_eq!(center.dismiss_at(area, 24, 2), Some(0));
    assert_eq!(center.dismiss_at(area, 24, 4), Some(2));
    assert_eq!(center.dismiss_at(area, 23, 2), None);
    assert_eq!(center.dismiss_at(area, 25, 2), None);
    assert_eq!(center.dismiss_at(area, 24, 1), None);
    assert_eq!(center.dismiss_at(area, 24, 5), None);
}

#[test]
fn list_area_sits_below_the_header() {
    let log = log();
    let center = center(&log);
    assert_eq!(
        center.list_area(Rect::new(3, 2, 20, 6)),
        Rect::new(3, 3, 20, 5)
    );
    assert_eq!(
        center.list_area(Rect::new(3, 2, 20, 1)),
        Rect::new(3, 3, 20, 0)
    );
    assert_eq!(
        center.list_area(Rect::new(3, 2, 20, 0)),
        Rect::new(3, 2, 20, 0)
    );
}

#[test]
fn a_header_only_area_draws_just_the_header() {
    let log = log();
    assert_eq!(rows(&center(&log), 24, 1), [" Notifications 3 unread"]);
}

#[test]
fn a_disabled_center_dims_everything() {
    let log = log();
    let (grid, _) = draw(&center(&log).state(WidgetState::Disabled), 24, 3);
    assert!(style_at(&grid, 1, 0).attrs.dim());
    assert!(style_at(&grid, 9, 1).attrs.dim());
    assert!(style_at(&grid, 1, 1).attrs.dim());
    let empty = NotificationLog::new(2);
    let (grid, _) = draw(&center(&empty).state(WidgetState::Disabled), 24, 3);
    assert!(style_at(&grid, 5, 1).attrs.dim());
}

#[test]
fn an_empty_area_draws_nothing() {
    let log = log();
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 4);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    center(&log).render(Rect::new(0, 0, 0, 4), &mut cx);
    center(&log).render(Rect::new(0, 0, 10, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    assert_eq!(grid.row_display(1, &clusters).trim_end(), "");
}

#[test]
fn measure_is_the_header_plus_one_row_per_entry() {
    let log = log();
    let center = center(&log);
    assert_eq!(center.measure(Size::new(30, 20)), Size::new(30, 4));
    assert_eq!(center.measure(Size::new(30, 2)), Size::new(30, 2));
    assert_eq!(center.measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(center.measure(Size::new(5, 0)), Size::new(0, 0));
    let empty = NotificationLog::new(2);
    assert_eq!(
        self::center(&empty).measure(Size::new(30, 20)),
        Size::new(30, 2)
    );
}

#[test]
fn the_log_keeps_the_newest_entries_and_numbers_them_in_order() {
    let mut log = NotificationLog::new(2);
    let a = log.push(ToastKind::Info, "a", "");
    let b = log.push(ToastKind::Info, "b", "");
    let c = log.push(ToastKind::Error, "c", "09:00");
    assert_eq!((a.0, b.0, c.0), (0, 1, 2));
    assert_eq!(log.len(), 2);
    assert!(log.get(a).is_none());
    let texts: Vec<&str> = log.iter().map(|item| item.text()).collect();
    assert_eq!(texts, ["c", "b"]);
    let newest = log
        .get(c)
        .map(|item| (item.kind(), item.stamp(), item.read()));
    assert_eq!(newest, Some((ToastKind::Error, "09:00", false)));
    assert_eq!(log.get(b).map(|item| item.id()), Some(b));
}

#[test]
fn a_log_holds_at_least_one_entry() {
    let mut log = NotificationLog::new(0);
    log.push(ToastKind::Info, "a", "");
    log.push(ToastKind::Info, "b", "");
    assert_eq!(log.len(), 1);
    assert_eq!(log.iter().next().map(|item| item.text()), Some("b"));
}

#[test]
fn dismiss_clear_and_read_marks_report_what_they_found() {
    let mut log = NotificationLog::new(4);
    let a = log.push(ToastKind::Info, "a", "");
    let b = log.push(ToastKind::Info, "b", "");
    assert_eq!(log.unread(), 2);
    assert!(log.mark_read(a));
    assert_eq!(log.unread(), 1);
    assert_eq!(log.get(b).map(|item| item.read()), Some(false));
    assert!(!log.mark_read(NotificationId(9)));
    assert!(log.dismiss(a));
    assert!(!log.dismiss(a));
    assert_eq!(log.len(), 1);
    log.clear();
    assert!(log.is_empty());
    assert_eq!(log.unread(), 0);
}
