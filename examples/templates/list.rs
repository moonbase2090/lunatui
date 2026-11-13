// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(any(unix, windows))]
use std::env;
#[cfg(any(unix, windows))]
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, Handle, SelectionMode, is_quit};
#[cfg(any(unix, windows))]
use lunatui_runtime::{RunOptions, render_once, run};
use lunatui_term::Event;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, List, ListState, Paragraph, RenderCx, VecSource, Widget};

#[cfg(any(unix, windows))]
const WIDTH: u16 = 42;
#[cfg(any(unix, windows))]
const HEIGHT: u16 = 14;
const ITEMS: [&str; 5] = ["Calendar", "Gauge", "Table", "Sparkline", "Form"];

struct Screen {
    list: ListState,
}

pub(crate) fn app() -> impl App {
    Screen::new()
}

impl Screen {
    fn new() -> Self {
        let mut list = ListState::new(SelectionMode::Single);
        list.sync(ITEMS.len(), ITEMS.len());
        list.focus(Some(1));
        Self { list }
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
        self.list.handle(event);
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
        let rows = block.inner(list_area).height;
        self.list.sync(ITEMS.len(), usize::from(rows));
        List::new(Self::source())
            .selected(self.list.focused())
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

#[cfg(any(unix, windows))]
/// Runs the native quickstart example.
///
/// # Errors
///
/// Returns terminal setup, rendering, or output errors.
pub fn main() -> io::Result<()> {
    let mut app = app();
    if env::args().any(|arg| arg == "--snapshot") {
        let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
        io::stdout().write_all(grid.display_text(&clusters).as_bytes())?;
        return Ok(());
    }
    run(app, RunOptions::new(WIDTH, HEIGHT))
}
