// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_layout::{Constraint, Layout};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{Block, BorderType, LayoutDebug, Paragraph, RenderCx, Widget};

const ON: u32 = 60;
const OFF: u32 = 225;
const SERVICES: [&str; 5] = ["api", "web", "worker", "billing", "search"];
const EVENTS: [&str; 8] = [
    "deploy api 2.4.0 finished",
    "worker queue drained",
    "cache hit rate 97%",
    "web p95 under 180 ms",
    "billing job started",
    "search index refreshed",
    "api autoscaled to 6",
    "worker retried 2 jobs",
];

fn ink(theme: &Theme, fg: Token, bg: Token) -> Style {
    Style::new().fg(theme.resolve(fg)).bg(theme.resolve(bg))
}

fn ramp(tick: u32, from: u16, to: u16, start: u32, span: u32) -> u16 {
    let t = tick.clamp(start, start + span) - start;
    let delta = u32::from(to - from) * t / span;
    from + u16::try_from(delta).unwrap_or(0)
}

fn sidebar_width(tick: u32) -> u16 {
    ramp(tick, 22, 34, 90, 30) - ramp(tick, 0, 12, 150, 30)
}

fn level(service: usize, tick: u32) -> u16 {
    let phase = (tick / 3 + u32::try_from(service).unwrap_or(0) * 7) % 40;
    let wave = if phase < 20 { phase } else { 40 - phase };
    u16::try_from(30 + wave * 3 + u32::try_from(service).unwrap_or(0) * 4).unwrap_or(100)
}

fn card(
    cx: &mut RenderCx<'_>,
    theme: &Theme,
    area: Rect,
    title: &str,
    value: &str,
    note: &str,
    accent: Token,
) {
    let body = ink(theme, Token::TextPrimary, Token::Surface1);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(body)
        .border_style(ink(theme, Token::Border, Token::Surface1))
        .title(format!(" {title} "))
        .title_style(ink(theme, Token::TextMuted, Token::Surface1))
        .render(area, cx);
    let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1);
    let big = ink(theme, accent, Token::Surface1).with_attrs(Attrs::new().set_bold(true));
    Paragraph::new(value).style(big).render(inner, cx);
    let line = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    Paragraph::new(note)
        .style(ink(theme, Token::TextMuted, Token::Surface1))
        .render(line, cx);
}

fn titled(cx: &mut RenderCx<'_>, theme: &Theme, area: Rect, title: &str) {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(ink(theme, Token::TextPrimary, Token::Surface1))
        .border_style(ink(theme, Token::Border, Token::Surface1))
        .title(format!(" {title} "))
        .title_style(ink(theme, Token::TextMuted, Token::Surface1))
        .render(area, cx);
}

fn header(cx: &mut RenderCx<'_>, theme: &Theme, top: Rect, tick: u32) {
    cx.fill(top, ink(theme, Token::TextStrong, Token::Surface1), ' ');
    let title = ink(theme, Token::Accent, Token::Surface1).with_attrs(Attrs::new().set_bold(true));
    cx.put_str(top.x + 1, top.y, "Operations", title);
    let clock = format!(
        "{:02}:{:02}:{:02} ",
        9 + tick / 1800,
        tick / 30 / 60 % 60,
        tick / 30 % 60
    );
    let width = u16::try_from(clock.len()).unwrap_or(0);
    let muted = ink(theme, Token::TextMuted, Token::Surface1);
    cx.put_str(top.x + top.width - width, top.y, &clock, muted);
}

fn sidebar(cx: &mut RenderCx<'_>, theme: &Theme, side: Rect, tick: u32) {
    let panel = ink(theme, Token::TextPrimary, Token::Surface1);
    let muted = ink(theme, Token::TextMuted, Token::Surface1);
    titled(cx, theme, side, "Services");
    for (index, name) in SERVICES.iter().enumerate() {
        let y = side.y + 2 + u16::try_from(index).unwrap_or(0) * 2;
        let dot = if level(index, tick) < 130 {
            Token::Ok
        } else {
            Token::Warn
        };
        cx.put_str(side.x + 2, y, "●", ink(theme, dot, Token::Surface1));
        cx.put_str(side.x + 4, y, name, panel);
        let x = side.x + side.width.saturating_sub(7);
        if x > side.x + 12 {
            cx.put_str(x, y, &format!("{:>3}%", level(index, tick).min(99)), muted);
        }
    }
    let facts = [
        ("On call", "Ana Ruiz"),
        ("Region", "us-west-2"),
        ("Uptime", "99.98%"),
        ("Deploys today", "7"),
    ];
    for (row, (label, value)) in facts.iter().enumerate() {
        let y = side.y + 13 + u16::try_from(row).unwrap_or(0) * 2;
        if y + 1 < side.y + side.height && side.width > 16 {
            cx.put_str(side.x + 2, y, label, muted);
            cx.put_str(side.x + 2, y + 1, value, panel);
        }
    }
}

fn stats(cx: &mut RenderCx<'_>, theme: &Theme, cards: Rect, tick: u32) -> Vec<Rect> {
    let split = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .spacing(1)
    .split(cards);
    let requests = 12_400 + u32::from(level(1, tick)) * 37 + tick * 3;
    let values = [
        (
            "Requests / min",
            format!("{requests}"),
            "across 5 services",
            Token::Accent,
        ),
        (
            "p95 latency",
            format!("{} ms", 120 + u32::from(level(0, tick)) / 3),
            "target 200 ms",
            Token::Ok,
        ),
        (
            "Error rate",
            format!("0.{:02}%", 4 + tick / 9 % 7),
            "budget 0.10%",
            Token::Warn,
        ),
    ];
    for (area, (title, value, note, accent)) in split.segments.iter().zip(values) {
        card(cx, theme, *area, title, &value, note, accent);
    }
    split.segments
}

fn chart(cx: &mut RenderCx<'_>, theme: &Theme, area: Rect, tick: u32) {
    titled(cx, theme, area, "Load by service");
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    let step = (inner.height / 5).clamp(1, 4);
    let muted = ink(theme, Token::TextMuted, Token::Surface1);
    let accent = ink(theme, Token::Accent, Token::Surface1);
    let track = inner.width.saturating_sub(8);
    for (index, name) in SERVICES.iter().enumerate() {
        let y = inner.y + u16::try_from(index).unwrap_or(0) * step;
        if y + step > inner.y + inner.height + 1 {
            break;
        }
        cx.put_str(inner.x, y, &format!("{name:<8}"), muted);
        let filled = usize::from(track) * usize::from(level(index, tick).min(150)) / 150;
        let bar = format!(
            "{}{}",
            "█".repeat(filled),
            "░".repeat(usize::from(track) - filled)
        );
        for row in 0..step.min(3).saturating_sub(1).max(1) {
            cx.put_str(inner.x + 8, y + row, &bar, accent);
        }
    }
}

fn events(cx: &mut RenderCx<'_>, theme: &Theme, log: Rect, tick: u32) {
    titled(cx, theme, log, "Events");
    let newest = usize::try_from(tick / 24).unwrap_or(0);
    for row in 0..usize::from(log.height.saturating_sub(2)) {
        let event = EVENTS[(newest + EVENTS.len() * 4 - row) % EVENTS.len()];
        let fade = if row == 0 {
            Token::TextPrimary
        } else {
            Token::TextMuted
        };
        let y = log.y + 1 + u16::try_from(row).unwrap_or(0);
        cx.put_str(log.x + 2, y, event, ink(theme, fade, Token::Surface1));
    }
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    grid.fill(
        area,
        ink(theme, Token::TextPrimary, Token::Surface0),
        ' ',
        clusters,
    );
    if area.width < 60 || area.height < 20 {
        return;
    }
    let mut cx = RenderCx::new(grid, clusters, theme);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);
    let (top, middle, foot) = (rows.segments[0], rows.segments[1], rows.segments[2]);
    let overlay_on = (ON..OFF).contains(&tick);

    header(&mut cx, theme, top, tick);
    let columns =
        Layout::horizontal([Constraint::Length(sidebar_width(tick)), Constraint::Fill(1)])
            .spacing(1)
            .split(middle);
    let (side, main) = (columns.segments[0], columns.segments[1]);
    sidebar(&mut cx, theme, side, tick);

    let stack = Layout::vertical([
        Constraint::Length(4),
        Constraint::Fill(1),
        Constraint::Length(7),
    ])
    .spacing(1)
    .split(main);
    let stat_boxes = stats(&mut cx, theme, stack.segments[0], tick);
    chart(&mut cx, theme, stack.segments[1], tick);
    events(&mut cx, theme, stack.segments[2], tick);

    let hint = if overlay_on {
        " Layout debug ON: every box is outlined with its size "
    } else {
        " Layout debug OFF "
    };
    cx.fill(foot, ink(theme, Token::TextMuted, Token::Surface1), ' ');
    cx.put_str(
        foot.x,
        foot.y,
        hint,
        ink(theme, Token::Accent, Token::Surface1),
    );

    LayoutDebug::new()
        .named("services", side)
        .named("requests", stat_boxes[0])
        .named("latency", stat_boxes[1])
        .named("errors", stat_boxes[2])
        .named("chart", stack.segments[1])
        .named("events", stack.segments[2])
        .enabled(overlay_on)
        .render(area, &mut cx);
}
