// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, CellWidth, ClusterTable, Grid, Rect, Style};

#[test]
fn put_str_keeps_style_attributes_for_ascii_wide_and_combining_text() -> Result<(), &'static str> {
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    let style = Style::new().with_attrs(Attrs::default().set_bold(true).set_dim(true));
    grid.put_str(0, 0, style, "ab日e\u{301}", &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), "ab日e\u{301}   ");
    for x in 0..5 {
        let cell = grid.get(x, 0).ok_or("missing cell")?;
        assert!(cell.style().attrs.bold());
        assert!(cell.style().attrs.dim());
    }
    assert_eq!(
        grid.get(2, 0).ok_or("missing wide lead")?.meta().width,
        CellWidth::Two
    );
    assert_eq!(
        grid.get(3, 0).ok_or("missing wide tail")?.meta().width,
        CellWidth::Continuation
    );
    Ok(())
}

#[test]
fn fill_and_erased_wide_fragments_keep_the_supplied_attributes() -> Result<(), &'static str> {
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "日", &mut clusters);
    let style = Style::new().with_attrs(Attrs::default().set_reverse(true));
    grid.fill(Rect::new(1, 0, 1, 1), style, 'x', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters), " x ");
    assert!(
        grid.get(0, 0)
            .ok_or("missing erased lead")?
            .style()
            .attrs
            .reverse()
    );
    assert!(
        grid.get(1, 0)
            .ok_or("missing fill")?
            .style()
            .attrs
            .reverse()
    );
    grid.fill(Rect::new(0, 0, 2, 1), Style::new(), 'y', &mut clusters);
    assert!(
        !grid
            .get(0, 0)
            .ok_or("missing reset")?
            .style()
            .attrs
            .reverse()
    );
    Ok(())
}

#[test]
fn printable_ascii_put_str_keeps_supplied_attributes() -> Result<(), &'static str> {
    let mut grid = Grid::new(5, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(
        1,
        0,
        Style::new().with_attrs(Attrs::default().set_dim(true)),
        "abc",
        &mut clusters,
    );
    assert_eq!(grid.row_display(0, &clusters), " abc ");
    for x in 1..4 {
        assert!(
            grid.get(x, 0)
                .ok_or("missing ASCII cell")?
                .style()
                .attrs
                .dim()
        );
    }
    Ok(())
}

#[test]
fn plain_row_fill_keeps_and_resets_attributes() -> Result<(), &'static str> {
    let mut grid = Grid::new(5, 1);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 5, 1),
        Style::new().with_attrs(Attrs::default().set_bold(true)),
        '─',
        &mut clusters,
    );
    assert_eq!(grid.row_display(0, &clusters), "─────");
    for x in 0..5 {
        assert!(
            grid.get(x, 0)
                .ok_or("missing fill cell")?
                .style()
                .attrs
                .bold()
        );
    }
    grid.fill(Rect::new(0, 0, 5, 1), Style::new(), 'x', &mut clusters);
    assert!(
        !grid
            .get(2, 0)
            .ok_or("missing reset cell")?
            .style()
            .attrs
            .bold()
    );
    Ok(())
}
