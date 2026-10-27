// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{LineGauge, RenderCx, Widget};

const LAV: Color = Color::Rgb(185, 167, 240);
const LINE: Color = Color::Rgb(46, 53, 96);

fn render(gauge: &LineGauge, width: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    gauge.render(Rect::new(0, 0, width, 1), &mut cx);
    (grid, clusters)
}

#[test]
fn filled_cells_round_up_so_any_progress_shows() {
    let cases = [
        (0.0, "──────────"),
        (0.01, "━─────────"),
        (0.34, "━━━━──────"),
        (0.5, "━━━━━─────"),
        (1.0, "━━━━━━━━━━"),
    ];
    for (ratio, want) in cases {
        let (grid, clusters) = render(&LineGauge::new(ratio), 10);
        assert_eq!(grid.row_display(0, &clusters), want, "ratio {ratio}");
    }
}

#[test]
fn filled_and_unfilled_styles_apply_to_their_parts() {
    let gauge = LineGauge::new(0.34)
        .filled_style(Style::new().fg(LAV))
        .unfilled_style(Style::new().fg(LINE));
    let (grid, _) = render(&gauge, 10);
    let fg = |x| grid.get(x, 0).map(|cell| cell.style().fg);
    assert_eq!(fg(3), Some(LAV));
    assert_eq!(fg(4), Some(LINE));
}

#[test]
fn ratio_is_clamped() {
    let (grid, clusters) = render(&LineGauge::new(1.7), 4);
    assert_eq!(grid.row_display(0, &clusters), "━━━━");
    let (grid, clusters) = render(&LineGauge::new(-0.5), 4);
    assert_eq!(grid.row_display(0, &clusters), "────");
}
