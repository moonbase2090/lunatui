// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{BarChart, RenderCx, Widget};

const LAV: Color = Color::Rgb(185, 167, 240);
const LAV2: Color = Color::Rgb(143, 124, 224);
const TRACK: Color = Color::Rgb(22, 28, 61);

fn render(chart: &BarChart<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    chart.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|y| grid.row_display(y, clusters))
        .collect()
}

#[test]
fn bars_use_eighth_block_tops_at_their_width() {
    let chart = BarChart::new(&[4, 13, 16, 1]).max(16).bar_width(2);
    let (grid, clusters) = render(&chart, 11, 2);
    assert_eq!(rows(&grid, &clusters), ["   ▅▅ ██   ", "▄▄ ██ ██ ▁▁"]);
}

#[test]
fn levels_below_one_eighth_leave_the_row_blank() {
    let chart = BarChart::new(&[16, 17]).max(32).bar_gap(0);
    let (grid, clusters) = render(&chart, 2, 2);
    assert_eq!(rows(&grid, &clusters), ["  ", "██"]);
}

#[test]
fn bar_gap_sets_the_cells_between_bars() {
    let chart = BarChart::new(&[16, 16, 16]).max(16).bar_width(2).bar_gap(0);
    let (grid, clusters) = render(&chart, 7, 1);
    assert_eq!(rows(&grid, &clusters), ["██████ "]);
}

#[test]
fn bar_styles_override_the_chart_style_and_the_track_fills_bar_cells() {
    let styles = [Style::new(), Style::new().fg(LAV)];
    let chart = BarChart::new(&[8, 16])
        .max(16)
        .bar_width(2)
        .style(Style::new().fg(LAV2))
        .bar_styles(&styles)
        .track_style(Style::new().bg(TRACK));
    let (grid, _) = render(&chart, 5, 2);
    let style = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default();
    assert_eq!(style(0, 1).fg, LAV2);
    assert_eq!(style(3, 0).fg, LAV);
    assert_eq!(style(0, 0).bg, TRACK, "empty part of a bar");
    assert_eq!(
        style(0, 1).bg,
        TRACK,
        "filled part keeps the track background"
    );
    assert_ne!(style(2, 0).bg, TRACK, "the gap is not part of the track");
}

#[test]
fn labels_sit_under_their_bars() {
    let chart = BarChart::new(&[16, 8])
        .max(16)
        .bar_width(2)
        .labels(&["M", "T"])
        .label_style(Style::new().fg(LAV2));
    let (grid, clusters) = render(&chart, 5, 3);
    assert_eq!(rows(&grid, &clusters), ["██   ", "██ ██", "M  T "]);
    assert_eq!(grid.get(3, 2).map(|cell| cell.style().fg), Some(LAV2));
}

#[test]
fn a_bar_at_the_edge_is_clipped_to_the_area() {
    let chart = BarChart::new(&[16, 16, 16]).max(16).bar_width(2);
    let (grid, clusters) = render(&chart, 7, 1);
    assert_eq!(rows(&grid, &clusters), ["██ ██ █"]);
}

#[test]
fn a_second_render_clears_old_bars_gaps_and_labels() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(5, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    for y in 0..3 {
        cx.put_str(0, y, "xxxxx", Style::new());
    }
    BarChart::new(&[16, 16])
        .max(16)
        .bar_width(2)
        .labels(&["Mon", "Tue"])
        .render(Rect::new(0, 0, 5, 3), &mut cx);
    BarChart::new(&[8])
        .max(16)
        .bar_width(2)
        .render(Rect::new(0, 0, 5, 3), &mut cx);
    assert_eq!(rows(&grid, &clusters), ["     ", "▄▄   ", "██   "]);
}

#[test]
fn an_empty_render_clears_the_previous_bars() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(5, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    BarChart::new(&[16, 16])
        .max(16)
        .bar_width(2)
        .render(Rect::new(0, 0, 5, 2), &mut cx);
    BarChart::new(&[]).render(Rect::new(0, 0, 5, 2), &mut cx);
    assert_eq!(rows(&grid, &clusters), ["     ", "     "]);
}

#[test]
fn a_render_without_a_track_drops_the_old_track_background() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(2, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    BarChart::new(&[16])
        .max(16)
        .bar_width(2)
        .track_style(Style::new().bg(TRACK))
        .render(Rect::new(0, 0, 2, 2), &mut cx);
    BarChart::new(&[8])
        .max(16)
        .bar_width(2)
        .render(Rect::new(0, 0, 2, 2), &mut cx);
    let style = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default();
    assert_eq!(rows(&grid, &clusters), ["  ", "██"]);
    assert_eq!(
        style(0, 0),
        Style::new(),
        "the shrunk bar's old cell is reset"
    );
    assert_eq!(
        style(0, 1).bg,
        Color::Default,
        "the bar keeps no track background"
    );
}

#[test]
fn labels_are_clipped_to_their_bar() {
    let chart = BarChart::new(&[16, 16])
        .max(16)
        .bar_width(2)
        .labels(&["Mon", "Tue"]);
    let (grid, clusters) = render(&chart, 5, 2);
    assert_eq!(rows(&grid, &clusters), ["██ ██", "Mo Tu"]);
}
