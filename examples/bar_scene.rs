// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Rect, Style};
use lunatui_theme::Token;
use lunatui_widgets::{
    Bar, BarChart, BarDirection, BarGroup, Block, Paragraph, RenderCx, Widget, WidgetState,
};

fn tri(t: u32, period: u32, low: u64, high: u64) -> u64 {
    let period = period.max(2);
    let phase = t % period;
    let half = period / 2;
    let up = if phase <= half { phase } else { period - phase };
    let span = high.saturating_sub(low);
    low.saturating_add(span.saturating_mul(u64::from(up)) / u64::from(half.max(1)))
}

const BAR_SERVICES: [&str; 4] = ["api", "web", "jobs", "edge"];
const BAR_PERCENTILES: [&str; 3] = ["p50", "p95", "p99"];
const BAR_REGIONS: [(&str, u32); 6] = [
    ("us-east", 41),
    ("us-west", 57),
    ("eu-west", 73),
    ("eu-north", 88),
    ("ap-south", 119),
    ("ap-east", 96),
];

fn bar_requests(tick: u32, service: u32, tier: u32) -> u64 {
    let base = [180, 120, 70][usize::try_from(tier).unwrap_or(0)] + service * 14;
    let swing = tri(tick + service * 9 + tier * 5, 48, 0, 60);
    u64::from(base) + swing
}

fn bar_latency(tick: u32, region: u32, base: u32) -> u64 {
    u64::from(base) + tri(tick + region * 7, 60, 0, 34)
}

pub(super) struct BarBoard {
    pub(super) text: Color,
    pub(super) surface: Color,
    edge: Color,
    ok: Color,
    accent: Color,
    warn: Color,
    focus: Color,
    tick: u32,
    pixels: bool,
}

impl BarBoard {
    pub(super) fn new(cx: &RenderCx<'_>, tick: u32) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            edge: cx.token(Token::Surface1),
            ok: cx.token(Token::Ok),
            accent: cx.token(Token::Accent),
            warn: cx.token(Token::Warn),
            focus: cx.token(Token::FocusRing),
            tick,
            pixels: true,
        }
    }

    pub(super) fn pixels(mut self, enabled: bool) -> Self {
        self.pixels = enabled;
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
            inner.y + 1,
            inner.width.saturating_sub(2),
            inner.height.saturating_sub(1),
        )
    }

    fn label_style(&self) -> Style {
        Style::new().fg(self.text)
    }

    fn background_style(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    pub(super) fn requests(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Requests per minute", area, cx);
        let tiers = [self.ok, self.accent, self.warn].map(|color| Style::new().fg(color));
        let texts: Vec<Vec<String>> = (0..4_u32)
            .map(|service| {
                (0..3_u32)
                    .map(|tier| bar_requests(self.tick, service, tier).to_string())
                    .collect()
            })
            .collect();
        let bars: Vec<Vec<Bar<'_>>> = (0..4_u32)
            .map(|service| {
                (0..3_u32)
                    .map(|tier| {
                        let value = bar_requests(self.tick, service, tier);
                        let (service, tier) = (
                            usize::try_from(service).unwrap_or(0),
                            usize::try_from(tier).unwrap_or(0),
                        );
                        Bar::new(value)
                            .label(BAR_PERCENTILES[tier])
                            .text_value(&texts[service][tier])
                            .style(tiers[tier])
                    })
                    .collect()
            })
            .collect();
        let groups: Vec<BarGroup<'_>> = bars
            .iter()
            .zip(BAR_SERVICES)
            .map(|(group, name)| BarGroup::new(group).label(name))
            .collect();
        BarChart::from_groups(&groups)
            .background(self.background_style())
            .pixels(self.pixels)
            .bar_width(4)
            .bar_gap(1)
            .group_gap(3)
            .max(300)
            .label_style(self.label_style())
            .render(plot, cx);
    }

    pub(super) fn latency(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Latency by region", area, cx);
        let values: Vec<u64> = BAR_REGIONS
            .iter()
            .zip(0_u32..)
            .map(|((_, base), region)| bar_latency(self.tick, region, *base))
            .collect();
        let texts: Vec<String> = values.iter().map(|value| format!("{value} ms")).collect();
        let bars: Vec<Bar<'_>> = BAR_REGIONS
            .iter()
            .zip(&values)
            .zip(&texts)
            .map(|(((name, _), value), text)| {
                let color = if *value > 120 { self.warn } else { self.accent };
                Bar::new(*value)
                    .label(name)
                    .text_value(text)
                    .style(Style::new().fg(color))
            })
            .collect();
        BarChart::from_bars(&bars)
            .background(self.background_style())
            .pixels(self.pixels)
            .direction(BarDirection::Horizontal)
            .bar_width(2)
            .bar_gap(1)
            .max(160)
            .label_style(self.label_style())
            .render(plot, cx);
    }

    pub(super) fn runs(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Build time, last runs", area, cx);
        let columns = usize::from(plot.width / 3).min(24);
        let bars: Vec<Bar<'_>> = (0..columns)
            .map(|index| {
                let run = u32::try_from(index).unwrap_or(0);
                let value = 40 + tri(run * 5 + self.tick / 2, 37, 0, 50) + u64::from(run % 4) * 6;
                let color = if index + 1 == columns {
                    self.focus
                } else {
                    self.accent
                };
                Bar::new(value).style(Style::new().fg(color))
            })
            .collect();
        BarChart::from_bars(&bars)
            .background(self.background_style())
            .pixels(self.pixels)
            .bar_width(2)
            .bar_gap(1)
            .max(130)
            .render(plot, cx);
    }

    pub(super) fn states(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Queue depth by state", area, cx);
        let cell_w = plot.width / 3;
        let depth: Vec<Bar<'_>> = (0..8_u32)
            .map(|index| Bar::new(6 + tri(self.tick + index * 6, 40, 0, 26)))
            .collect();
        let states = [
            ("normal", WidgetState::Normal),
            ("focused", WidgetState::Focused),
            ("disabled", WidgetState::Disabled),
        ];
        for (index, (name, state)) in (0_u16..).zip(states) {
            let x = plot.x + index * cell_w;
            let chart = Rect::new(
                x,
                plot.y,
                cell_w.saturating_sub(1),
                plot.height.saturating_sub(2),
            );
            BarChart::from_bars(&depth)
                .background(self.background_style())
                .pixels(self.pixels)
                .state(state)
                .max(32)
                .render(chart, cx);
            Paragraph::new(name)
                .style(Style::new().fg(self.text).bg(self.surface))
                .render(
                    Rect::new(x, plot.y + plot.height - 1, cell_w.saturating_sub(1), 1),
                    cx,
                );
        }
    }
}
