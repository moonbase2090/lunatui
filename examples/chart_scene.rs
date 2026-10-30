// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Rect, Style};
use lunatui_theme::Token;
use lunatui_widgets::{
    Axis, Block, Chart, Dataset, GraphType, LegendPosition, Marker, RenderCx, Widget, WidgetState,
};

pub(super) struct ChartBoard {
    text: Color,
    surface: Color,
    edge: Color,
    tick: u32,
    pixels: bool,
    braille: bool,
}

fn chart_wave(tick: u32, index: u32, base: f64, swing: f64, speed: f64, phase: f64) -> f64 {
    let x = f64::from(index);
    let t = f64::from(tick) * speed;
    base + swing * ((x * 0.22 + t + phase).sin() * 0.7 + (x * 0.61 + t * 1.7 + phase).cos() * 0.3)
}

fn trace(tick: u32, base: f64, swing: f64, speed: f64, phase: f64) -> Vec<(f64, f64)> {
    (0..64_u32)
        .map(|index| {
            (
                f64::from(index),
                chart_wave(tick, index, base, swing, speed, phase).max(0.0),
            )
        })
        .collect()
}

impl ChartBoard {
    pub(super) fn new(cx: &RenderCx<'_>, tick: u32) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            edge: cx.token(Token::Surface1),
            tick,
            pixels: true,
            braille: false,
        }
    }

    pub(super) fn pixels(mut self, enabled: bool) -> Self {
        self.pixels = enabled;
        self
    }

    pub(super) fn braille(mut self, enabled: bool) -> Self {
        self.braille = enabled;
        self
    }

    fn panel(&self, title: &str, area: Rect, cx: &mut RenderCx<'_>) -> Rect {
        let panel = Block::bordered()
            .title(format!(" {title} "))
            .style(Style::new().fg(self.text).bg(self.surface))
            .border_style(Style::new().fg(self.edge).bg(self.surface))
            .title_style(Style::new().fg(self.text).bg(self.surface));
        panel.render(area, cx);
        let inner = panel.inner(area);
        Rect::new(
            inner.x + 1,
            inner.y,
            inner.width.saturating_sub(2),
            inner.height,
        )
    }

    fn panel_style(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    pub(super) fn throughput(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Throughput", area, cx);
        let requests = trace(self.tick, 330.0, 150.0, 0.11, 0.0);
        let p95 = trace(self.tick, 190.0, 80.0, 0.09, 2.0);
        let errors = trace(self.tick, 60.0, 40.0, 0.14, 4.0);
        let sets = [
            Dataset::new(&requests)
                .name("requests")
                .graph_type(GraphType::Line),
            Dataset::new(&p95)
                .name("p95 ms")
                .graph_type(GraphType::Line),
            Dataset::new(&errors)
                .name("errors")
                .graph_type(GraphType::Line),
        ];
        Chart::new(&sets)
            .pixels(self.pixels)
            .background(self.panel_style())
            .x_axis(
                Axis::new()
                    .title("last 64 s")
                    .bounds([0.0, 63.0])
                    .labels(&["-60s", "-30s", "now"]),
            )
            .y_axis(
                Axis::new()
                    .title("per second")
                    .bounds([0.0, 500.0])
                    .labels(&["0", "250", "500"]),
            )
            .render(plot, cx);
    }

    pub(super) fn scatter(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Latency by request size", area, cx);
        let point = |index: u32, drift: f64, spread: f64| {
            let x = f64::from(index) * 0.5 + drift;
            let y = 20.0
                + x * 3.4
                + spread * (f64::from(index) * 2.399 + f64::from(self.tick) * 0.05).sin();
            (x.rem_euclid(32.0), y)
        };
        let fast: Vec<(f64, f64)> = (0..48).map(|index| point(index, 0.0, 14.0)).collect();
        let slow: Vec<(f64, f64)> = (0..36).map(|index| point(index, 3.0, 30.0)).collect();
        let sets = [
            Dataset::new(&fast).name("cached").marker(if self.braille {
                Marker::Braille
            } else {
                Marker::Dot
            }),
            Dataset::new(&slow).name("cold").marker(if self.braille {
                Marker::Braille
            } else {
                Marker::Dot
            }),
        ];
        Chart::new(&sets)
            .pixels(self.pixels)
            .background(self.panel_style())
            .x_axis(
                Axis::new()
                    .title("KiB")
                    .bounds([0.0, 32.0])
                    .labels(&["0", "16", "32"]),
            )
            .y_axis(
                Axis::new()
                    .title("ms")
                    .bounds([0.0, 160.0])
                    .labels(&["0", "80", "160"]),
            )
            .legend_position(Some(LegendPosition::BottomRight))
            .render(plot, cx);
    }

    pub(super) fn histogram(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Requests by hour", area, cx);
        let bars: Vec<(f64, f64)> = (0..24_u32)
            .map(|hour| {
                let day = (f64::from(hour) / 24.0 * std::f64::consts::TAU - 1.9).sin() * 0.5 + 0.5;
                let beat = (f64::from(self.tick) * 0.12 + f64::from(hour) * 0.7).sin() * 0.06;
                (
                    f64::from(hour),
                    (day * 0.8 + 0.12 + beat).clamp(0.0, 1.0) * 100.0,
                )
            })
            .collect();
        let sets = [Dataset::new(&bars).graph_type(GraphType::Bar)];
        Chart::new(&sets)
            .pixels(self.pixels)
            .background(self.panel_style())
            .x_axis(
                Axis::new()
                    .bounds([0.0, 23.0])
                    .labels(&["00:00", "12:00", "23:00"]),
            )
            .y_axis(Axis::new().bounds([0.0, 100.0]).labels(&["0", "50", "100"]))
            .render(plot, cx);
    }

    pub(super) fn saturation(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Saturation", area, cx);
        let cpu = trace(self.tick, 62.0, 26.0, 0.1, 1.0);
        let memory = trace(self.tick, 44.0, 10.0, 0.06, 3.0);
        let sets = [
            Dataset::new(&cpu)
                .name("cpu %")
                .marker(if self.braille {
                    Marker::Braille
                } else {
                    Marker::HalfBlock
                })
                .graph_type(GraphType::Line),
            Dataset::new(&memory)
                .name("mem %")
                .marker(if self.braille {
                    Marker::Braille
                } else {
                    Marker::HalfBlock
                })
                .graph_type(GraphType::Line),
        ];
        Chart::new(&sets)
            .pixels(self.pixels)
            .background(self.panel_style())
            .state(WidgetState::Focused)
            .x_axis(Axis::new().bounds([0.0, 63.0]).labels(&["-60s", "now"]))
            .y_axis(Axis::new().bounds([0.0, 100.0]).labels(&["0", "50", "100"]))
            .legend_position(Some(LegendPosition::TopLeft))
            .render(plot, cx);
    }
}
