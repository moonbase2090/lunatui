// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, RenderCx, Widget};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::{Color, Style as RatStyle};
use ratatui::text::Span;
use ratatui::widgets::Block as RatBlock;

fn lunatui_display(block: &Block, width: u16, height: u16) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    block.render(Rect::new(0, 0, width, height), &mut cx);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn ratatui_lines(block: RatBlock, width: u16, height: u16) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| frame.render_widget(block, RatRect::new(0, 0, width, height)))
        .expect("draw");
    let buf: &Buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
        .collect()
}

#[test]
fn plain_bordered_block_matches_ratatui() {
    let width = 8;
    let height = 8;
    let block = Block::bordered();
    let ours = lunatui_display(&block, width, height);
    let theirs = ratatui_lines(RatBlock::bordered(), width, height);
    assert_eq!(ours, theirs);
}

#[test]
fn titled_block_matches_ratatui() {
    let width = 10;
    let height = 10;
    let block = Block::bordered()
        .title("Title")
        .title_style(lunatui_core::Style::new().fg(lunatui_core::Color::Rgb(173, 216, 230)));
    let ours = lunatui_display(&block, width, height);
    let theirs = ratatui_lines(
        RatBlock::bordered().title(Span::styled(
            "Title",
            RatStyle::default().fg(Color::LightBlue),
        )),
        width,
        height,
    );
    assert_eq!(ours, theirs);
}
