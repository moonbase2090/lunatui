// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

mod chart_scene;
use chart_scene::ChartBoard;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 32;
const CASES: [&str; 4] = ["Requests", "Saturation", "Latency scatter", "Hourly bars"];

fn main() -> io::Result<()> {
    run(Screen { page: 0, tick: 0 }, RunOptions::new(WIDTH, HEIGHT))
}

struct Screen {
    page: usize,
    tick: u32,
}

impl Screen {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = style.fg(cx.token(Token::Accent));
        Block::default().style(style).render(area, cx);
        Paragraph::new(format!("CHART  /  {}", CASES[self.page]))
            .style(accent)
            .render(Rect::new(3, 1, 94, 1), cx);
        Paragraph::new("← → change chart    u advance sample    q quit")
            .style(style)
            .render(Rect::new(3, 30, 94, 1), cx);
        if self.page == 3 {
            Paragraph::new("Antialiased pixels · native bars, axes and labels")
                .style(style)
                .render(Rect::new(3, 3, 94, 1), cx);
            ChartBoard::new(cx, self.tick).histogram(Rect::new(2, 5, 96, 24), cx);
        } else {
            for index in 0..2_u16 {
                let left = 2 + index * 49;
                Paragraph::new(if index == 0 { "Braille" } else { "Pixels" })
                    .style(accent)
                    .render(Rect::new(left + 1, 3, 44, 1), cx);
                let board = ChartBoard::new(cx, self.tick)
                    .pixels(index == 1)
                    .braille(true);
                let plot = Rect::new(left, 5, 47, 24);
                match self.page {
                    0 => board.throughput(plot, cx),
                    1 => board.saturation(plot, cx),
                    _ => board.scatter(plot, cx),
                }
            }
        }
    }
}

impl App for Screen {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Right => self.page = (self.page + 1) % CASES.len(),
                KeyCode::Left => self.page = (self.page + CASES.len() - 1) % CASES.len(),
                KeyCode::Char('u') => self.tick = self.tick.wrapping_add(5),
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.draw(area, &mut RenderCx::new(grid, clusters, &theme));
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let theme = Theme::builtin(Variant::Dark);
        let (grid, clusters, images) = frame.parts();
        self.draw(
            area,
            &mut RenderCx::new(grid, clusters, &theme).image_sink(images),
        );
    }
}
