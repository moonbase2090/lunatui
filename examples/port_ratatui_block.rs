// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_ratatui::RatatuiWidget;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, RenderCx, Widget};
use ratatui::widgets::Block as RatBlock;

const WIDTH: u16 = 40;
const HEIGHT: u16 = 10;

fn render_frame() -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .title("lunatui")
        .render(Rect::new(0, 0, 20, HEIGHT), &mut cx);
    RatatuiWidget::new(RatBlock::bordered().title("ratatui"))
        .render(Rect::new(20, 0, 20, HEIGHT), &mut cx);
    (grid, clusters)
}

fn main() -> io::Result<()> {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let (grid, clusters) = render_frame();
    if snapshot {
        for row in 0..HEIGHT {
            println!("{}", grid.row_display(row, &clusters));
        }
        return Ok(());
    }
    eprintln!("pipe stdout or pass --snapshot");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HEIGHT, WIDTH, render_frame};

    #[test]
    fn bordered_blocks_render() {
        let (grid, clusters) = render_frame();
        let top = grid.row_display(0, &clusters);
        assert!(top.contains('┌') || top.contains('─'));
        assert!(grid.row_display(4, &clusters).contains('│'));
        assert_eq!(grid.width(), WIDTH);
        assert_eq!(grid.height(), HEIGHT);
    }
}
