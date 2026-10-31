// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_card::{Card, Empty, Separator};
use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Widget};

fn draw(w: u16, h: u16, widget: &dyn Widget) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(w, h);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, w, h), &mut cx);
    (grid, clusters)
}

#[test]
fn card_draws_header_body_and_footer() {
    let body = Empty::new("Nothing", "yet");
    let card = Card::new("Deploys")
        .description("Last 24 hours")
        .footer("3 total")
        .body(&body);
    let (grid, clusters) = draw(30, 13, &card);
    let text = grid.display_text(&clusters);
    let rows: Vec<&str> = text.lines().collect();
    assert!(rows[0].starts_with('╭'), "{rows:?}");
    assert!(rows[2].contains("Deploys"));
    assert!(rows[3].contains("Last 24 hours"));
    assert!(text.contains("Nothing"));
    assert!(rows[10].contains("3 total"));
    assert!(rows[12].starts_with('╰'));
}

#[test]
fn card_survives_tiny_areas() {
    let card = Card::new("Title").footer("f");
    for (w, h) in [(0, 0), (1, 1), (3, 2), (8, 4)] {
        let _ = draw(w, h, &card);
    }
}

#[test]
fn separator_runs_the_full_width_or_height() {
    let (grid, clusters) = draw(5, 2, &Separator::horizontal());
    assert_eq!(grid.row_display(0, &clusters), "─────");
    let (grid, clusters) = draw(2, 3, &Separator::vertical());
    assert_eq!(grid.row_display(2, &clusters).chars().next(), Some('│'));
}

#[test]
fn empty_centers_both_lines() {
    let (grid, clusters) = draw(11, 5, &Empty::new("No data", "Try later"));
    assert_eq!(grid.row_display(1, &clusters).trim(), "No data");
    assert_eq!(grid.row_display(2, &clusters).trim(), "Try later");
    assert!(grid.row_display(1, &clusters).starts_with("  No"));
}
