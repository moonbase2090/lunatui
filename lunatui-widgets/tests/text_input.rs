// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, BorderType, RenderCx, TextInput, Widget};

const BG: Color = Color::Rgb(10, 13, 28);
const FIELD: Color = Color::Rgb(22, 28, 61);
const INK: Color = Color::Rgb(236, 231, 218);
const LAV: Color = Color::Rgb(185, 167, 240);
const LINE_HI: Color = Color::Rgb(143, 124, 224);

fn field_block() -> Block {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(Style::new().bg(BG))
        .border_style(Style::new().fg(LINE_HI).bg(BG))
}

fn render(input: &TextInput, width: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    input.render(Rect::new(0, 0, width, 3), &mut cx);
    (grid, clusters)
}

#[test]
fn field_style_fills_the_row_inside_the_border_only() {
    let input = TextInput::new("moon", 4)
        .block(field_block())
        .padding(2)
        .field_style(Style::new().bg(FIELD))
        .style(Style::new().fg(INK).bg(FIELD))
        .cursor_glyph(" ")
        .cursor_style(Style::new().fg(BG).bg(LAV));
    let (grid, clusters) = render(&input, 14);
    assert_eq!(grid.row_display(0, &clusters), "╭────────────╮");
    assert_eq!(grid.row_display(1, &clusters), "│  moon      │");
    assert_eq!(grid.row_display(2, &clusters), "╰────────────╯");
    let bg_at = |x, y| grid.get(x, y).map(|cell| cell.style().bg);
    assert_eq!(bg_at(0, 1), Some(BG));
    assert_eq!(bg_at(13, 1), Some(BG));
    assert_eq!(bg_at(5, 0), Some(BG));
    for x in [1, 2, 3, 12] {
        assert_eq!(bg_at(x, 1), Some(FIELD), "x={x}");
    }
    assert_eq!(bg_at(7, 1), Some(LAV));
    assert_eq!(bg_at(8, 1), Some(FIELD));
}

#[test]
fn padding_moves_the_text_and_cursor_in_from_the_edge() {
    let input = TextInput::new("ab", 1).padding(3);
    let (grid, clusters) = render(&input, 10);
    assert_eq!(grid.row_display(1, &clusters), "   a│     ");
}

#[test]
fn hidden_cursor_leaves_the_field_text_untouched() {
    let input = TextInput::new("dash", 4)
        .block(field_block())
        .padding(2)
        .field_style(Style::new().bg(FIELD))
        .show_cursor(false);
    let (grid, clusters) = render(&input, 12);
    assert_eq!(grid.row_display(1, &clusters), "│  dash    │");
    assert_eq!(grid.get(7, 1).map(|cell| cell.style().bg), Some(FIELD));
}

#[test]
fn field_style_is_off_by_default() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    cx.put_str(0, 0, "xxxxxxxx", Style::new());
    TextInput::new("ab", 2).render(Rect::new(0, 0, 8, 1), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "ab│xxxxx");
}
