// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_ratatui::RatatuiStatefulWidget;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, RenderCx, Widget};
use ratatui::widgets::{List, ListItem, ListState};

const WIDTH: u16 = 28;
const HEIGHT: u16 = 8;

fn render_frame() -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .title("port list")
        .render(Rect::new(0, 0, WIDTH, HEIGHT), &mut cx);
    let inner = Rect::new(1, 1, WIDTH - 2, HEIGHT - 2);
    let items = vec![
        ListItem::new("alpha"),
        ListItem::new("beta"),
        ListItem::new("gamma"),
    ];
    let widget = RatatuiStatefulWidget::new(
        List::new(items).highlight_symbol("> "),
        ListState::default().with_selected(Some(1)),
    );
    widget.render(inner, &mut cx);
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
    use super::{render_frame};

    #[test]
    fn list_selection_visible() {
        let (grid, clusters) = render_frame();
        let joined = (0..grid.height())
            .map(|row| grid.row_display(row, &clusters))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("beta"));
        assert!(joined.contains('>'));
    }
}
