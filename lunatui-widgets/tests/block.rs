// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, GlyphSlot, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, RenderCx, Widget};

fn render_block(block: &Block, width: u16, height: u16) -> String {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    block.render(Rect::new(0, 0, width, height), &mut cx);
    grid.styled_snapshot(&clusters)
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
