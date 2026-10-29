// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;
use std::time::{Duration, Instant};

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_runtime::RenderFrame;
use lunatui_term::{Event, KeyCode};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, BorderType, ImageId, Paragraph, PieChart, PieSlice, PixelPieChart, RenderCx, Sparkline,
    Table, VecSource, Widget,
};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 32;
const REGIONS: [(&str, f64, Color); 5] = [
    ("North America", 42.0, Color::Rgb(154, 140, 248)),
    ("Europe", 27.0, Color::Rgb(111, 183, 247)),
    ("Asia Pacific", 18.0, Color::Rgb(93, 210, 185)),
    ("South America", 8.0, Color::Rgb(246, 192, 104)),
    ("Africa", 5.0, Color::Rgb(244, 139, 162)),
];

fn specification(reveal: f64, selected: Option<usize>) -> PieChart {
    PieChart::new(REGIONS.map(|(name, value, color)| PieSlice::new(name, value).color(color)))
        .donut(0.53)
        .legend(false)
        .cell_aspect(2.0)
        .reveal(reveal)
        .highlight(selected)
}

fn main() -> io::Result<()> {
    let id = ImageId::new(41).ok_or_else(|| io::Error::other("invalid chart identity"))?;
    run(
        Screen {
            pie: PixelPieChart::new(id, specification(0.0, None)),
            started: Instant::now(),
            selected: None,
            visible: true,
        },
        RunOptions::new(WIDTH, HEIGHT),
    )
}

struct Screen {
    pie: PixelPieChart,
    started: Instant,
    selected: Option<usize>,
    visible: bool,
}

impl Screen {
    fn draw(&mut self, area: Rect, cx: &mut RenderCx<'_>) {
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(cx.token(Token::Surface0));
        Block::default().style(text).render(area, cx);
        Paragraph::new("EDGE NETWORK")
            .style(accent)
            .render(Rect::new(3, 1, 60, 1), cx);
        Paragraph::new("Requests by region")
            .style(text)
            .render(Rect::new(3, 3, 60, 1), cx);
        Paragraph::new("128,640 requests / second")
            .style(accent)
            .render(Rect::new(3, 5, 60, 1), cx);
        let left = Rect::new(2, 7, 57, 22);
        let right = Rect::new(61, 7, 37, 22);
        let panel = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(text)
            .border_style(accent);
        panel.clone().title(" Traffic share ").render(left, cx);
        panel.clone().title(" Regions ").render(right, cx);
        let elapsed = self.started.elapsed().as_secs_f64();
        self.pie
            .set_chart(specification((elapsed / 1.8).min(1.0), self.selected));
        if self.visible {
            self.pie
                .render(Rect::new(left.x + 3, left.y + 1, 50, 20), cx);
        }
        let mut rows = VecSource::new();
        for (name, value, _) in REGIONS {
            rows.push_row(vec![String::from(name), format!("{value:.0}%")]);
        }
        Table::new(rows, vec![Constraint::Fill(1), Constraint::Length(7)])
            .header([("REGION", accent), ("SHARE", accent)])
            .style(text)
            .selected(self.selected)
            .highlight_style(accent)
            .render(Rect::new(right.x + 2, right.y + 2, 33, 8), cx);
        Paragraph::new("GLOBAL TRAFFIC\n128.6k req/s\n\nP95 latency   42 ms\nAvailability  99.99%")
            .style(text)
            .render(Rect::new(right.x + 2, right.y + 11, 33, 6), cx);
        Sparkline::new(&[
            22, 24, 21, 30, 26, 33, 29, 31, 38, 35, 32, 41, 37, 44, 40, 46, 42, 48, 45, 52, 49, 55,
            51, 58, 54, 60, 57, 63, 59, 66, 62, 68, 65,
        ])
        .render(Rect::new(right.x + 2, right.y + 18, 33, 2), cx);
        Paragraph::new("← → select region    Space hide / show    q quit")
            .style(text)
            .render(Rect::new(3, 30, 94, 1), cx);
    }
}

impl App for Screen {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Right => {
                    self.selected =
                        Some(self.selected.map_or(0, |index| (index + 1) % REGIONS.len()));
                }
                KeyCode::Left => {
                    self.selected = Some(self.selected.map_or(REGIONS.len() - 1, |index| {
                        (index + REGIONS.len() - 1) % REGIONS.len()
                    }));
                }
                KeyCode::Char(' ') => self.visible = !self.visible,
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
    fn needs_animation_frames(&self) -> bool {
        self.started.elapsed() < Duration::from_secs(2)
    }
}
