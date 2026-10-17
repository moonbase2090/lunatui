// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_ratatui::RatatuiWidget;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, RenderCx, Widget};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

const WIDTH: u16 = 36;
const HEIGHT: u16 = 6;

fn render_frame() -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .title("port paragraph")
        .render(Rect::new(0, 0, WIDTH, HEIGHT), &mut cx);
    let inner = Rect::new(1, 1, WIDTH - 2, HEIGHT - 2);
    let line = Line::from(vec![
        Span::styled("bold ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("green", Style::default().fg(Color::Green)),
    ]);
    RatatuiWidget::new(Paragraph::new(line)).render(inner, &mut cx);
    (grid, clusters)
}

fn main() {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let (grid, clusters) = render_frame();
    if snapshot {
        for row in 0..HEIGHT {
            println!("{}", grid.row_display(row, &clusters));
        }
        return;
    }
    eprintln!("pipe stdout or pass --snapshot");
}

#[cfg(test)]
mod tests {
    use super::{HEIGHT, render_frame};

    #[test]
    fn paragraph_text_present() {
        let (grid, clusters) = render_frame();
        let body = (0..grid.height())
            .map(|row| grid.row_display(row, &clusters))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(body.contains("bold"));
        assert!(body.contains("green"));
        assert_eq!(grid.height(), HEIGHT);
    }
}
