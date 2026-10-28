// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, RowDirty, Style};

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|row| grid.row_display(row, clusters))
        .collect()
}

fn source(lines: &[&str], clusters: &mut ClusterTable) -> Grid {
    let width = u16::try_from(lunatui_core::width(lines[0])).unwrap_or(0);
    let height = u16::try_from(lines.len()).unwrap_or(0);
    let mut grid = Grid::new(width, height);
    for (row, line) in (0_u16..).zip(lines) {
        grid.put_str(0, row, Style::new(), line, clusters);
    }
    grid
}

#[test]
fn copies_a_region_to_an_offset() {
    let mut clusters = ClusterTable::new();
    let src = source(&["abcd", "efgh", "ijkl"], &mut clusters);
    let mut dst = Grid::new(5, 3);
    dst.blit(&src, Rect::new(1, 1, 2, 2), 2, 0);
    assert_eq!(rows(&dst, &clusters), ["  fg ", "  jk ", "     "]);
}

#[test]
fn clips_to_the_source_and_the_destination() {
    let mut clusters = ClusterTable::new();
    let src = source(&["abcd", "efgh"], &mut clusters);
    let mut dst = Grid::new(4, 2);
    dst.blit(&src, Rect::new(2, 0, 9, 9), 1, 1);
    assert_eq!(rows(&dst, &clusters), ["    ", " cd "]);
    dst.blit(&src, Rect::new(0, 0, 4, 2), 3, 0);
    assert_eq!(rows(&dst, &clusters), ["   a", " cde"]);
}

#[test]
fn copies_nothing_from_outside_either_grid() {
    let mut clusters = ClusterTable::new();
    let src = source(&["ab"], &mut clusters);
    let mut dst = Grid::new(2, 1);
    dst.blit(&src, Rect::new(2, 0, 1, 1), 0, 0);
    dst.blit(&src, Rect::new(0, 0, 2, 1), 2, 0);
    dst.blit(&src, Rect::new(0, 0, 0, 1), 0, 0);
    assert_eq!(rows(&dst, &clusters), ["  "]);
}

#[test]
fn a_default_background_keeps_the_background_underneath() {
    let mut clusters = ClusterTable::new();
    let mut src = Grid::new(2, 1);
    src.put_str(
        0,
        0,
        Style::new().fg(Color::Rgb(1, 2, 3)),
        "a",
        &mut clusters,
    );
    src.put_str(
        1,
        0,
        Style::new().bg(Color::Rgb(9, 9, 9)),
        "b",
        &mut clusters,
    );
    let mut dst = Grid::new(2, 1);
    dst.fill(
        Rect::new(0, 0, 2, 1),
        Style::new().bg(Color::Rgb(50, 60, 70)),
        '.',
        &mut clusters,
    );
    dst.blit(&src, Rect::new(0, 0, 2, 1), 0, 0);
    let style = |x| dst.get(x, 0).map(lunatui_core::Cell::style);
    assert_eq!(
        style(0),
        Some(
            Style::new()
                .fg(Color::Rgb(1, 2, 3))
                .bg(Color::Rgb(50, 60, 70))
        )
    );
    assert_eq!(style(1), Some(Style::new().bg(Color::Rgb(9, 9, 9))));
    assert_eq!(rows(&dst, &clusters), ["ab"]);
}

#[test]
fn a_wide_glyph_cut_by_the_source_edges_becomes_blank() {
    let mut clusters = ClusterTable::new();
    let src = source(&["世界ab"], &mut clusters);
    let mut dst = Grid::new(4, 1);
    dst.blit(&src, Rect::new(1, 0, 4, 1), 0, 0);
    assert_eq!(rows(&dst, &clusters), [" 界a"]);
    let mut dst = Grid::new(3, 1);
    dst.blit(&src, Rect::new(0, 0, 3, 1), 0, 0);
    assert_eq!(rows(&dst, &clusters), ["世 "]);
}

#[test]
fn a_wide_glyph_in_the_destination_cut_by_the_copy_becomes_blank() {
    let mut clusters = ClusterTable::new();
    let src = source(&["x"], &mut clusters);
    let mut dst = source(&["世界"], &mut clusters);
    dst.blit(&src, Rect::new(0, 0, 1, 1), 1, 0);
    assert_eq!(rows(&dst, &clusters), [" x界"]);
    let mut dst = source(&["世界"], &mut clusters);
    dst.blit(&src, Rect::new(0, 0, 1, 1), 2, 0);
    assert_eq!(rows(&dst, &clusters), ["世x "]);
}

#[test]
fn marks_the_copied_span_dirty() {
    let mut clusters = ClusterTable::new();
    let src = source(&["abc"], &mut clusters);
    let mut dst = Grid::new(6, 2);
    dst.clear_dirty();
    dst.blit(&src, Rect::new(0, 0, 3, 1), 2, 1);
    assert_eq!(dst.row_dirty(0), Some(RowDirty::default()));
    assert_eq!(
        dst.row_dirty(1),
        Some(RowDirty {
            dirty: true,
            min_x: 2,
            max_x: 5,
        })
    );
}
