// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Alignment, CellStyle, RenderCx, Table, VecSource, Widget};

#[test]
fn table_renders_visible_rows_only() {
    let mut source = VecSource::new();
    for i in 0..1000 {
        source.push_row(vec![format!("row-{i}"), "ok".to_owned()]);
    }
    let table = Table::new(source, vec![Constraint::Length(8), Constraint::Fill(1)])
        .offset(5)
        .selected(Some(6));
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    table.render(Rect::new(0, 0, 20, 3), &mut cx);
    assert!(grid.row_display(0, &clusters).contains("row-5"));
    assert!(grid.row_display(1, &clusters).contains("row-6"));
}

const SEL: Color = Color::Rgb(42, 35, 87);
const ZEBRA: Color = Color::Rgb(17, 22, 49);
const INK: Color = Color::Rgb(236, 231, 218);
const MUTED: Color = Color::Rgb(170, 176, 204);
const GREEN: Color = Color::Rgb(155, 227, 168);
const LAV: Color = Color::Rgb(185, 167, 240);

fn bench_source() -> VecSource {
    let mut source = VecSource::new();
    for (name, ms) in [("draw", "0.81"), ("diff", "0.19"), ("scroll", "1.44")] {
        source.push_row(vec![name.to_owned(), ms.to_owned(), "ok".to_owned()]);
    }
    source
}

fn render_table<C: CellStyle>(table: &Table<VecSource, C>, width: u16, height: u16) -> Grid {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    table.render(Rect::new(0, 0, width, height), &mut cx);
    grid
}

fn row_text(grid: &Grid, y: u16) -> String {
    let clusters = ClusterTable::new();
    grid.row_display(y, &clusters)
}

fn columns() -> Vec<Constraint> {
    vec![
        Constraint::Length(7),
        Constraint::Length(5),
        Constraint::Fill(1),
    ]
}

#[test]
fn header_row_sits_above_the_data_with_its_own_styles() {
    let table = Table::new(bench_source(), columns()).header([
        ("Name", Style::new().fg(MUTED)),
        ("ms", Style::new().fg(LAV)),
        ("", Style::new()),
    ]);
    let grid = render_table(&table, 16, 4);
    assert_eq!(row_text(&grid, 0), "Name   ms       ");
    assert_eq!(row_text(&grid, 1), "draw   0.81 ok  ");
    assert_eq!(grid.get(0, 0).map(|cell| cell.style().fg), Some(MUTED));
    assert_eq!(grid.get(7, 0).map(|cell| cell.style().fg), Some(LAV));
}

#[test]
fn right_aligned_columns_end_at_the_column_edge() {
    let table = Table::new(bench_source(), columns()).alignments(vec![
        Alignment::Left,
        Alignment::Right,
        Alignment::Right,
    ]);
    let grid = render_table(&table, 16, 3);
    assert_eq!(row_text(&grid, 0), "draw    0.81  ok");
}

#[test]
fn column_and_cell_styles_layer_in_order() {
    let mut cells = vec![vec![Style::new(); 3]; 3];
    cells[1][2] = Style::new().fg(GREEN);
    let table = Table::new(bench_source(), columns())
        .style(Style::new().fg(INK))
        .column_styles(vec![Style::new(), Style::new().fg(MUTED)])
        .cell_styles(cells);
    let grid = render_table(&table, 16, 3);
    let fg = |x, y| grid.get(x, y).map(|cell| cell.style().fg);
    assert_eq!(fg(0, 0), Some(INK), "no column style: table style");
    assert_eq!(fg(7, 0), Some(MUTED), "column style");
    assert_eq!(fg(12, 0), Some(INK));
    assert_eq!(fg(12, 1), Some(GREEN), "cell style over the column");
}

#[test]
fn highlight_symbol_marks_only_the_selected_row_and_the_band_fills_the_row() {
    let table = Table::new(bench_source(), columns())
        .selected(Some(1))
        .highlight_style(Style::new().fg(INK).bg(SEL))
        .highlight_symbol("▌", Style::new().fg(LAV).bg(SEL));
    let grid = render_table(&table, 16, 3);
    assert_eq!(row_text(&grid, 0), " draw   0.81 ok ");
    assert_eq!(row_text(&grid, 1), "▌diff   0.19 ok ");
    assert_eq!(grid.get(0, 1).map(|cell| cell.style().fg), Some(LAV));
    for x in 0..16 {
        assert_eq!(
            grid.get(x, 1).map(|cell| cell.style().bg),
            Some(SEL),
            "x={x}"
        );
    }
    assert_ne!(grid.get(5, 0).map(|cell| cell.style().bg), Some(SEL));
}

#[test]
fn odd_rows_get_a_full_width_band_except_the_selected_one() {
    let mut source = bench_source();
    source.push_row(vec!["sort".to_owned(), "0.07".to_owned(), "ok".to_owned()]);
    let table = Table::new(source, columns())
        .selected(Some(1))
        .highlight_style(Style::new().bg(SEL))
        .odd_row_style(Style::new().bg(ZEBRA));
    let grid = render_table(&table, 16, 4);
    let bg = |x, y| grid.get(x, y).map(|cell| cell.style().bg);
    assert_eq!(bg(15, 1), Some(SEL));
    assert_eq!(bg(15, 3), Some(ZEBRA));
    assert_eq!(bg(6, 3), Some(ZEBRA));
    assert_ne!(bg(15, 2), Some(ZEBRA));
}

#[test]
fn row_gap_leaves_blank_rows_between_data_rows() {
    let table = Table::new(bench_source(), columns()).row_gap(1);
    let grid = render_table(&table, 16, 5);
    assert_eq!(row_text(&grid, 0), "draw   0.81 ok  ");
    assert_eq!(row_text(&grid, 1), "                ");
    assert_eq!(row_text(&grid, 2), "diff   0.19 ok  ");
    assert_eq!(row_text(&grid, 4), "scroll 1.44 ok  ");
}
