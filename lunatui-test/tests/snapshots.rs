// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Style};
use lunatui_term::ColorMode;
use lunatui_test::diff_matches;

#[test]
fn styled_snapshot_grid_matches_the_oracle() {
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
    assert!(snap.contains("Indexed(1)"));
    let blank = Grid::new(grid.width(), grid.height());
    let result = diff_matches(&blank, &grid, &clusters, ColorMode::TrueColor);
    assert_eq!(
        result.map_or_else(|err| err.to_string(), |()| String::new()),
        ""
    );
}

#[test]
fn wide_and_combining_snapshot_matches_the_oracle() {
    let mut grid = Grid::new(8, 2);
    let mut clusters = ClusterTable::new();
    let style = Style::new().bg(Color::Rgb(8, 8, 8));
    grid.put_str(0, 0, style, "日本", &mut clusters);
    grid.put_str(0, 1, style, "e\u{301}x", &mut clusters);
    let _snap = grid.styled_snapshot(&clusters);
    let blank = Grid::new(grid.width(), grid.height());
    let result = diff_matches(&blank, &grid, &clusters, ColorMode::TrueColor);
    assert_eq!(
        result.map_or_else(|err| err.to_string(), |()| String::new()),
        ""
    );
}
