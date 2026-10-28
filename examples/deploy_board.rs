// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, Gauge, Paragraph, RenderCx, Spinner, SpinnerPreset, StatusBar, Widget,
};

struct Job {
    name: &'static str,
    phase: &'static str,
    detail: &'static str,
    preset: SpinnerPreset,
}

const CARDS: &[Job] = &[
    Job {
        name: "image",
        phase: "api",
        detail: "compiling workspace",
        preset: SpinnerPreset::DOTS,
    },
    Job {
        name: "checks",
        phase: "web",
        detail: "418 passed",
        preset: SpinnerPreset::LINE,
    },
    Job {
        name: "canary",
        phase: "edge",
        detail: "10 percent of traffic",
        preset: SpinnerPreset::MOON,
    },
];

const TASKS: &[Job] = &[
    Job {
        name: "worker",
        phase: "package",
        detail: "layer 4 of 8",
        preset: SpinnerPreset::ARC,
    },
    Job {
        name: "migrate",
        phase: "schema",
        detail: "revision 48",
        preset: SpinnerPreset::BOUNCE,
    },
    Job {
        name: "cdn",
        phase: "warm",
        detail: "62 regions",
        preset: SpinnerPreset::PULSE,
    },
    Job {
        name: "proxy",
        phase: "reload",
        detail: "draining connections",
        preset: SpinnerPreset::RING,
    },
    Job {
        name: "batch",
        phase: "windows",
        detail: "job 19",
        preset: SpinnerPreset::BARS,
    },
    Job {
        name: "docs",
        phase: "publish",
        detail: "site sync",
        preset: SpinnerPreset::QUADRANT,
    },
    Job {
        name: "monitor",
        phase: "probe",
        detail: "3 regions left",
        preset: SpinnerPreset::HALF_MOON,
    },
];

const EVENTS: &[&str] = &[
    "14:01  build queued",
    "14:02  image compiling",
    "14:04  checks running",
    "14:06  418 passed",
    "14:07  canary at 10 percent",
    "14:08  worker layer 4",
    "14:09  schema revision 48",
    "14:11  cdn warming",
    "14:12  proxy draining",
    "14:13  batch window 19",
    "14:14  site sync",
    "14:15  probes finishing",
];

const LANE: u16 = 6;
const STATUS: &str = "us-west-3    release 2.4.1    rolling    no incidents";

struct Ink {
    text: Style,
    muted: Style,
    accent: Style,
    surface: Style,
    card: Style,
    card_text: Style,
    card_muted: Style,
    card_accent: Style,
    border: Style,
    title: Style,
    gauge: Style,
    status: Style,
}

struct Regions {
    header: Rect,
    cards: Rect,
    tasks: Rect,
    events: Rect,
    gauge: Rect,
    status: Rect,
}

pub(super) fn frame_index(preset: SpinnerPreset, elapsed_ms: u64) -> u32 {
    let interval = u64::try_from(preset.interval().as_millis()).unwrap_or(u64::MAX);
    let interval = interval.max(1);
    u32::try_from(elapsed_ms / interval).unwrap_or(u32::MAX)
}

pub(super) fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, elapsed_ms: u64) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let focus = theme.resolve(Token::FocusRing);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ink = Ink {
        text: Style::new().fg(text).bg(surface),
        muted: Style::new().fg(focus).bg(surface),
        accent: Style::new().fg(accent).bg(surface),
        surface: Style::new().fg(text).bg(surface),
        card: Style::new().fg(text).bg(raised),
        card_text: Style::new().fg(text).bg(raised),
        card_muted: Style::new().fg(focus).bg(raised),
        card_accent: Style::new().fg(accent).bg(raised),
        border: Style::new().fg(focus).bg(surface),
        title: Style::new().fg(accent).bg(surface),
        gauge: Style::new().fg(accent).bg(surface),
        status: Style::new().fg(text).bg(raised),
    };
    let frame = Block::bordered()
        .title(" production deploy ")
        .style(ink.surface)
        .border_style(ink.border)
        .title_style(ink.title);
    frame.render(area, &mut cx);
    let inner = frame.inner(area);
    let regions = regions(inner);
    paint_header(&mut cx, regions.header, &ink);
    paint_cards(&mut cx, regions.cards, elapsed_ms, &ink);
    paint_tasks(&mut cx, regions.tasks, elapsed_ms, &ink);
    paint_events(&mut cx, regions.events, elapsed_ms, &ink);
    paint_gauge(&mut cx, regions.gauge, elapsed_ms, &ink);
    paint_status(&mut cx, regions.status, &ink);
}

fn regions(inner: Rect) -> Regions {
    let none = Rect::new(inner.x, inner.y, 0, 0);
    if inner.width == 0 || inner.height == 0 {
        return Regions {
            header: none,
            cards: none,
            tasks: none,
            events: none,
            gauge: none,
            status: none,
        };
    }
    let status_h = u16::from(inner.height >= 4);
    let gauge_h = if inner.height >= 10 {
        3
    } else {
        u16::from(inner.height >= 6)
    };
    let header_h = 1.min(inner.height);
    let used = status_h.saturating_add(gauge_h).saturating_add(header_h);
    let rest = inner.height.saturating_sub(used);
    let card_h = if rest >= 8 && inner.width >= 24 { 4 } else { 0 };
    let split_h = rest.saturating_sub(card_h);
    let mut y = inner.y;
    let header = Rect::new(inner.x, y, inner.width, header_h);
    y = y.saturating_add(header_h);
    let cards = Rect::new(inner.x, y, inner.width, card_h);
    y = y.saturating_add(card_h);
    let split = Rect::new(inner.x, y, inner.width, split_h);
    y = y.saturating_add(split_h);
    let gauge = Rect::new(inner.x, y, inner.width, gauge_h);
    y = y.saturating_add(gauge_h);
    let status = Rect::new(inner.x, y, inner.width, status_h);
    let (tasks, events) = if split.width >= 72 && split.height >= 5 {
        let tasks_w = split.width.saturating_mul(3) / 5;
        let tasks_w = tasks_w.max(24).min(split.width);
        (
            Rect::new(split.x, split.y, tasks_w, split.height),
            Rect::new(
                split.x.saturating_add(tasks_w),
                split.y,
                split.width.saturating_sub(tasks_w),
                split.height,
            ),
        )
    } else {
        (split, none)
    };
    Regions {
        header,
        cards,
        tasks,
        events,
        gauge,
        status,
    }
}

fn paint_header(cx: &mut RenderCx<'_>, area: Rect, ink: &Ink) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    Paragraph::new("release 2.4.1    us-west-3    rolling")
        .style(ink.text)
        .render(area, cx);
}

fn paint_cards(cx: &mut RenderCx<'_>, area: Rect, elapsed_ms: u64, ink: &Ink) {
    if area.height < 3 || area.width < 24 {
        return;
    }
    let count = u16::try_from(CARDS.len()).unwrap_or(0);
    if count == 0 {
        return;
    }
    let slot = area.width / count;
    if slot < 18 {
        paint_card(cx, &CARDS[0], area, elapsed_ms, ink);
        return;
    }
    let mut x = area.x;
    for (index, job) in CARDS.iter().enumerate() {
        let width = if index + 1 == CARDS.len() {
            area.x.saturating_add(area.width).saturating_sub(x)
        } else {
            slot
        };
        paint_card(
            cx,
            job,
            Rect::new(x, area.y, width, area.height),
            elapsed_ms,
            ink,
        );
        x = x.saturating_add(width);
    }
}

fn paint_card(cx: &mut RenderCx<'_>, job: &Job, area: Rect, elapsed_ms: u64, ink: &Ink) {
    if area.width < 4 || area.height < 3 {
        return;
    }
    let block = Block::bordered()
        .title(format!(" {} ", job.name))
        .style(ink.card)
        .border_style(ink.border.bg(ink.card.bg))
        .title_style(ink.title.bg(ink.card.bg));
    block.render(area, cx);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    Paragraph::new(job.phase)
        .style(ink.card_muted)
        .render(Rect::new(inner.x, inner.y, inner.width, 1), cx);
    let row = inner.y.saturating_add(1);
    if row >= inner.y.saturating_add(inner.height) {
        return;
    }
    let lane = LANE.min(inner.width);
    paint_spinner(
        cx,
        job.preset,
        elapsed_ms,
        inner.x,
        row,
        lane,
        ink.card_accent,
    );
    let text_x = inner.x.saturating_add(lane).saturating_add(1);
    let right = inner.x.saturating_add(inner.width);
    if text_x >= right {
        return;
    }
    Paragraph::new(job.detail)
        .style(ink.card_text)
        .render(Rect::new(text_x, row, right - text_x, 1), cx);
}

fn paint_tasks(cx: &mut RenderCx<'_>, area: Rect, elapsed_ms: u64, ink: &Ink) {
    if area.width < 12 || area.height < 3 {
        return;
    }
    let block = Block::bordered()
        .title(" tasks ")
        .style(ink.surface)
        .border_style(ink.border)
        .title_style(ink.title);
    block.render(area, cx);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let lane = LANE.min(inner.width.saturating_sub(1));
    let shown = TASKS.len().min(usize::from(inner.height));
    for (index, job) in TASKS.iter().enumerate() {
        let Some(row) = place(index, shown, inner.height) else {
            break;
        };
        let y = inner.y.saturating_add(row);
        paint_spinner(cx, job.preset, elapsed_ms, inner.x, y, lane, ink.accent);
        let text_x = inner.x.saturating_add(lane).saturating_add(1);
        let right = inner.x.saturating_add(inner.width);
        if text_x >= right {
            continue;
        }
        let label = format!("{:<8}{:<10}{}", job.name, job.phase, job.detail);
        Paragraph::new(label)
            .style(ink.text)
            .render(Rect::new(text_x, y, right - text_x, 1), cx);
    }
}

fn paint_events(cx: &mut RenderCx<'_>, area: Rect, elapsed_ms: u64, ink: &Ink) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    let block = Block::bordered()
        .title(" events ")
        .style(ink.surface)
        .border_style(ink.border)
        .title_style(ink.muted);
    block.render(area, cx);
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let count = EVENTS.len();
    let start = usize::try_from(elapsed_ms / 500).unwrap_or(0) % count;
    let shown = count.min(usize::from(inner.height));
    for slot in 0..shown {
        let Some(row) = place(slot, shown, inner.height) else {
            break;
        };
        let index = start.saturating_add(slot) % count;
        let y = inner.y.saturating_add(row);
        Paragraph::new(EVENTS[index])
            .style(ink.muted)
            .render(Rect::new(inner.x, y, inner.width, 1), cx);
    }
}

fn paint_gauge(cx: &mut RenderCx<'_>, area: Rect, elapsed_ms: u64, ink: &Ink) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let step = u16::try_from((elapsed_ms / 100) % 80).unwrap_or(0);
    let ratio = f32::from(step) / 80.0;
    let gauge = if area.height >= 3 {
        Gauge::new(ratio)
            .block(
                Block::bordered()
                    .title(" rollout ")
                    .style(ink.surface)
                    .border_style(ink.border)
                    .title_style(ink.title),
            )
            .gauge_style(ink.gauge)
    } else {
        Gauge::new(ratio).gauge_style(ink.gauge)
    };
    gauge.render(area, cx);
}

fn paint_status(cx: &mut RenderCx<'_>, area: Rect, ink: &Ink) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    Block::new().style(ink.status).render(area, cx);
    StatusBar::new(STATUS).style(ink.status).render(area, cx);
}

fn place(index: usize, shown: usize, height: u16) -> Option<u16> {
    let rows = usize::from(height);
    if shown == 0 || rows == 0 || index >= shown || index >= rows {
        return None;
    }
    let row = if shown >= rows {
        index
    } else if shown == 1 {
        0
    } else {
        index * (rows - 1) / (shown - 1)
    };
    u16::try_from(row).ok().filter(|row| *row < height)
}

fn paint_spinner(
    cx: &mut RenderCx<'_>,
    preset: SpinnerPreset,
    elapsed_ms: u64,
    x: u16,
    y: u16,
    lane: u16,
    style: Style,
) {
    if lane == 0 {
        return;
    }
    let spinner = Spinner::new(frame_index(preset, elapsed_ms))
        .preset(preset)
        .style(style);
    let columns = spinner.measure(Size::new(lane, 1)).width.min(lane);
    if columns == 0 {
        return;
    }
    spinner.render(Rect::new(x, y, columns, 1), cx);
}
