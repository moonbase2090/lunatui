// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Style};

#[test]
fn ascii_writes_cells_and_preserves_attributes() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(8, 1);
    let bold = Style::new()
        .fg(Color::Indexed(2))
        .with_attrs(Attrs::default().set_bold(true));
    grid.put_str(2, 0, bold, "abc", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "  abc   ");
    assert_eq!(
        grid.get(3, 0)
            .map(Cell::style)
            .map(|style| (style.fg, style.attrs.bold())),
        Some((Color::Indexed(2), true))
    );
    let dirty = grid.row_dirty(0);
    assert_eq!(
        dirty.map(|d| (d.dirty, d.min_x, d.max_x)),
        Some((true, 2, 5))
    );
}

#[test]
fn ascii_clips_at_the_right_edge() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(5, 1);
    grid.put_str(3, 0, Style::new(), "abcdef", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "   ab");
    grid.put_str(5, 0, Style::new(), "zz", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "   ab");
}

#[test]
fn ascii_over_a_wide_glyph_replaces_both_halves() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(8, 1);
    grid.put_str(2, 0, Style::new(), "日", &mut clusters);
    grid.put_str(2, 0, Style::new(), "ab", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "  ab    ");
}

#[test]
fn ascii_starting_on_a_continuation_blanks_the_lead() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(8, 1);
    grid.put_str(2, 0, Style::new(), "日", &mut clusters);
    grid.put_str(3, 0, Style::new(), "z", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "   z    ");
}

#[test]
fn non_ascii_text_still_works_next_to_ascii() {
    let mut clusters = ClusterTable::new();
    let mut grid = Grid::new(8, 1);
    grid.put_str(0, 0, Style::new(), "ab", &mut clusters);
    grid.put_str(2, 0, Style::new(), "日é", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "ab日é   ");
}
