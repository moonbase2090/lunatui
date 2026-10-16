// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt::Write as _;

use lunatui_core::{CellWidth, ClusterTable, Grid};

/// Serializes `grid` for styled snapshot tests: text plus run-length fg/bg/attr layers.
#[must_use]
pub fn styled_snapshot(grid: &Grid, clusters: &ClusterTable) -> String {
    let mut out = String::from("@text\n");
    out.push_str(&grid.display_text(clusters));
    out.push_str("@fg\n");
    write_color_runs(grid, &mut out, true);
    out.push_str("@bg\n");
    write_color_runs(grid, &mut out, false);
    out.push_str("@attrs\n");
    write_attr_runs(grid, &mut out);
    out
}

fn write_color_runs(grid: &Grid, out: &mut String, fg: bool) {
    for row in 0..grid.height() {
        write_color_runs_for_row(grid, out, row, fg);
    }
}

fn write_color_runs_for_row(grid: &Grid, out: &mut String, row: u16, fg: bool) {
    let mut col = 0_u16;
    while col < grid.width() {
        let Some(cell) = grid.get(col, row) else {
            break;
        };
        let span = style_span_width(grid, col, row);
        let style = cell.style();
        let color = if fg { style.fg } else { style.bg };
        let _ = writeln!(out, "{row}:{col}:{span} {color:?}");
        col += span;
    }
}

fn write_attr_runs(grid: &Grid, out: &mut String) {
    for row in 0..grid.height() {
        let mut col = 0_u16;
        while col < grid.width() {
            let Some(cell) = grid.get(col, row) else {
                break;
            };
            let span = style_span_width(grid, col, row);
            let attrs = cell.style().attrs;
            let _ = writeln!(out, "{row}:{col}:{span} {attrs:?}");
            col += span;
        }
    }
}

fn style_span_width(grid: &Grid, col: u16, row: u16) -> u16 {
    match grid.get(col, row).map(|c| c.meta().width) {
        Some(CellWidth::Two) => 2,
        _ => 1,
    }
}
