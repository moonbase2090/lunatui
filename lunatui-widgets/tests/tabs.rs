// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, BorderType, RenderCx, Tabs, Widget};

const LINE: Color = Color::Rgb(46, 53, 96);
const LINE_HI: Color = Color::Rgb(143, 124, 224);
const LAV: Color = Color::Rgb(185, 167, 240);
const MUTED: Color = Color::Rgb(170, 176, 204);

fn boxed_tabs() -> Tabs {
    Tabs::new(["1 Overview", "2 Logs", "3 Metrics"])
        .select(1)
        .style(Style::new().fg(MUTED))
        .highlight_style(Style::new().fg(LAV).with_attrs(Attrs::new().set_bold(true)))
        .boxed(Style::new().fg(LINE_HI), Style::new().fg(LINE))
}

fn rows(grid: &Grid, clusters: &ClusterTable, from: u16, to: u16) -> Vec<String> {
    (from..to).map(|y| grid.row_display(y, clusters)).collect()
}

#[test]
fn boxed_tab_opens_into_the_divider() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(38, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    boxed_tabs()
        .indent(1)
        .render(Rect::new(0, 0, 38, 3), &mut cx);
    assert_eq!(
        rows(&grid, &clusters, 0, 3),
        [
            "                ╭────────╮            ",
            "   1 Overview   │ 2 Logs │   3 Metrics",
            "────────────────╯        ╰────────────",
        ]
    );
}

#[test]
fn boxed_tabs_join_a_parent_border_on_the_divider_row_only() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(42, 5);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .render(Rect::new(0, 0, 42, 5), &mut cx);
    boxed_tabs()
        .indent(1)
        .join_parent_border(true)
        .render(Rect::new(0, 1, 42, 3), &mut cx);
    assert_eq!(
        rows(&grid, &clusters, 0, 5),
        [
            "╭────────────────────────────────────────╮",
            "│                ╭────────╮              │",
            "│   1 Overview   │ 2 Logs │   3 Metrics  │",
            "├────────────────╯        ╰──────────────┤",
            "╰────────────────────────────────────────╯",
        ]
    );
}

#[test]
fn boxed_tabs_style_the_box_divider_and_labels() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(38, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    boxed_tabs()
        .indent(1)
        .render(Rect::new(0, 0, 38, 3), &mut cx);
    let style_at = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default();
    assert_eq!(style_at(16, 0).fg, LINE_HI);
    assert_eq!(style_at(16, 1).fg, LINE_HI);
    assert_eq!(style_at(16, 2).fg, LINE_HI);
    assert_eq!(style_at(25, 2).fg, LINE_HI);
    assert_eq!(style_at(3, 2).fg, LINE);
    let selected = style_at(18, 1);
    assert_eq!(selected.fg, LAV);
    assert!(selected.attrs.bold());
    let other = style_at(3, 1);
    assert_eq!(other.fg, MUTED);
    assert!(!other.attrs.bold());
}

#[test]
fn boxed_tabs_clip_the_last_tab_at_the_area_edge() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    boxed_tabs().render(Rect::new(0, 0, 20, 3), &mut cx);
    assert_eq!(
        rows(&grid, &clusters, 0, 3),
        [
            "               ╭────",
            "  1 Overview   │ 2 L",
            "───────────────╯    ",
        ]
    );
}

#[test]
fn plain_tabs_keep_their_one_row_layout() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Tabs::new(["One", "Two"])
        .select(1)
        .render(Rect::new(0, 0, 20, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "One Two             ");
}

#[test]
fn boxed_tabs_clear_the_old_box_when_the_selection_moves() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(38, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    boxed_tabs()
        .indent(1)
        .render(Rect::new(0, 0, 38, 3), &mut cx);
    boxed_tabs()
        .indent(1)
        .select(0)
        .render(Rect::new(0, 0, 38, 3), &mut cx);
    assert_eq!(
        rows(&grid, &clusters, 0, 3),
        [
            " ╭────────────╮                       ",
            " │ 1 Overview │   2 Logs     3 Metrics",
            "─╯            ╰───────────────────────",
        ]
    );
}
