// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{List, RenderCx, VecSource, Widget};

const SEL: Color = Color::Rgb(42, 35, 87);
const LAV: Color = Color::Rgb(185, 167, 240);
const GREEN: Color = Color::Rgb(155, 227, 168);
const MOON: Color = Color::Rgb(243, 226, 176);

fn source() -> VecSource {
    let mut source = VecSource::new();
    for name in ["Calendar", "Gauge", "Table"] {
        source.push_row(vec![name.to_owned()]);
    }
    source
}

fn render(list: &List<VecSource>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, &"x".repeat(usize::from(width)), Style::new());
    list.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..grid.height())
        .map(|y| grid.row_display(y, clusters))
        .collect()
}

#[test]
fn icons_marks_and_the_highlight_symbol_lay_out_each_row() {
    let list = List::new(source())
        .selected(Some(1))
        .highlight_style(
            Style::new()
                .bg(SEL)
                .with_attrs(lunatui_core::Attrs::new().set_bold(true)),
        )
        .highlight_symbol("▌ ", Style::new().fg(LAV))
        .icons(vec![
            ("◆".to_owned(), Style::new().fg(LAV)),
            ("◆".to_owned(), Style::new().fg(GREEN)),
            ("◆".to_owned(), Style::new().fg(MOON)),
        ])
        .marks(
            vec![
                Some(("✓".to_owned(), Style::new().fg(GREEN))),
                Some(("✓".to_owned(), Style::new().fg(GREEN))),
                None,
            ],
            1,
        );
    let (grid, clusters) = render(&list, 16, 4);
    assert_eq!(
        rows(&grid, &clusters),
        [
            "  ◆ Calendar  ✓ ",
            "▌ ◆ Gauge     ✓ ",
            "  ◆ Table       ",
            "                ",
        ]
    );
    let style = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default();
    assert_eq!(style(2, 0).fg, LAV);
    assert_eq!(
        style(2, 1).fg,
        GREEN,
        "icon keeps its color on the selected row"
    );
    assert_eq!(style(2, 1).bg, SEL, "and takes the band background");
    assert!(!style(2, 1).attrs.bold(), "but not the row's bold");
    assert!(!style(0, 1).attrs.bold());
    assert!(style(4, 1).attrs.bold(), "the item text is bold");
    assert_eq!(style(14, 1).fg, GREEN);
    assert_eq!(style(0, 1).fg, LAV);
    for x in 0..16 {
        assert_eq!(style(x, 1).bg, SEL, "x={x}");
    }
    assert_ne!(style(15, 0).bg, SEL);
}

#[test]
fn a_plain_list_still_draws_text_from_the_left_edge() {
    let list = List::new(source()).selected(Some(0));
    let (grid, clusters) = render(&list, 10, 3);
    assert_eq!(
        rows(&grid, &clusters),
        ["Calendar  ", "Gauge     ", "Table     "]
    );
}
