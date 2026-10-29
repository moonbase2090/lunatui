// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, List, Paragraph, RenderCx, VecSource, Widget};

const WIDTH: u16 = 42;
const HEIGHT: u16 = 14;
const ITEMS: [&str; 5] = ["Calendar", "Gauge", "Table", "Sparkline", "Form"];

struct Screen {
    selected: usize,
}

impl Screen {
    fn new() -> Self {
        Self { selected: 1 }
    }

    fn move_selection(&mut self, later: bool) {
        if later {
            self.selected = self
                .selected
                .saturating_add(1)
                .min(ITEMS.len().saturating_sub(1));
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    fn source() -> VecSource {
        let mut source = VecSource::new();
        for item in ITEMS {
            source.push_row(vec![String::from(item)]);
        }
        source
    }
}

impl App for Screen {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
                KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
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
        let raised = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1));
        Block::new().style(text).render(area, &mut cx);
        let list_area = Rect::new(0, 0, area.width, area.height.saturating_sub(1));
        let block = Block::bordered()
            .title(" items ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        block.render(list_area, &mut cx);
        List::new(Self::source())
            .selected(Some(self.selected))
            .style(text)
            .highlight_style(raised)
            .highlight_symbol("> ", accent)
            .render(block.inner(list_area), &mut cx);
        let hint = Rect::new(
            1,
            area.height.saturating_sub(1),
            area.width.saturating_sub(2),
            1,
        );
        Paragraph::new("Up and down move the row. q quits.")
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
