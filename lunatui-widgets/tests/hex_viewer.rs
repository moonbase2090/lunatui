// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{HexViewer, HexViewerState, RenderCx, Widget, WidgetState};

const SAMPLE: &[u8] = b"Hello, lunatui!\n\x00\x7f\x80\xff";

fn draw(viewer: &HexViewer<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    viewer.render(area, &mut cx);
    (grid, clusters)
}

fn rows(viewer: &HexViewer<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(viewer, Rect::new(0, 0, width, height), width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn glyph_at(grid: &Grid, clusters: &ClusterTable, x: u16, y: u16) -> char {
    grid.row_display(y, clusters)
        .chars()
        .nth(usize::from(x))
        .unwrap_or(' ')
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn fg(token: Token) -> Style {
    Style::new().fg(color(token))
}

fn dim(style: Style) -> Style {
    style.with_attrs(Attrs::new().set_dim(true))
}

fn cursor(ring: Token) -> Style {
    Style::new()
        .fg(color(Token::Surface0))
        .bg(color(ring))
        .with_attrs(Attrs::new().set_bold(true))
}

fn selected(style: Style) -> Style {
    style.bg(color(Token::Surface1))
}

fn counting(len: usize) -> Vec<u8> {
    (0..len)
        .map(|n| u8::try_from(n % 256).unwrap_or(0))
        .collect()
}

#[test]
fn rows_show_the_offset_the_hex_bytes_and_the_ascii() {
    assert_eq!(
        rows(&HexViewer::new(SAMPLE), 80, 2),
        [
            "00000000  48 65 6c 6c 6f 2c 20 6c  75 6e 61 74 75 69 21 0a  Hello, lunatui!.",
            "00000010  00 7f 80 ff                                       ....",
        ]
    );
}

#[test]
fn the_area_offset_moves_every_column() {
    let (grid, clusters) = draw(&HexViewer::new(SAMPLE), Rect::new(3, 1, 77, 2), 80, 3);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    assert!(
        grid.row_display(1, &clusters)
            .starts_with("   00000000  48 65 6c")
    );
    assert_eq!(glyph_at(&grid, &clusters, 63, 1), 'H');
    assert_eq!(glyph_at(&grid, &clusters, 63, 2), '.');
}

#[test]
fn a_narrow_area_cuts_the_row_at_its_edge() {
    assert_eq!(
        rows(&HexViewer::new(SAMPLE), 30, 2),
        ["00000000  48 65 6c 6c 6f 2c 20", "00000010  00 7f 80 ff"]
    );
}

#[test]
fn byte_kinds_take_their_theme_colors() {
    let (grid, _) = draw(&HexViewer::new(SAMPLE), Rect::new(0, 0, 80, 2), 80, 2);
    let text = fg(Token::TextPrimary);
    assert_eq!(style_at(&grid, 0, 0), dim(text), "offset");
    assert_eq!(style_at(&grid, 10, 0), text, "printable hex");
    assert_eq!(style_at(&grid, 60, 0), text, "printable ascii");
    assert_eq!(style_at(&grid, 28, 0), text, "space");
    assert_eq!(style_at(&grid, 66, 0), text, "space ascii");
    assert_eq!(style_at(&grid, 57, 0), fg(Token::Warn), "newline");
    assert_eq!(style_at(&grid, 75, 0), fg(Token::Warn), "newline ascii");
    assert_eq!(style_at(&grid, 10, 1), dim(text), "zero");
    assert_eq!(style_at(&grid, 13, 1), fg(Token::Warn), "control");
    assert_eq!(style_at(&grid, 61, 1), fg(Token::Warn), "control ascii");
    assert_eq!(style_at(&grid, 16, 1), fg(Token::Accent), "0x80");
    assert_eq!(style_at(&grid, 20, 1), fg(Token::Accent), "0xff");
    assert_eq!(style_at(&grid, 12, 0), Style::new(), "space between bytes");
}

#[test]
fn the_cursor_marks_its_byte_in_both_columns() {
    let viewer = HexViewer::new(SAMPLE).cursor(Some(1));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    for x in [13, 14, 61] {
        assert_eq!(style_at(&grid, x, 0), cursor(Token::Accent), "x={x}");
    }
    assert_eq!(style_at(&grid, 10, 0), fg(Token::TextPrimary));
    assert_eq!(style_at(&grid, 15, 0), Style::new());

    let focused = viewer.state(WidgetState::Focused);
    let (grid, _) = draw(&focused, Rect::new(0, 0, 80, 2), 80, 2);
    assert_eq!(style_at(&grid, 13, 0), cursor(Token::FocusRing));
    assert_eq!(style_at(&grid, 61, 0), cursor(Token::FocusRing));
}

#[test]
fn a_selection_shades_its_bytes_and_the_spaces_between_them() {
    let viewer = HexViewer::new(SAMPLE).selection(Some(2..5));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    let text = fg(Token::TextPrimary);
    assert_eq!(style_at(&grid, 13, 0), text, "byte 1 is outside");
    assert_eq!(style_at(&grid, 15, 0), Style::new(), "no shade before");
    assert_eq!(style_at(&grid, 16, 0), selected(text));
    assert_eq!(style_at(&grid, 18, 0), selected(Style::new()));
    assert_eq!(style_at(&grid, 21, 0), selected(Style::new()));
    assert_eq!(style_at(&grid, 22, 0), selected(text));
    assert_eq!(style_at(&grid, 24, 0), Style::new(), "no shade after");
    assert_eq!(style_at(&grid, 25, 0), text);
    for x in 62..65 {
        assert_eq!(style_at(&grid, x, 0), selected(text), "ascii x={x}");
    }
    assert_eq!(style_at(&grid, 65, 0), text);
}

#[test]
fn a_selection_across_a_group_shades_the_group_gap() {
    let viewer = HexViewer::new(SAMPLE).selection(Some(6..10));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    for x in [30, 33, 34] {
        assert_eq!(style_at(&grid, x, 0), selected(Style::new()), "x={x}");
    }
    assert_eq!(style_at(&grid, 35, 0), selected(fg(Token::TextPrimary)));
    assert_eq!(style_at(&grid, 40, 0), Style::new());
}

#[test]
fn a_selection_does_not_shade_past_the_end_of_a_row() {
    let viewer = HexViewer::new(SAMPLE).selection(Some(14..18));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    assert_eq!(style_at(&grid, 55, 0), selected(Style::new()));
    assert_eq!(style_at(&grid, 58, 0), Style::new());
    assert_eq!(style_at(&grid, 59, 0), Style::new());
    assert_eq!(
        style_at(&grid, 10, 1),
        selected(dim(fg(Token::TextPrimary)))
    );
    assert_eq!(style_at(&grid, 12, 1), selected(Style::new()));
    assert_eq!(style_at(&grid, 15, 1), Style::new());
}

#[test]
fn the_cursor_wins_over_the_selection() {
    let viewer = HexViewer::new(SAMPLE).selection(Some(2..5)).cursor(Some(3));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    assert_eq!(style_at(&grid, 19, 0), cursor(Token::Accent));
    assert_eq!(style_at(&grid, 63, 0), cursor(Token::Accent));
    assert_eq!(style_at(&grid, 16, 0), selected(fg(Token::TextPrimary)));
}

#[test]
fn a_disabled_viewer_dims_every_byte_and_hides_the_marks() {
    let viewer = HexViewer::new(SAMPLE)
        .cursor(Some(1))
        .selection(Some(2..5))
        .state(WidgetState::Disabled);
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    let dimmed = dim(fg(Token::TextPrimary));
    for (x, y) in [(13, 0), (16, 0), (28, 0), (13, 1), (16, 1), (61, 0)] {
        assert_eq!(style_at(&grid, x, y), dimmed, "({x},{y})");
    }
    assert_eq!(style_at(&grid, 18, 0), Style::new());
}

#[test]
fn an_explicit_style_replaces_the_byte_colors_but_not_the_cursor() {
    let plain = Style::new().fg(Color::Rgb(1, 2, 3));
    let viewer = HexViewer::new(SAMPLE).style(plain).cursor(Some(0));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 2), 80, 2);
    assert_eq!(style_at(&grid, 0, 0), plain);
    assert_eq!(style_at(&grid, 13, 0), plain);
    assert_eq!(style_at(&grid, 13, 1), plain);
    assert_eq!(style_at(&grid, 10, 0), cursor(Token::Accent));
}

#[test]
fn the_header_labels_each_column() {
    let viewer = HexViewer::new(SAMPLE).header(true);
    let shown = rows(&viewer, 80, 3);
    assert_eq!(
        shown[0],
        "          00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  0123456789abcdef"
    );
    assert!(shown[1].starts_with("00000000  48 65"));
    assert!(shown[2].starts_with("00000010  00 7f"));
    let (grid, _) = draw(&viewer, Rect::new(0, 0, 80, 3), 80, 3);
    assert_eq!(style_at(&grid, 10, 0), dim(fg(Token::TextPrimary)));
    assert_eq!(style_at(&grid, 75, 0), dim(fg(Token::TextPrimary)));
}

#[test]
fn header_labels_past_sixteen_columns_count_on() {
    let viewer = HexViewer::new(SAMPLE).bytes_per_row(18).group(0);
    let header =
        "          00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11  0123456789abcdef01";
    assert_eq!(rows(&viewer.header(true), 90, 3)[0], header);
}

#[test]
fn an_area_with_no_rows_draws_nothing() {
    let viewer = HexViewer::new(SAMPLE).header(true);
    let (grid, clusters) = draw(&viewer, Rect::new(0, 0, 80, 0), 80, 1);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
}

#[test]
fn a_one_row_area_with_a_header_shows_only_the_header() {
    let viewer = HexViewer::new(SAMPLE).header(true);
    assert_eq!(
        rows(&viewer, 80, 1),
        ["          00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  0123456789abcdef"]
    );
}

#[test]
fn the_base_offset_numbers_the_rows_and_widens_the_column() {
    let data = counting(32);
    assert_eq!(
        rows(&HexViewer::new(&data).base_offset(0x10), 80, 2)[0],
        "00000010  00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  ................"
    );
    let shown = rows(&HexViewer::new(&data).base_offset(0xffff_fff0), 80, 2);
    assert!(shown[0].starts_with("0fffffff0  00 01 02"), "{}", shown[0]);
    assert!(shown[1].starts_with("100000000  10 11 12"), "{}", shown[1]);
    assert_eq!(&shown[1][61..], "................");
}

#[test]
fn the_largest_offset_fits_in_sixteen_digits() {
    let data = counting(2);
    let shown = rows(&HexViewer::new(&data).base_offset(u64::MAX - 1), 80, 1);
    assert!(
        shown[0].starts_with("fffffffffffffffe  00 01"),
        "{}",
        shown[0]
    );
}

#[test]
fn rows_and_groups_can_be_resized() {
    let viewer = HexViewer::new(SAMPLE).bytes_per_row(8).group(4);
    assert_eq!(
        rows(&viewer, 80, 3),
        [
            "00000000  48 65 6c 6c  6f 2c 20 6c  Hello, l",
            "00000008  75 6e 61 74  75 69 21 0a  unatui!.",
            "00000010  00 7f 80 ff               ....",
        ]
    );
    assert_eq!(viewer.row_width(), 44);
    assert_eq!(viewer.row_count(), 3);
    let ungrouped = HexViewer::new(SAMPLE)
        .bytes_per_row(4)
        .group(0)
        .scrollbar(false);
    assert_eq!(rows(&ungrouped, 80, 1), ["00000000  48 65 6c 6c  Hell"]);
    assert_eq!(HexViewer::new(SAMPLE).bytes_per_row(0).row_count(), 20);
}

#[test]
fn the_row_width_and_fitting_row_follow_the_layout() {
    let viewer = HexViewer::new(SAMPLE);
    assert_eq!(viewer.row_width(), 76);
    assert_eq!(viewer.row_count(), 2);
    assert_eq!(viewer.fitting_bytes_per_row(109), 24);
    assert_eq!(viewer.fitting_bytes_per_row(108), 16);
    assert_eq!(viewer.fitting_bytes_per_row(76), 16);
    assert_eq!(viewer.fitting_bytes_per_row(75), 8);
    assert_eq!(viewer.fitting_bytes_per_row(10), 1);
    let ungrouped = HexViewer::new(SAMPLE).group(0);
    assert_eq!(ungrouped.fitting_bytes_per_row(79), 17);
    assert_eq!(ungrouped.fitting_bytes_per_row(78), 16);
    assert_eq!(ungrouped.bytes_per_row(17).row_width(), 79);
}

#[test]
fn rows_past_the_area_scroll_with_the_top_row_and_a_scrollbar() {
    let data = counting(100);
    let shown = rows(&HexViewer::new(&data).top_row(2), 80, 3);
    assert!(shown[0].starts_with("00000020  20 21 22"), "{}", shown[0]);
    assert!(shown[2].starts_with("00000040  40 41 42"), "{}", shown[2]);

    let (grid, clusters) = draw(
        &HexViewer::new(&data).top_row(99),
        Rect::new(0, 0, 80, 3),
        80,
        3,
    );
    assert!(
        grid.row_display(0, &clusters)
            .starts_with("00000040  40 41")
    );
    assert!(
        grid.row_display(2, &clusters)
            .starts_with("00000060  60 61 62 63 ")
    );
    assert_eq!(glyph_at(&grid, &clusters, 79, 0), '│');
    assert_eq!(style_at(&grid, 79, 0), fg(Token::Surface1));
    assert_eq!(glyph_at(&grid, &clusters, 79, 2), '█');
    assert_eq!(style_at(&grid, 79, 2), fg(Token::Accent));
}

#[test]
fn the_scrollbar_takes_the_last_column_from_the_ascii() {
    let data = counting(100);
    let (grid, clusters) = draw(&HexViewer::new(&data), Rect::new(0, 0, 76, 3), 76, 3);
    let first = grid.row_display(0, &clusters);
    assert_eq!(first.chars().nth(74), Some('.'));
    assert!(matches!(first.chars().nth(75), Some('█' | '│')));

    let bare = HexViewer::new(&data).scrollbar(false);
    let (grid, clusters) = draw(&bare, Rect::new(0, 0, 76, 3), 76, 3);
    assert_eq!(glyph_at(&grid, &clusters, 75, 0), '.');
}

#[test]
fn the_scrollbar_thumb_follows_the_state() {
    let data = counting(100);
    let thumb = |state| {
        let (grid, _) = draw(
            &HexViewer::new(&data).state(state),
            Rect::new(0, 0, 80, 3),
            80,
            3,
        );
        style_at(&grid, 79, 0)
    };
    assert_eq!(thumb(WidgetState::Normal), fg(Token::Accent));
    assert_eq!(thumb(WidgetState::Focused), fg(Token::FocusRing));
    assert_eq!(thumb(WidgetState::Disabled), dim(fg(Token::TextPrimary)));
}

#[test]
fn rows_that_fit_draw_no_scrollbar() {
    let (grid, clusters) = draw(&HexViewer::new(SAMPLE), Rect::new(0, 0, 80, 2), 80, 2);
    assert_eq!(glyph_at(&grid, &clusters, 79, 0), ' ');
    assert_eq!(glyph_at(&grid, &clusters, 79, 1), ' ');
}

#[test]
fn empty_data_draws_nothing_but_the_header() {
    assert_eq!(rows(&HexViewer::new(&[]), 80, 2), ["", ""]);
    let shown = rows(&HexViewer::new(&[]).header(true), 80, 2);
    assert!(shown[0].starts_with("          00 01"));
    assert_eq!(shown[1], "");
}

#[test]
fn byte_at_finds_bytes_in_both_columns() {
    let viewer = HexViewer::new(SAMPLE);
    let area = Rect::new(0, 0, 80, 4);
    assert_eq!(viewer.byte_at(area, 10, 0), Some(0));
    assert_eq!(viewer.byte_at(area, 11, 0), Some(0));
    assert_eq!(viewer.byte_at(area, 12, 0), None);
    assert_eq!(viewer.byte_at(area, 34, 0), None);
    assert_eq!(viewer.byte_at(area, 35, 0), Some(8));
    assert_eq!(viewer.byte_at(area, 57, 0), Some(15));
    assert_eq!(viewer.byte_at(area, 60, 0), Some(0));
    assert_eq!(viewer.byte_at(area, 75, 0), Some(15));
    assert_eq!(viewer.byte_at(area, 76, 0), None);
    assert_eq!(viewer.byte_at(area, 0, 0), None);
    assert_eq!(viewer.byte_at(area, 9, 0), None);
    assert_eq!(viewer.byte_at(area, 59, 0), None);
    assert_eq!(viewer.byte_at(area, 13, 1), Some(17));
    assert_eq!(viewer.byte_at(area, 20, 1), Some(19));
    assert_eq!(viewer.byte_at(area, 22, 1), None);
    assert_eq!(viewer.byte_at(area, 63, 1), Some(19));
    assert_eq!(viewer.byte_at(area, 64, 1), None);
    assert_eq!(viewer.byte_at(area, 10, 2), None);
    assert_eq!(viewer.byte_at(area, 10, 4), None);
}

#[test]
fn byte_at_follows_the_area_header_scroll_and_scrollbar() {
    let viewer = HexViewer::new(SAMPLE);
    let area = Rect::new(5, 3, 80, 4);
    assert_eq!(viewer.byte_at(area, 15, 3), Some(0));
    assert_eq!(viewer.byte_at(area, 10, 3), None);
    assert_eq!(viewer.byte_at(area, 15, 2), None);
    assert_eq!(viewer.byte_at(area, 4, 3), None);

    let data = counting(100);
    let scrolled = HexViewer::new(&data).header(true).top_row(1);
    let area = Rect::new(0, 0, 80, 3);
    assert_eq!(scrolled.byte_at(area, 10, 0), None);
    assert_eq!(scrolled.byte_at(area, 10, 1), Some(16));
    assert_eq!(scrolled.byte_at(area, 10, 2), Some(32));
    assert_eq!(scrolled.byte_at(area, 10, 3), None);
    assert_eq!(scrolled.byte_at(area, 79, 1), None);
    assert_eq!(
        HexViewer::new(&data).top_row(99).byte_at(area, 10, 0),
        Some(64)
    );
}

#[test]
fn byte_at_follows_a_resized_row() {
    let viewer = HexViewer::new(SAMPLE).bytes_per_row(8).group(4);
    let area = Rect::new(0, 0, 80, 3);
    assert_eq!(viewer.byte_at(area, 21, 0), None);
    assert_eq!(viewer.byte_at(area, 22, 0), None);
    assert_eq!(viewer.byte_at(area, 23, 0), Some(4));
    assert_eq!(viewer.byte_at(area, 24, 0), Some(4));
    assert_eq!(viewer.byte_at(area, 36, 0), Some(0));
    assert_eq!(viewer.byte_at(area, 43, 1), Some(15));
    assert_eq!(viewer.byte_at(area, 44, 1), None);
}

#[test]
fn measure_fits_the_rows_and_the_scrollbar() {
    let viewer = HexViewer::new(SAMPLE);
    assert_eq!(viewer.measure(Size::new(80, 10)), Size::new(76, 2));
    assert_eq!(
        viewer.clone().header(true).measure(Size::new(80, 10)),
        Size::new(76, 3)
    );
    assert_eq!(viewer.measure(Size::new(50, 10)), Size::new(50, 2));
    assert_eq!(viewer.measure(Size::new(80, 2)), Size::new(76, 2));
    assert_eq!(viewer.measure(Size::new(80, 1)), Size::new(77, 1));
    assert_eq!(
        viewer.clone().header(true).measure(Size::new(80, 2)),
        Size::new(77, 2)
    );
    let data = counting(100);
    assert_eq!(
        HexViewer::new(&data).measure(Size::new(80, 3)),
        Size::new(77, 3)
    );
    assert_eq!(
        HexViewer::new(&data)
            .scrollbar(false)
            .measure(Size::new(80, 3)),
        Size::new(76, 3)
    );
}

fn synced(data: &[u8], height: u16, state: &mut HexViewerState) {
    HexViewer::new(data).sync(Rect::new(0, 0, 80, height), state);
}

#[test]
fn a_new_state_starts_on_the_first_byte() {
    let state = HexViewerState::new();
    assert_eq!(state, HexViewerState::default());
    assert_eq!(state.cursor(), 0);
    assert_eq!(state.top_row(), 0);
    assert_eq!(state.selection(), None);
}

#[test]
fn moves_before_a_sync_use_sixteen_byte_rows() {
    let mut state = HexViewerState::new();
    state.move_right(false);
    state.move_right(false);
    assert_eq!(state.cursor(), 2);
    state.move_down(false);
    assert_eq!(state.cursor(), 18);
    state.move_up(false);
    assert_eq!(state.cursor(), 2);
    state.move_up(false);
    assert_eq!(state.cursor(), 2);
    state.move_left(false);
    state.move_left(false);
    state.move_left(false);
    assert_eq!(state.cursor(), 0);
}

#[test]
fn moving_down_stops_on_the_last_byte_and_the_last_row() {
    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    synced(&data, 8, &mut state);
    state.move_to(20, false);
    state.move_down(false);
    assert_eq!(state.cursor(), 36);
    state.move_to(30, false);
    state.move_down(false);
    assert_eq!(state.cursor(), 39);
    state.move_to(36, false);
    state.move_down(false);
    assert_eq!(state.cursor(), 36);
    state.move_right(false);
    state.move_right(false);
    state.move_right(false);
    state.move_right(false);
    assert_eq!(state.cursor(), 39);
}

#[test]
fn pages_move_by_the_visible_rows() {
    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    synced(&data, 2, &mut state);
    state.page_down(false);
    assert_eq!(state.cursor(), 32);
    assert_eq!(state.top_row(), 1);
    state.page_down(false);
    assert_eq!(state.cursor(), 32);
    state.page_up(false);
    assert_eq!(state.cursor(), 0);
    assert_eq!(state.top_row(), 0);
    state.move_to(21, false);
    state.page_up(false);
    assert_eq!(state.cursor(), 5);

    let data = [0u8; 160];
    let mut state = HexViewerState::new();
    synced(&data, 3, &mut state);
    state.move_to(5, false);
    state.page_down(false);
    assert_eq!(state.cursor(), 53);
    assert_eq!(state.top_row(), 1);

    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    synced(&data, 3, &mut state);
    state.move_to(5, false);
    state.page_down(false);
    assert_eq!(state.cursor(), 39);
}

#[test]
fn a_page_before_any_sync_is_one_row() {
    let mut state = HexViewerState::new();
    state.page_down(false);
    assert_eq!(state.cursor(), 16);
    state.page_up(false);
    assert_eq!(state.cursor(), 0);
}

#[test]
fn row_start_and_end_stay_on_the_row() {
    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    synced(&data, 4, &mut state);
    state.move_to(20, false);
    state.row_start(false);
    assert_eq!(state.cursor(), 16);
    state.row_end(false);
    assert_eq!(state.cursor(), 31);
    state.move_to(20, false);
    state.row_end(false);
    assert_eq!(state.cursor(), 31);
    state.move_to(35, false);
    state.row_end(false);
    assert_eq!(state.cursor(), 39);
    state.row_start(false);
    assert_eq!(state.cursor(), 32);

    let narrow = HexViewer::new(&data).bytes_per_row(8);
    narrow.sync(Rect::new(0, 0, 80, 4), &mut state);
    state.move_to(20, false);
    state.row_start(false);
    assert_eq!(state.cursor(), 16);
    state.row_end(false);
    assert_eq!(state.cursor(), 23);
}

#[test]
fn start_and_end_jump_to_the_first_and_last_bytes() {
    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    synced(&data, 2, &mut state);
    state.to_end(false);
    assert_eq!(state.cursor(), 39);
    assert_eq!(state.top_row(), 1);
    state.to_start(false);
    assert_eq!(state.cursor(), 0);
    assert_eq!(state.top_row(), 0);
}

#[test]
fn extending_moves_select_from_the_anchor_to_the_cursor() {
    let mut state = HexViewerState::new();
    state.move_to(4, false);
    state.move_right(true);
    state.move_right(true);
    assert_eq!(state.selection(), Some(4..7));
    state.move_left(false);
    assert_eq!(state.selection(), None);
    assert_eq!(state.cursor(), 5);

    state.move_to(10, false);
    state.move_left(true);
    state.move_left(true);
    state.move_left(true);
    assert_eq!(state.selection(), Some(7..11));
    state.move_right(true);
    assert_eq!(state.selection(), Some(8..11));
    state.move_down(true);
    assert_eq!(state.selection(), Some(10..25));
    state.clear_selection();
    assert_eq!(state.selection(), None);
    assert_eq!(state.cursor(), 24);
}

#[test]
fn a_one_byte_selection_covers_the_cursor() {
    let mut state = HexViewerState::new();
    state.move_to(3, false);
    state.move_to(3, true);
    assert_eq!(state.selection(), Some(3..4));
}

#[test]
fn select_all_covers_every_byte_once_synced() {
    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    state.select_all();
    assert_eq!(state.selection().map(|range| range.start), Some(0));
    synced(&data, 2, &mut state);
    assert_eq!(state.selection(), Some(0..40));
    assert_eq!(state.cursor(), 39);
    assert_eq!(state.top_row(), 1);
    state.move_to(5, false);
    state.select_all();
    assert_eq!(state.selection(), Some(0..40));
}

#[test]
fn a_sync_keeps_the_cursor_and_anchor_on_the_data() {
    let data = [0u8; 40];
    let mut state = HexViewerState::new();
    state.move_to(600, false);
    state.move_to(500, true);
    synced(&data, 10, &mut state);
    assert_eq!(state.cursor(), 39);
    assert_eq!(state.selection(), Some(39..40));
}

#[test]
fn empty_data_leaves_nothing_to_select() {
    let mut state = HexViewerState::new();
    state.select_all();
    synced(&[], 4, &mut state);
    assert_eq!(state.cursor(), 0);
    assert_eq!(state.selection(), None);
    state.move_right(true);
    assert_eq!(state.cursor(), 0);
}

#[test]
fn scrolling_leaves_the_cursor_until_the_next_move() {
    let data = [0u8; 160];
    let mut state = HexViewerState::new();
    synced(&data, 2, &mut state);
    state.scroll_down(3);
    assert_eq!(state.top_row(), 3);
    synced(&data, 2, &mut state);
    assert_eq!(state.top_row(), 3);
    assert_eq!(state.cursor(), 0);
    state.move_right(false);
    assert_eq!(state.top_row(), 0);
    state.scroll_down(100);
    assert_eq!(state.top_row(), 8);
    state.scroll_up(5);
    assert_eq!(state.top_row(), 3);
    state.scroll_up(50);
    assert_eq!(state.top_row(), 0);
}

#[test]
fn a_resize_keeps_the_cursor_on_screen() {
    let data = [0u8; 160];
    let mut state = HexViewerState::new();
    synced(&data, 2, &mut state);
    state.to_end(false);
    assert_eq!(state.top_row(), 8);
    synced(&data, 5, &mut state);
    assert_eq!(state.top_row(), 5);
    synced(&data, 1, &mut state);
    assert_eq!(state.top_row(), 9);
    state.move_up(false);
    assert_eq!(state.top_row(), 8);
}

#[test]
fn a_sync_counts_the_header_out_of_the_page() {
    let data = [0u8; 160];
    let mut state = HexViewerState::new();
    HexViewer::new(&data)
        .header(true)
        .sync(Rect::new(0, 0, 80, 3), &mut state);
    state.page_down(false);
    assert_eq!(state.cursor(), 32);
    state.to_end(false);
    assert_eq!(state.top_row(), 8);
}

#[test]
fn a_viewer_draws_the_state_it_is_given() {
    let data = counting(40);
    let mut state = HexViewerState::new();
    let area = Rect::new(0, 0, 80, 2);
    HexViewer::new(&data).sync(area, &mut state);
    state.move_to(30, false);
    state.to_end(true);
    let viewer = HexViewer::new(&data).view(&state);
    let (grid, clusters) = draw(&viewer, area, 80, 2);
    assert!(
        grid.row_display(0, &clusters)
            .starts_with("00000010  10 11")
    );
    assert_eq!(style_at(&grid, 31, 1), cursor(Token::Accent));
    assert_eq!(style_at(&grid, 67, 1), cursor(Token::Accent));
    assert_eq!(style_at(&grid, 53, 0), selected(fg(Token::Warn)));
    assert_eq!(style_at(&grid, 52, 0), Style::new());
    assert_eq!(style_at(&grid, 50, 0), fg(Token::Warn));
    assert_eq!(viewer.byte_at(area, 31, 1), Some(39));
}

#[test]
fn bytes_keep_the_background_under_the_viewer() {
    let theme = Theme::builtin(Variant::Dark);
    let panel = Color::Rgb(9, 9, 9);
    let mut grid = Grid::new(80, 2);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 80, 2),
        Style::new().bg(panel),
        ' ',
        &mut clusters,
    );
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    HexViewer::new(SAMPLE)
        .selection(Some(2..4))
        .cursor(Some(0))
        .render(Rect::new(0, 0, 80, 2), &mut cx);
    assert_eq!(style_at(&grid, 0, 0), dim(fg(Token::TextPrimary)).bg(panel));
    assert_eq!(style_at(&grid, 13, 0), fg(Token::TextPrimary).bg(panel));
    assert_eq!(style_at(&grid, 61, 0), fg(Token::TextPrimary).bg(panel));
    assert_eq!(style_at(&grid, 12, 0), Style::new().bg(panel));
    assert_eq!(style_at(&grid, 16, 0), selected(fg(Token::TextPrimary)));
    assert_eq!(style_at(&grid, 18, 0), selected(Style::new()));
    assert_eq!(style_at(&grid, 10, 0), cursor(Token::Accent));
}
