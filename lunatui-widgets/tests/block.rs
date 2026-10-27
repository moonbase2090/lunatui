// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, GlyphSlot, Grid, Rect, Style};
use lunatui_test::styled_snapshot;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Alignment, Block, BorderType, RenderCx, TitlePosition, Widget};

fn render_block(block: &Block, width: u16, height: u16) -> String {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    block.render(Rect::new(0, 0, width, height), &mut cx);
    styled_snapshot(&grid, &clusters)
}

#[test]
fn bordered_block_styled_snapshot() {
    let block = Block::bordered();
    let snap = render_block(&block, 5, 3);
    assert!(snap.contains('┌'));
    assert!(snap.contains('┐'));
    assert!(snap.contains('└'));
    assert!(snap.contains('┘'));
}

#[test]
fn long_title_is_clipped_to_title_band() {
    let block = Block::bordered().title("aaaaaaaaaa");
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(10, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    block.render(Rect::new(0, 0, 10, 3), &mut cx);
    assert_eq!(
        grid.get(9, 0).map(Cell::glyph),
        GlyphSlot::inline_scalar('┐')
    );
}

fn render_rows(block: &Block, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    block.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

#[test]
fn title_inset_keeps_border_cells_before_a_left_title() {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Logs ")
        .title_inset(1);
    let (grid, clusters) = render_rows(&block, 12, 3);
    assert_eq!(grid.row_display(0, &clusters), "╭─ Logs ───╮");
}

#[test]
fn title_inset_defaults_to_no_border_cells() {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Logs ");
    let (grid, clusters) = render_rows(&block, 12, 3);
    assert_eq!(grid.row_display(0, &clusters), "╭ Logs ────╮");
}

#[test]
fn bottom_title_sits_on_the_bottom_border_next_to_a_top_title() {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Logs ")
        .title_inset(1)
        .title_at(
            " q quit ",
            TitlePosition::Bottom,
            Alignment::Right,
            Style::new(),
        );
    let (grid, clusters) = render_rows(&block, 16, 3);
    assert_eq!(grid.row_display(0, &clusters), "╭─ Logs ───────╮");
    assert_eq!(grid.row_display(2, &clusters), "╰────── q quit ╯");
}

#[test]
fn title_at_style_overrides_the_shared_title_style() {
    let shared = Style::new().fg(Color::Rgb(185, 167, 240));
    let hint = Style::new().fg(Color::Rgb(90, 97, 120));
    let block = Block::bordered()
        .title(" Logs ")
        .title_style(shared)
        .title_at(" q ", TitlePosition::Bottom, Alignment::Right, hint);
    let (grid, _) = render_rows(&block, 12, 3);
    assert_eq!(grid.get(2, 0).map(|cell| cell.style().fg), Some(shared.fg));
    assert_eq!(grid.get(9, 2).map(|cell| cell.style().fg), Some(hint.fg));
}

#[test]
fn title_style_attributes_reach_the_title_cells() {
    let bold = Style::new()
        .fg(Color::Rgb(185, 167, 240))
        .with_attrs(Attrs::new().set_bold(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Rgb(46, 53, 96)))
        .title(" Logs ")
        .title_inset(1)
        .title_style(bold);
    let (grid, _) = render_rows(&block, 12, 3);
    let title = grid.get(3, 0).map(Cell::style).unwrap_or_default();
    assert!(title.attrs.bold());
    assert_eq!(title.fg, Color::Rgb(185, 167, 240));
    let border = grid.get(1, 0).map(Cell::style).unwrap_or_default();
    assert!(!border.attrs.bold());
    assert_eq!(border.fg, Color::Rgb(46, 53, 96));
}
