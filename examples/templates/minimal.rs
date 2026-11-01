// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(any(unix, windows))]
use std::env;
#[cfg(any(unix, windows))]
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, is_quit};
#[cfg(any(unix, windows))]
use lunatui_runtime::{RunOptions, render_once, run};
use lunatui_term::Event;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

#[cfg(any(unix, windows))]
const WIDTH: u16 = 48;
#[cfg(any(unix, windows))]
const HEIGHT: u16 = 10;

struct Hello;

pub(crate) fn app() -> impl App {
    Hello
}

impl App for Hello {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
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
        let block = Block::bordered()
            .title(" hello ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        block.render(area, &mut cx);
        Paragraph::new("Hello. Press q to quit.")
            .style(text)
            .render(block.inner(area), &mut cx);
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
