// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::env;
use std::io::{self, Write};
use std::time::Instant;

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};
use lunatui_term::Event;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Gauge, RenderCx, Sparkline, Table, VecSource, Widget};

const WIDTH: u16 = 72;
const HEIGHT: u16 = 22;

struct Screen {
    started: Instant,
}

impl Screen {
    fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }

    fn tick(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis() / 200).unwrap_or(0)
    }

    fn samples(&self) -> [u64; 48] {
        const TRAFFIC: [u64; 48] = [
            14, 15, 16, 18, 22, 27, 24, 19, 17, 16, 18, 21, 29, 36, 33, 26, 22, 19, 18, 17, 16, 18,
            20, 23, 21, 19, 17, 15, 16, 19, 25, 31, 28, 23, 20, 18, 17, 16, 18, 22, 26, 24, 21, 19,
            17, 16, 15, 14,
        ];
        let shift = usize::try_from(self.tick() % 48).unwrap_or(0);
        let mut samples = [0; 48];
        for (index, sample) in samples.iter_mut().enumerate() {
            let src = (index + shift) % TRAFFIC.len();
            *sample = TRAFFIC.get(src).copied().unwrap_or(0);
        }
        samples
    }

    fn ratio(&self) -> f32 {
        let step = u8::try_from(self.tick() % 8).unwrap_or(0);
        0.25 + f32::from(step) * 0.08
    }

    fn table() -> VecSource {
        let mut source = VecSource::new();
        for (service, state, latency) in [
            ("api", "ok", "12 ms"),
            ("worker", "warn", "40 ms"),
            ("edge", "ok", "9 ms"),
        ] {
            source.push_row(vec![
                String::from(service),
                String::from(state),
                String::from(latency),
            ]);
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
        let warn = Style::new()
            .fg(cx.token(Token::Warn))
            .bg(cx.token(Token::Surface0));
        let raised = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1));
        let rows = lunatui_layout::Layout::vertical([
            Constraint::Length(8),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area);
        let Some(spark_area) = rows.segments.first().copied() else {
            return;
        };
        let Some(gauge_area) = rows.segments.get(1).copied() else {
            return;
        };
        let Some(table_area) = rows.segments.get(2).copied() else {
            return;
        };
        let spark = Block::bordered()
            .title(" requests ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        spark.render(spark_area, &mut cx);
        let samples = self.samples();
        Sparkline::new(&samples)
            .style(accent)
            .render(spark.inner(spark_area), &mut cx);
        let gauge = Block::bordered()
            .title(" capacity ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        gauge.render(gauge_area, &mut cx);
        Gauge::new(self.ratio())
            .gauge_style(accent)
            .track_style(text)
            .render(gauge.inner(gauge_area), &mut cx);
        let table = Block::bordered()
            .title(" services ")
            .style(text)
            .border_style(accent)
            .title_style(accent);
        table.render(table_area, &mut cx);
        let selected = usize::try_from(self.tick() % 3).unwrap_or(0);
        Table::new(
            Self::table(),
            vec![
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Fill(1),
            ],
        )
        .header([("service", text), ("state", text), ("latency", text)])
        .selected(Some(selected))
        .style(text)
        .highlight_style(if selected == 1 { warn } else { raised })
        .render(table.inner(table_area), &mut cx);
    }

    fn needs_animation_frames(&self) -> bool {
        true
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
