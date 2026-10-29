// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_layout::{Constraint, Layout};
use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};
use lunatui_term::Event;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, List, Paragraph, RenderCx, StatusBar, VecSource, Widget};

const WIDTH: u16 = 72;
const HEIGHT: u16 = 22;

struct Screen;

impl Screen {
    fn nav() -> VecSource {
        let mut source = VecSource::new();
        for item in ["Templates", "Gallery", "Docs"] {
            source.push_row(vec![String::from(item)]);
        }
        source
    }
}

impl App for Screen {
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
        let raised = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1));
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);
        let Some(header) = rows.segments.first().copied() else {
            return;
        };
        let Some(middle) = rows.segments.get(1).copied() else {
            return;
        };
        let Some(status) = rows.segments.get(2).copied() else {
            return;
        };
        let columns =
            Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]).split(middle);
        let Some(sidebar) = columns.segments.first().copied() else {
            return;
        };
        let Some(body) = columns.segments.get(1).copied() else {
            return;
        };
        let header_block = Block::bordered()
            .title(" header ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        header_block.render(header, &mut cx);
        Paragraph::new("Quickstart")
            .style(text)
            .render(header_block.inner(header), &mut cx);
        let side_block = Block::bordered()
            .title(" sidebar ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        side_block.render(sidebar, &mut cx);
        List::new(Self::nav())
            .selected(Some(0))
            .style(text)
            .highlight_style(raised)
            .render(side_block.inner(sidebar), &mut cx);
        let body_block = Block::bordered()
            .title(" body ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        body_block.render(body, &mut cx);
        Paragraph::new("The body fills the space beside the sidebar.")
            .style(text)
            .render(body_block.inner(body), &mut cx);
        StatusBar::new("  ready")
            .style(raised)
            .left([("templates", accent)])
            .render(status, &mut cx);
    }
}

fn main() -> io::Result<()> {
    let mut app = Screen;
    if env::args().any(|arg| arg == "--snapshot") {
        let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
        io::stdout().write_all(grid.display_text(&clusters).as_bytes())?;
        return Ok(());
    }
    run(app, RunOptions::new(WIDTH, HEIGHT))
}
