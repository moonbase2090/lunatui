// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Gauge, RenderCx, Widget};

const LAV: Color = Color::Rgb(185, 167, 240);
const TRACK: Color = Color::Rgb(22, 28, 61);

fn render(gauge: &Gauge, width: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    gauge.render(Rect::new(0, 0, width, 1), &mut cx);
    (grid, clusters)
}

#[test]
fn sub_cell_draws_the_fractional_end_in_eighths() {
    let cases = [
        (0.375, "███     "),
        (0.4, "███▏    "),
        (0.45, "███▌    "),
        (0.49, "███▉    "),
        (0.5, "████    "),
        (1.0, "████████"),
    ];
    for (ratio, want) in cases {
        let (grid, clusters) = render(&Gauge::new(ratio).sub_cell(true), 8);
        assert_eq!(grid.row_display(0, &clusters), want, "ratio {ratio}");
    }
}

#[test]
fn without_sub_cell_the_gauge_keeps_whole_cells() {
    let (grid, clusters) = render(&Gauge::new(0.45), 8);
    assert_eq!(grid.row_display(0, &clusters), "███     ");
}

#[test]
fn track_style_fills_the_unfilled_cells() {
    let gauge = Gauge::new(0.45)
        .sub_cell(true)
        .gauge_style(Style::new().fg(LAV).bg(TRACK))
        .track_style(Style::new().bg(TRACK));
    let (grid, _) = render(&gauge, 8);
    let style = |x| grid.get(x, 0).map(Cell::style).unwrap_or_default();
    assert_eq!(style(0).fg, LAV);
    assert_eq!(style(3).fg, LAV, "the partial cell uses the fill style");
    for x in 3..8 {
        assert_eq!(style(x).bg, TRACK, "x={x}");
    }
}
