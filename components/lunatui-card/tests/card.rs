// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Card rendering checks.

use lunatui_card::Card;
use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_test::styled_snapshot;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Paragraph, RenderCx, Widget};

fn render(card: &Card<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    card.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

#[test]
fn header_and_corners_render() {
    let card = Card::new().title("Billing").description("Plan");
    let (grid, clusters) = render(&card, 20, 5);
    let snap = styled_snapshot(&grid, &clusters);
    assert!(snap.contains('╭'));
    assert!(snap.contains('╯'));
    assert!(snap.contains("Billing"));
    assert!(snap.contains("Plan"));
}

#[test]
fn body_renders_below_header_gap() {
    let body = Paragraph::new("hello");
    let card = Card::new().title("T").body(&body);
    let (grid, clusters) = render(&card, 20, 6);
    let snap = styled_snapshot(&grid, &clusters);
    let lines: Vec<&str> = snap.lines().collect();
    let title_row = lines.iter().position(|l| l.contains('T')).unwrap_or(0);
    let body_row = lines.iter().position(|l| l.contains("hello")).unwrap_or(0);
    assert_eq!(body_row, title_row + 2);
}

#[test]
fn measure_counts_chrome_and_header() {
    let card = Card::new()
        .title("Billing")
        .description("Plan and invoices");
    assert_eq!(card.measure(Size::new(40, 10)), Size::new(21, 4));
    assert_eq!(card.measure(Size::new(10, 3)), Size::new(10, 3));
}

#[test]
fn zero_area_is_a_no_op() {
    let card = Card::new().title("x");
    let (grid, _) = render(&card, 0, 0);
    assert_eq!(grid.width(), 0);
}
