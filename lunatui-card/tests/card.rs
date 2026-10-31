// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_card::{Card, Empty, Separator};
use lunatui_core::{ClusterTable, Grid, Rect, Size};
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
fn card_in_tiny_areas_draws_only_what_fits() {
    let card = Card::new("Title").footer("f");
    for (w, h) in [(0, 0), (1, 1), (3, 2), (8, 4)] {
        let (grid, clusters) = draw(w, h, &card);
        let text = grid.display_text(&clusters);
        assert!(
            text.lines()
                .all(|row| row.chars().count() <= usize::from(w))
        );
        assert!(!text.contains("Title") || w >= 5 + 6);
    }
    let (grid, clusters) = draw(12, 6, &card);
    assert!(grid.display_text(&clusters).contains("Title"));
}

#[test]
fn measure_never_exceeds_the_available_size() {
    let long = "x".repeat(70_000);
    let card = Card::new(&long).description(&long).footer(&long);
    let avail = Size::new(80, 24);
    assert_eq!(card.measure(avail), Size::new(80, 7));
    for sep in [Separator::horizontal(), Separator::vertical()] {
        for avail in [Size::new(5, 0), Size::new(0, 5), Size::new(5, 5)] {
            let size = sep.measure(avail);
            assert!(size.width <= avail.width && size.height <= avail.height);
        }
    }
    let empty = Empty::new(&long, &long).measure(avail);
    assert!(empty.width <= avail.width && empty.height <= avail.height);
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
