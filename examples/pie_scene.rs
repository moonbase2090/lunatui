// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Style, width};
use lunatui_theme::Token;
use lunatui_widgets::{
    Axis, Block, BorderType, Chart, Constraint, Dataset, GraphType, Marker, Paragraph, PieChart,
    PieSlice, PixelPieChart, RenderCx, StatusBar, Table, VecSource, Widget, WidgetState,
};

#[derive(Clone, Copy)]
pub(super) struct ClusterFrame {
    pub tick: u32,
    pub selected: Option<usize>,
}

impl ClusterFrame {
    pub(super) fn at(tick: u32) -> Self {
        Self {
            tick,
            selected: (tick >= 60).then(|| usize::try_from((tick - 60) / 36).unwrap_or(0) % 5),
        }
    }
}

pub(super) enum Display<'a> {
    Hidden,
    Cells,
    Native(&'a mut PixelPieChart),
}

pub(super) fn specification(frame: &ClusterFrame) -> PieChart {
    let rates = pc_rates(frame.tick);
    PieChart::new(
        PC_REGIONS
            .iter()
            .zip(rates)
            .map(|((label, _), rate)| PieSlice::new(*label, rate)),
    )
    .marker(Marker::Braille)
    .cell_aspect(2.3)
    .reveal(pc_ease(frame.tick, 4, 44))
    .highlight(frame.selected)
    .state(WidgetState::Focused)
}

fn label(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    let columns = u16::try_from(width(text)).unwrap_or(u16::MAX);
    Paragraph::new(text)
        .style(style)
        .render(Rect::new(x, y, columns, 1), cx);
}

const PC_REGIONS: [(&str, f64); 5] = [
    ("us-east", 3420.0),
    ("eu-west", 2610.0),
    ("ap-south", 1480.0),
    ("us-west", 1190.0),
    ("sa-east", 540.0),
];

fn pc_ease(tick: u32, from: u32, span: u32) -> f64 {
    let t = f64::from(tick.saturating_sub(from).min(span)) / f64::from(span);
    1.0 - (1.0 - t).powi(3)
}

fn pc_rates(tick: u32) -> [f64; 5] {
    pc_rates_at(f64::from(tick))
}

fn pc_rates_at(time: f64) -> [f64; 5] {
    let phase = time / 18.0;
    let mut rates = [0.0; 5];
    let mut offset = 0.0;
    for (rate, (_, base)) in rates.iter_mut().zip(PC_REGIONS) {
        *rate = base * (1.0 + 0.22 * (phase + offset).sin() + 0.08 * (phase * 2.3 + offset).cos());
        offset += 1.3;
    }
    rates
}

fn pc_latency(rate: f64, total: f64) -> f64 {
    38.0 + 140.0 * rate / total
}

pub(super) fn draw(area: Rect, frame: &ClusterFrame, display: Display<'_>, cx: &mut RenderCx<'_>) {
    let tick = frame.tick;
    let theme = cx.theme();
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    let fill = Style::new().fg(text).bg(surface);
    let blank = vec![" ".repeat(usize::from(area.width)); usize::from(area.height)].join("\n");
    Paragraph::new(blank).style(fill).render(area, cx);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let head = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    label(
        cx,
        area.x,
        area.y,
        &" ".repeat(usize::from(area.width)),
        head,
    );
    label(
        cx,
        area.x + 1,
        area.y,
        "Cluster overview  ·  prod-eu-2  ·  live",
        head,
    );
    let left = Rect::new(area.x + 2, area.y + 2, 70, 32);
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        32,
    );
    for (title, pane) in [(" Requests by region ", left), (" Regions ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, cx);
    }
    let picked = frame.selected;
    let rates = pc_rates(tick);
    let pixels = matches!(&display, Display::Native(_));
    let visible = !matches!(&display, Display::Hidden);
    let pie_area = Rect::new(left.x + 3, left.y + 2, left.width - 6, 28);
    match display {
        Display::Hidden => {}
        Display::Cells => specification(frame).render(pie_area, cx),
        Display::Native(pie) => {
            pie.set_chart(specification(frame));
            pie.render(pie_area, cx);
        }
    }
    pc_table(cx, right, &rates, picked, [fill, dim]);
    if visible {
        pc_trend(cx, right, tick, [fill, dim], pixels);
    }
    StatusBar::new(" ←→ region  ●  r refresh  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            cx,
        );
}

fn pc_table(
    cx: &mut RenderCx<'_>,
    pane: Rect,
    rates: &[f64; 5],
    picked: Option<usize>,
    styles: [Style; 2],
) {
    let [fill, dim] = styles;
    let x = pane.x + 3;
    let top = pane.y + 2;
    label(cx, x, top, "region       share   req/s    p95", dim);
    let total: f64 = rates.iter().sum();
    let lit = fill
        .fg(cx.token(Token::FocusRing))
        .with_attrs(Attrs::new().set_bold(true));
    let mut rows = VecSource::new();
    for ((name, _), rate) in PC_REGIONS.iter().zip(rates) {
        let share = 100.0 * rate / total;
        let latency = pc_latency(*rate, total);
        rows.push_row(vec![
            format!("{name:<10} "),
            format!("{share:>5.1}%  "),
            format!("{rate:>6.0}  "),
            format!("{latency:>4.0} ms"),
        ]);
    }
    Table::new(
        rows,
        vec![
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(7),
        ],
    )
    .style(fill)
    .selected(picked)
    .highlight_style(lit)
    .render(Rect::new(x, top + 2, 34, 5), cx);
    label(
        cx,
        x,
        top + 8,
        &format!("total              {total:>6.0} req/s"),
        fill.with_attrs(Attrs::new().set_bold(true)),
    );
    let (name, rate) = picked.map_or(("All regions", total), |index| {
        (PC_REGIONS[index].0, rates[index])
    });
    label(
        cx,
        x,
        top + 12,
        name,
        fill.fg(cx.token(Token::Accent))
            .with_attrs(Attrs::new().set_bold(true)),
    );
    let detail = if picked.is_some() {
        format!(
            "{:.0}% of traffic  ·  p95 {:.0} ms",
            100.0 * rate / total,
            pc_latency(rate, total)
        )
    } else {
        format!("{} regions  ·  {total:.0} req/s", PC_REGIONS.len())
    };
    label(cx, x, top + 13, &detail, dim);
}

fn pc_trend(cx: &mut RenderCx<'_>, pane: Rect, tick: u32, styles: [Style; 2], pixels: bool) {
    const SECONDS: usize = 40;
    let [fill, dim] = styles;
    let mut points = [(0.0, 0.0); SECONDS];
    for (at, point) in (0_u32..).zip(points.iter_mut()) {
        let back = f64::from(u32::try_from(SECONDS).unwrap_or(0) - 1 - at);
        let total: f64 = pc_rates_at(f64::from(tick) - back * 6.0).iter().sum();
        *point = (f64::from(at), total);
    }
    let x = pane.x + 3;
    let top = pane.y + 17;
    label(cx, x, top, "total req/s", dim);
    let sets = [Dataset::new(&points)
        .name("total")
        .graph_type(GraphType::Line)
        .marker(Marker::Braille)];
    Chart::new(&sets)
        .pixels(pixels)
        .background(fill)
        .legend_position(None)
        .x_axis(
            Axis::new()
                .bounds([0.0, 39.0])
                .labels(&["-40s", "-20s", "now"]),
        )
        .y_axis(
            Axis::new()
                .bounds([8000.0, 11000.0])
                .labels(&["8k", "9.5k", "11k"]),
        )
        .render(Rect::new(x, top + 1, pane.width.saturating_sub(6), 12), cx);
}
