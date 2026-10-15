// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_text::wrap::Wrap;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Paragraph, RenderCx, Widget};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::widgets::{Paragraph as RatParagraph, Widget as RatWidget, Wrap as RatWrap};

fn lunatui_lines(paragraph: &Paragraph, width: u16, height: u16) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    paragraph.render(Rect::new(0, 0, width, height), &mut cx);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn ratatui_lines(text: &str, width: u16, height: u16) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let widget = RatParagraph::new(text).wrap(RatWrap { trim: true });
    terminal
        .draw(|frame| frame.render_widget(widget, RatRect::new(0, 0, width, height)))
        .expect("draw");
    let buf: &Buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
        .collect()
}

#[test]
fn wrapped_log_line_matches_ratatui() {
    let text = "[0001] lint warning in src/lib.rs line 1 consider removing the borrow here\n";
    let width = 40;
    let height = 4;
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
    let ours = lunatui_lines(&paragraph, width, height);
    let theirs = ratatui_lines(text, width, height);
    assert_eq!(ours, theirs);
}
