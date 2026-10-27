// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Bar, BarChart, BarDirection, BarGroup, RenderCx, Widget, WidgetState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::widgets::Widget as RatWidget;
use ratatui::widgets::{Bar as RatBar, BarChart as RatBarChart, BarGroup as RatGroup};

fn paint(chart: &BarChart<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(chart: &BarChart<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = paint(chart, width, height);
    (0..height)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn token(variant_token: Token) -> lunatui_core::Color {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    RenderCx::new(&mut grid, &mut clusters, &theme).token(variant_token)
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .unwrap_or_default()
}

fn bars(values: &[u64]) -> Vec<Bar<'static>> {
    values.iter().map(|&value| Bar::new(value)).collect()
}

#[test]
fn vertical_bars_rise_from_the_bottom() {
    let data = bars(&[0, 4, 8]);
    assert_eq!(rows(&BarChart::from_bars(&data), 5, 2), ["    █", "  █ █"]);
}

#[test]
fn partial_heights_use_eighth_blocks() {
    for (value, glyph) in [
        (1, "▁"),
        (2, "▂"),
        (3, "▃"),
        (4, "▄"),
        (5, "▅"),
        (6, "▆"),
        (7, "▇"),
    ] {
        let data = bars(&[value, 8]);
        assert_eq!(
            rows(&BarChart::from_bars(&data), 3, 1),
            [format!("{glyph} █")]
        );
    }
}

#[test]
fn max_scales_the_bars() {
    let data = bars(&[4]);
    assert_eq!(rows(&BarChart::from_bars(&data).max(8), 1, 1), ["▄"]);
    assert_eq!(rows(&BarChart::from_bars(&data), 1, 1), ["█"]);
    assert_eq!(rows(&BarChart::from_bars(&data).max(2), 1, 1), ["█"]);
}

#[test]
fn zero_max_and_zero_values_draw_nothing() {
    let data = bars(&[0, 0]);
    assert_eq!(rows(&BarChart::from_bars(&data), 3, 2), ["   ", "   "]);
    let data = bars(&[5]);
    assert_eq!(rows(&BarChart::from_bars(&data).max(0), 1, 1), [" "]);
}

#[test]
fn bar_width_and_gap_set_the_layout() {
    let data = bars(&[8, 4]);
    let chart = BarChart::from_bars(&data).bar_width(2).bar_gap(0);
    assert_eq!(rows(&chart, 4, 2), ["██  ", "████"]);
    let chart = BarChart::from_bars(&data).bar_width(2).bar_gap(2);
    assert_eq!(rows(&chart, 6, 2), ["██    ", "██  ██"]);
}

#[test]
fn bar_width_zero_is_one() {
    let data = bars(&[8]);
    assert_eq!(rows(&BarChart::from_bars(&data).bar_width(0), 2, 1), ["█ "]);
}

#[test]
fn bars_past_the_edge_are_clipped() {
    let data = bars(&[8, 8, 8, 8]);
    assert_eq!(rows(&BarChart::from_bars(&data), 3, 1), ["█ █"]);
    let wide = BarChart::from_bars(&data).bar_width(3);
    assert_eq!(rows(&wide, 5, 1), ["███ █"]);
}

#[test]
fn degenerate_areas_do_not_panic() {
    let data = bars(&[1, 2, 3]);
    let chart = BarChart::from_bars(&data);
    for (width, height) in [(0, 0), (0, 3), (3, 0), (1, 1)] {
        let (_, _) = paint(&chart, width, height);
    }
    let empty: [Bar<'_>; 0] = [];
    assert_eq!(rows(&BarChart::from_bars(&empty), 3, 1), ["   "]);
    let horizontal = chart.clone().direction(BarDirection::Horizontal);
    for (width, height) in [(0, 0), (0, 3), (3, 0), (1, 1)] {
        let (_, _) = paint(&horizontal, width, height);
    }
}

#[test]
fn bar_labels_take_the_bottom_row() {
    let data = [Bar::new(8).label("a"), Bar::new(4).label("b")];
    assert_eq!(rows(&BarChart::from_bars(&data), 3, 2), ["█ ▄", "a b"]);
}

#[test]
fn bar_labels_center_and_clip_to_the_bar() {
    let data = [Bar::new(8).label("ab"), Bar::new(8).label("wxyz")];
    let chart = BarChart::from_bars(&data).bar_width(3).bar_gap(1);
    assert_eq!(rows(&chart, 7, 2), ["███ ███", "ab  wx…"]);
}

#[test]
fn group_labels_sit_under_bar_labels() {
    let left = [Bar::new(8).label("a")];
    let right = [Bar::new(8).label("b")];
    let groups = [
        BarGroup::new(&left).label("G1"),
        BarGroup::new(&right).label("G2"),
    ];
    let chart = BarChart::from_groups(&groups).bar_width(2).group_gap(1);
    assert_eq!(rows(&chart, 5, 3), ["██ ██", "a  b ", "G1 G2"]);
}

#[test]
fn groups_are_separated_by_the_group_gap() {
    let left = bars(&[8, 8]);
    let right = bars(&[8]);
    let groups = [BarGroup::new(&left), BarGroup::new(&right)];
    let chart = BarChart::from_groups(&groups);
    assert_eq!(rows(&chart, 7, 1), ["█ █  █ "]);
    let chart = BarChart::from_groups(&groups).bar_gap(0).group_gap(1);
    assert_eq!(rows(&chart, 4, 1), ["██ █"]);
}

#[test]
fn text_value_is_drawn_on_the_bar() {
    let data = [Bar::new(8).text_value("12")];
    let chart = BarChart::from_bars(&data).bar_width(3);
    let (grid, clusters) = paint(&chart, 3, 1);
    assert_eq!(grid.row_display(0, &clusters), "12█");
    let accent = token(Token::Accent);
    for x in 0..2 {
        let style = style_at(&grid, x, 0);
        assert_eq!(style.bg, accent, "text sits on the bar color at {x}");
        assert_eq!(style.fg, token(Token::Surface0), "fg at {x}");
    }
}

#[test]
fn show_values_prints_the_number_and_text_value_wins() {
    let data = [Bar::new(8), Bar::new(8).text_value("x")];
    let plain = BarChart::from_bars(&data).bar_width(2);
    assert_eq!(rows(&plain, 5, 1), ["██ x█"]);
    let shown = plain.clone().show_values(true);
    assert_eq!(rows(&shown, 5, 1), ["8█ x█"]);
}

#[test]
fn value_on_a_short_bar_uses_the_value_style() {
    let data = [Bar::new(0).text_value("0")];
    let value_style = Style::new().fg(lunatui_core::Color::Rgb(1, 2, 3));
    let chart = BarChart::from_bars(&data).value_style(value_style);
    let (grid, clusters) = paint(&chart, 1, 2);
    assert_eq!(grid.row_display(1, &clusters), "0");
    assert_eq!(style_at(&grid, 0, 1).fg, lunatui_core::Color::Rgb(1, 2, 3));
}

#[test]
fn horizontal_bars_grow_to_the_right() {
    let data = [Bar::new(8).label("aa"), Bar::new(4).label("b")];
    let chart = BarChart::from_bars(&data).direction(BarDirection::Horizontal);
    assert_eq!(rows(&chart, 8, 3), ["aa █████", "        ", "b  ██▌  "]);
}

#[test]
fn horizontal_bars_without_labels_start_at_the_edge() {
    let data = bars(&[8, 4]);
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_gap(0);
    assert_eq!(rows(&chart, 4, 2), ["████", "██  "]);
}

#[test]
fn horizontal_bar_width_is_the_row_thickness() {
    let data = bars(&[8, 8]);
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(2);
    assert_eq!(rows(&chart, 2, 5), ["██", "██", "  ", "██", "██"]);
    assert_eq!(rows(&chart, 2, 4), ["██", "██", "  ", "██"]);
}

#[test]
fn horizontal_groups_get_a_heading_line() {
    let first = [Bar::new(8).label("x")];
    let second = [Bar::new(4).label("y")];
    let groups = [
        BarGroup::new(&first).label("One"),
        BarGroup::new(&second).label("Two"),
    ];
    let chart = BarChart::from_groups(&groups)
        .direction(BarDirection::Horizontal)
        .group_gap(1);
    assert_eq!(
        rows(&chart, 6, 5),
        ["One   ", "x █████", "      ", "Two   ", "y ██   "]
            .map(|row| row.chars().take(6).collect::<String>())
    );
}

#[test]
fn horizontal_value_goes_inside_a_long_bar_and_after_a_short_one() {
    let data = [Bar::new(16).text_value("80"), Bar::new(1).text_value("1")];
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_gap(0);
    assert_eq!(rows(&chart, 8, 2), ["80██████", "▌ 1     "]);
}

#[test]
fn state_picks_theme_tokens_for_bars() {
    let data = bars(&[8]);
    let color = |state| {
        let chart = BarChart::from_bars(&data).state(state);
        let (grid, _) = paint(&chart, 1, 1);
        style_at(&grid, 0, 0)
    };
    assert_eq!(color(WidgetState::Normal).fg, token(Token::Accent));
    assert_eq!(color(WidgetState::Focused).fg, token(Token::FocusRing));
    assert_eq!(color(WidgetState::Disabled).fg, token(Token::TextPrimary));
    assert!(color(WidgetState::Disabled).attrs.dim());
    assert!(!color(WidgetState::Normal).attrs.dim());
}

#[test]
fn explicit_styles_win_over_state() {
    let custom = Style::new().fg(lunatui_core::Color::Rgb(9, 8, 7));
    let own = Style::new().fg(lunatui_core::Color::Rgb(1, 1, 1));
    let data = [Bar::new(8), Bar::new(8).style(own)];
    let chart = BarChart::from_bars(&data)
        .state(WidgetState::Disabled)
        .style(custom);
    let (grid, _) = paint(&chart, 3, 1);
    let fg = |x| grid.get(x, 0).map(|cell| cell.style().fg);
    assert_eq!(fg(0), Some(custom.fg));
    assert_eq!(fg(2), Some(own.fg));
}

#[test]
fn label_style_is_applied_to_labels() {
    let data = [Bar::new(8).label("a")];
    let label_style = Style::new().fg(lunatui_core::Color::Rgb(4, 5, 6));
    let chart = BarChart::from_bars(&data).label_style(label_style);
    let (grid, _) = paint(&chart, 1, 2);
    assert_eq!(
        grid.get(0, 1).map(|cell| cell.style().fg),
        Some(label_style.fg)
    );
}

fn rat_rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn bar_glyphs_match_the_reference_chart() {
    let values = [0_u64, 1, 3, 4, 6, 7, 8, 5];
    let data = bars(&values);
    let ours = rows(&BarChart::from_bars(&data), 15, 3);
    let area = RatRect::new(0, 0, 15, 3);
    let mut buffer = Buffer::empty(area);
    let reference_bars: Vec<RatBar<'_>> = values
        .iter()
        .map(|&value| RatBar::default().value(value).text_value(String::new()))
        .collect();
    RatWidget::render(
        RatBarChart::default()
            .data(RatGroup::default().bars(&reference_bars))
            .bar_width(1)
            .bar_gap(1)
            .max(8),
        area,
        &mut buffer,
    );
    assert_eq!(ours, rat_rows(&buffer));
}

#[test]
fn bars_keep_the_background_behind_them() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(3, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let panel = lunatui_core::Color::Rgb(10, 20, 30);
    let data = [Bar::new(4).label("a"), Bar::new(1)];
    BarChart::from_bars(&data)
        .background(Style::new().bg(panel))
        .max(8)
        .render(Rect::new(0, 0, 3, 2), &mut cx);
    for (x, y) in [(0, 0), (0, 1), (1, 0), (2, 0)] {
        assert_eq!(style_at(&grid, x, y).bg, panel, "background at {x},{y}");
    }
    assert_eq!(style_at(&grid, 0, 0).fg, token(Token::Accent));
}

#[test]
fn value_sits_above_a_bar_that_leaves_room() {
    let data = [Bar::new(4).text_value("7")];
    let chart = BarChart::from_bars(&data).max(8);
    assert_eq!(rows(&chart, 1, 3), ["7", "▄", "█"]);
}

#[test]
fn long_text_is_cut_with_an_ellipsis() {
    let data = [Bar::new(8).text_value("12345")];
    assert_eq!(
        rows(&BarChart::from_bars(&data).bar_width(3), 3, 1),
        ["12…"]
    );
    assert_eq!(rows(&BarChart::from_bars(&data), 1, 1), ["…"]);
}

fn rows_at(chart: &BarChart<'_>, width: u16, height: u16, ox: u16, oy: u16) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width + ox + 2, height + oy + 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(ox, oy, width, height), &mut cx);
    let blank = " ".repeat(usize::from(width + ox + 2));
    for y in (0..oy).chain(oy + height..height + oy + 2) {
        assert_eq!(grid.row_display(y, &clusters), blank, "row {y} is outside");
    }
    (oy..oy + height)
        .map(|y| {
            let row = grid.row_display(y, &clusters);
            assert!(row.chars().take(usize::from(ox)).all(|c| c == ' '));
            assert!(row.chars().skip(usize::from(ox + width)).all(|c| c == ' '));
            row.chars()
                .skip(usize::from(ox))
                .take(usize::from(width))
                .collect()
        })
        .collect()
}

#[test]
fn charts_draw_the_same_at_any_offset() {
    let left = [
        Bar::new(8).label("a").text_value("88"),
        Bar::new(3).label("b").text_value("3"),
    ];
    let right = [Bar::new(5).label("c")];
    let groups = [
        BarGroup::new(&left).label("one"),
        BarGroup::new(&right).label("two"),
    ];
    let vertical = BarChart::from_groups(&groups).bar_width(2).max(8);
    let horizontal = BarChart::from_groups(&groups)
        .direction(BarDirection::Horizontal)
        .bar_width(2)
        .max(8);
    for chart in [&vertical, &horizontal] {
        let at_origin = rows(chart, 14, 9);
        assert_eq!(rows_at(chart, 14, 9, 0, 0), at_origin);
        assert_eq!(rows_at(chart, 14, 9, 3, 2), at_origin);
    }
}

#[test]
fn value_text_sits_inside_a_bar_that_fills_the_chart() {
    let tall = [Bar::new(8).text_value("9")];
    assert_eq!(rows(&BarChart::from_bars(&tall), 1, 3), ["9", "█", "█"]);
    let nearly = [Bar::new(7).text_value("7")];
    assert_eq!(rows(&BarChart::from_bars(&nearly).max(8), 1, 1), ["7"]);
    let empty = [Bar::new(0).text_value("z"), Bar::new(8)];
    assert_eq!(rows(&BarChart::from_bars(&empty), 3, 2), ["  █", "z █"]);
}

#[test]
fn horizontal_value_text_follows_the_bar_end() {
    let after = [Bar::new(4).label("ab").text_value("1234")];
    let chart = BarChart::from_bars(&after)
        .direction(BarDirection::Horizontal)
        .max(8);
    assert_eq!(rows(&chart, 10, 1), ["ab ███▌ 1…"]);
    let inside = [Bar::new(8).label("ab").text_value("42")];
    let chart = BarChart::from_bars(&inside).direction(BarDirection::Horizontal);
    assert_eq!(rows(&chart, 10, 1), ["ab 42█████"]);
}

#[test]
fn horizontal_labels_sit_on_the_middle_row_of_a_thick_bar() {
    let data = [Bar::new(8).label("a")];
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(3);
    assert_eq!(rows(&chart, 6, 3), ["  ████", "a ████", "  ████"]);
}

#[test]
fn horizontal_label_column_is_at_most_half_the_width() {
    let data = [Bar::new(8).label("abcdefgh")];
    let chart = BarChart::from_bars(&data).direction(BarDirection::Horizontal);
    assert_eq!(rows(&chart, 6, 1), ["ab… ██"]);
}

#[test]
fn an_empty_height_draws_nothing_even_with_labels() {
    let data = [Bar::new(8).label("a")];
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    BarChart::from_bars(&data).render(Rect::new(0, 0, 3, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "   ");
}

#[test]
fn partial_tops_fill_the_bar_width() {
    let data = [Bar::new(4), Bar::new(8)];
    let chart = BarChart::from_bars(&data).bar_width(2).max(8);
    assert_eq!(rows(&chart, 5, 1), ["▄▄ ██"]);
}

#[test]
fn a_bar_cut_by_the_edge_stays_inside_the_area() {
    let data = bars(&[8, 8]);
    let chart = BarChart::from_bars(&data).bar_width(3);
    assert_eq!(rows_at(&chart, 5, 1, 2, 1), ["███ █"]);
    let horizontal = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(2);
    assert_eq!(rows_at(&horizontal, 2, 4, 1, 1), ["██", "██", "  ", "██"]);
}

#[test]
fn empty_cells_above_a_bar_carry_its_style() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let data = bars(&[4]);
    BarChart::from_bars(&data)
        .max(8)
        .render(Rect::new(0, 0, 1, 2), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), " ");
    assert_eq!(style_at(&grid, 0, 0).fg, token(Token::Accent));
}

#[test]
fn text_value_in_a_chart_with_no_plot_rows_is_skipped() {
    let data = [Bar::new(8).label("a").text_value("9")];
    assert_eq!(rows(&BarChart::from_bars(&data), 1, 1), ["a"]);
}

#[test]
fn value_text_is_one_row_above_a_short_bar() {
    let data = [Bar::new(1).text_value("7")];
    let chart = BarChart::from_bars(&data).max(3);
    assert_eq!(rows(&chart, 1, 3), [" ", "7", "█"]);
}

#[test]
fn horizontal_partial_ends_fill_the_bar_thickness() {
    let data = [Bar::new(4)];
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(2)
        .max(8);
    assert_eq!(rows(&chart, 4, 2), ["██  ", "██  "]);
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(2)
        .max(16);
    assert_eq!(rows(&chart, 4, 2), ["█   ", "█   "]);
    let data = [Bar::new(3)];
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(2)
        .max(16);
    assert_eq!(rows(&chart, 4, 2), ["▊   ", "▊   "]);
}

#[test]
fn horizontal_label_and_value_use_the_upper_middle_row() {
    let data = [Bar::new(8).label("a").text_value("9")];
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(2);
    assert_eq!(rows(&chart, 4, 2), ["a 9█", "  ██"]);
    let chart = BarChart::from_bars(&data)
        .direction(BarDirection::Horizontal)
        .bar_width(3);
    assert_eq!(rows(&chart, 4, 3), ["  ██", "a 9█", "  ██"]);
}

#[test]
fn a_second_render_leaves_nothing_of_the_first() {
    let theme = Theme::builtin(Variant::Dark);
    let area = Rect::new(0, 0, 24, 6);
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let big = [
        Bar::new(100).label("ab").text_value("100"),
        Bar::new(60).text_value("60"),
    ];
    let small = [Bar::new(10)];
    let cells = |grid: &Grid| -> Vec<u128> {
        (0..6)
            .flat_map(|y| (0..24).map(move |x| (x, y)))
            .map(|(x, y)| grid.cell_bits(x, y))
            .collect()
    };
    for direction in [BarDirection::Vertical, BarDirection::Horizontal] {
        let chart = |bars| {
            BarChart::from_bars(bars)
                .direction(direction)
                .max(100)
                .background(panel)
        };
        let mut grid = Grid::new(24, 6);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        chart(&big).render(area, &mut cx);
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        chart(&small).render(area, &mut cx);
        let mut fresh = Grid::new(24, 6);
        let mut fresh_clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut fresh, &mut fresh_clusters, &theme);
        chart(&small).render(area, &mut cx);
        assert_eq!(cells(&grid), cells(&fresh), "{direction:?}");
    }
}

#[test]
fn the_background_style_fills_cells_the_chart_does_not_draw() {
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let data = [Bar::new(8)];
    let chart = BarChart::from_bars(&data).background(panel);
    let (grid, _) = paint(&chart, 3, 2);
    assert_eq!(style_at(&grid, 2, 0), panel);
    assert_eq!(style_at(&grid, 1, 1), panel);
    let (grid, _) = paint(&BarChart::from_bars(&data), 3, 2);
    assert_eq!(style_at(&grid, 2, 0), Style::new());
}
