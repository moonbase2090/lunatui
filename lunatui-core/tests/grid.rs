// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{CellWidth, ClusterTable, Color, GlyphSlot, Grid, Rect, RowDirty, Style};
use proptest::prelude::*;

fn assert_wide_invariants(grid: &Grid) {
    let w = grid.width();
    let h = grid.height();
    for y in 0..h {
        for x in 0..w {
            assert_ne!(grid.get(x, y), None);
            let cell = grid.get(x, y);
            let Some(cell) = cell else {
                continue;
            };
            match cell.meta().width {
                CellWidth::Two => {
                    assert_ne!(x, w - 1);
                    let tail = grid.get(x + 1, y);
                    assert_eq!(
                        tail.map(lunatui_core::Cell::glyph),
                        Some(GlyphSlot::continuation())
                    );
                    assert_eq!(tail.map(|c| c.meta().width), Some(CellWidth::Continuation));
                    assert_eq!(tail.map(lunatui_core::Cell::style), Some(cell.style()));
                }
                CellWidth::Continuation => {
                    assert!(x > 0);
                    let lead = grid.get(x - 1, y);
                    assert_eq!(lead.map(|c| c.meta().width), Some(CellWidth::Two));
                }
                CellWidth::One => {}
            }
        }
    }
}

#[test]
fn put_str_marks_dirty_span_exactly() {
    let mut grid = Grid::new(12, 3);
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    grid.clear_dirty();
    grid.put_str(3, 1, style, "ab", &mut clusters);
    assert_eq!(
        grid.row_dirty(1),
        Some(RowDirty {
            dirty: true,
            min_x: 3,
            max_x: 5,
        })
    );
    assert_eq!(grid.row_dirty(0), Some(RowDirty::default()));
}

#[test]
fn wide_grapheme_uses_lead_and_continuation() {
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    let style = Style::new().fg(Color::Indexed(12));
    grid.put_str(1, 0, style, "日", &mut clusters);
    assert_eq!(grid.get(1, 0).map(|c| c.meta().width), Some(CellWidth::Two));
    assert_eq!(
        grid.get(2, 0).map(|c| c.meta().width),
        Some(CellWidth::Continuation)
    );
    assert_wide_invariants(&grid);
}

#[test]
fn wide_at_right_edge_becomes_blank() {
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(2, 0, Style::new(), "日", &mut clusters);
    assert_eq!(grid.get(2, 0).map(|c| c.meta().width), Some(CellWidth::One));
    assert_eq!(
        grid.get(2, 0).map(|c| c.glyph().decode()),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
}

#[test]
fn overwriting_continuation_clears_pair() {
    let mut grid = Grid::new(6, 1);
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    grid.put_str(1, 0, style, "日", &mut clusters);
    grid.put_str(2, 0, style, "x", &mut clusters);
    assert_eq!(
        grid.get(1, 0).map(|c| c.glyph().decode()),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    assert_wide_invariants(&grid);
}

#[test]
fn fill_wide_char_in_one_column_area_becomes_blank() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        },
        Style::new(),
        '日',
        &mut clusters,
    );
    assert_eq!(
        grid.get(1, 0).map(|c| c.glyph().decode()),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    assert_eq!(grid.get(1, 0).map(|c| c.meta().width), Some(CellWidth::One));
    assert_wide_invariants(&grid);
}

#[test]
fn fill_wide_char_at_grid_edge_becomes_blank() {
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect {
            x: 2,
            y: 0,
            width: 1,
            height: 1,
        },
        Style::new(),
        '日',
        &mut clusters,
    );
    assert_eq!(
        grid.get(2, 0).map(|c| c.glyph().decode()),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    assert_wide_invariants(&grid);
}

#[test]
fn set_style_on_continuation_also_styles_lead() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "日", &mut clusters);
    grid.set_style(
        Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        },
        Style::new().fg(Color::Indexed(11)),
    );
    assert_eq!(
        grid.get(0, 0).map(|c| c.style().fg),
        Some(Color::Indexed(11))
    );
    assert_eq!(
        grid.get(1, 0).map(|c| c.style().fg),
        Some(Color::Indexed(11))
    );
}

#[test]
fn set_style_on_lead_only_in_area_also_styles_continuation() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "日", &mut clusters);
    grid.set_style(
        Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        Style::new().fg(Color::Indexed(10)),
    );
    assert_eq!(
        grid.get(0, 0).map(|c| c.style().fg),
        Some(Color::Indexed(10))
    );
    assert_eq!(
        grid.get(1, 0).map(|c| c.style().fg),
        Some(Color::Indexed(10))
    );
}

#[test]
fn set_style_updates_colors_without_changing_glyph() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "hi", &mut clusters);
    grid.set_style(
        Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        },
        Style::new().fg(Color::Indexed(9)),
    );
    assert_eq!(
        grid.get(0, 0).map(|c| c.style().fg),
        Some(Color::Indexed(9))
    );
    assert_eq!(
        grid.get(0, 0).map(|c| c.glyph().decode()),
        Some(lunatui_core::DecodedGlyph::Inline('h'))
    );
}

#[test]
fn fill_covers_rectangle_and_dirty_span() {
    let mut grid = Grid::new(10, 4);
    let mut clusters = ClusterTable::new();
    grid.clear_dirty();
    grid.fill(
        Rect {
            x: 2,
            y: 1,
            width: 3,
            height: 2,
        },
        Style::new(),
        '.',
        &mut clusters,
    );
    let row = grid.row_dirty(1);
    assert_eq!(row.map(|r| r.min_x), Some(2));
    assert_eq!(row.map(|r| r.max_x), Some(5));
    assert_eq!(row.map(|r| r.dirty), Some(true));
    assert_eq!(
        grid.get(2, 1).map(|c| c.glyph().decode()),
        Some(lunatui_core::DecodedGlyph::Inline('.'))
    );
}

#[test]
fn styled_snapshot_has_text_and_style_layers() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(
        0,
        0,
        Style::new().fg(Color::Indexed(1)),
        "ab",
        &mut clusters,
    );
    let snap = grid.styled_snapshot(&clusters);
    assert!(snap.starts_with("@text\n"));
    assert!(snap.contains("ab"));
    assert!(snap.contains("@fg\n"));
    assert!(snap.contains("Indexed(1)"));
}

#[test]
fn ascii_put_str_does_not_intern_clusters() {
    let mut grid = Grid::new(40, 2);
    let mut clusters = ClusterTable::new();
    let style = Style::new();
    for _ in 0..64 {
        grid.put_str(0, 0, style, "hello", &mut clusters);
    }
    assert!(clusters.referenced().is_empty());
}

proptest! {
    #[test]
    fn random_writes_keep_wide_pairs(
        w in 4u16..20,
        h in 2u16..8,
        ops in prop::collection::vec((0u16..15, 0u16..5, 0u16..8), 1..40),
    ) {
        let mut grid = Grid::new(w, h);
        let mut clusters = ClusterTable::new();
        let style = Style::new();
        let samples = ["a", "bc", "日", "x", " ", "👩\u{200D}💻"];
        for (col, row, pick) in ops {
            let row = row % h;
            let col = col % w;
            let text = samples[usize::from(pick) % samples.len()];
            grid.put_str(col, row, style, text, &mut clusters);
            if pick % 3 == 0 {
                grid.set_style(
                    Rect { x: col, y: row, width: 1, height: 1 },
                    Style::new().fg(Color::Indexed((pick % 15) as u8)),
                );
            }
            if pick % 5 == 0 {
                grid.fill(
                    Rect { x: col, y: row, width: 2, height: 1 },
                    style,
                    '-',
                    &mut clusters,
                );
            }
            assert_wide_invariants(&grid);
        }
    }
}

#[test]
fn put_str_cluster_wider_than_two_stays_aligned() {
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "\u{1100}\u{1100}Z", &mut clusters);
    assert_wide_invariants(&grid);
    assert_eq!(grid.get(4, 0).map(|c| c.meta().width), Some(CellWidth::One));
    assert_eq!(grid.get(0, 0).map(|c| c.meta().width), Some(CellWidth::Two));
    assert_eq!(grid.get(2, 0).map(|c| c.meta().width), Some(CellWidth::Two));
}

#[test]
fn put_str_lone_combining_mark_takes_no_column() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "\u{301}Z", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "Z   ");
}
