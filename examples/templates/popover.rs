// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, Popover, RenderCx, Size, Widget};

const WIDTH: u16 = 56;
const HEIGHT: u16 = 16;

struct Screen {
    open: bool,
}

impl Screen {
    fn new() -> Self {
        Self { open: true }
    }
}

impl App for Screen {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter => self.open = !self.open,
                KeyCode::Esc => self.open = false,
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(cx.token(Token::Surface0));
        Block::new().style(text).render(area, &mut cx);
        let anchor = Rect::new(2, 1, 16, 3);
        let button = Block::bordered()
            .title(" views ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        button.render(anchor, &mut cx);
        if self.open {
            Popover::new(Paragraph::new("Open items\nAssigned to me\nClosed"), anchor)
                .title(" filters ")
                .size(Size::new(20, 5))
                .style(text)
                .border_style(accent)
                .title_style(accent)
                .render(area, &mut cx);
        }
        let hint = Rect::new(
            2,
            area.height.saturating_sub(2),
            area.width.saturating_sub(4),
            1,
        );
        Paragraph::new("Enter toggles the panel. q quits.")
            .style(text)
            .render(hint, &mut cx);
    }
}

fn main() -> io::Result<()> {
    let mut app = Screen::new();
    if env::args().any(|arg| arg == "--snapshot") {
        let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
        io::stdout().write_all(grid.display_text(&clusters).as_bytes())?;
        return Ok(());
    }
    run(app, RunOptions::new(WIDTH, HEIGHT))
}
