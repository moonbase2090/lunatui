// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    RenderCx, Timeline, TimelineItem, TimelineLane, TimelineStatus, TimelineTick, TimelineView,
    Widget,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect as RatRect,
    style::{Modifier, Style},
};

use super::rat_color;

const LANES: usize = 8;
const VIEW: i64 = 240;
const NOW: i64 = 100;
const TICK_LABELS: [&str; 20] = [
    "W00", "W01", "W02", "W03", "W04", "W05", "W06", "W07", "W08", "W09", "W10", "W11", "W12",
    "W13", "W14", "W15", "W16", "W17", "W18", "W19",
];

fn view_start(tick: u32) -> i64 {
    i64::from(tick % 150) * 2 - 40
}

fn plan(lane: usize, tick: u32) -> [(i64, i64, Option<u8>, TimelineStatus, bool); 3] {
    let index = i64::try_from(lane).unwrap_or(0);
    let start = index * 22;
    let length = 40 + (index % 3) * 15;
    let progress = u8::try_from((index * 13 + i64::from(tick)) % 101).unwrap_or(0);
    let second = if lane % 2 == 1 {
        TimelineStatus::Done
    } else if lane.is_multiple_of(4) {
        TimelineStatus::Planned
    } else {
        TimelineStatus::AtRisk
    };
    let milestone = if lane == 5 {
        TimelineStatus::Blocked
    } else {
        TimelineStatus::Planned
    };
    [
        (
            start,
            start + length,
            Some(progress),
            TimelineStatus::Planned,
            false,
        ),
        (
            start + length + 10,
            start + length + 40,
            None,
            second,
            false,
        ),
        (start + length + 50, 0, None, milestone, true),
    ]
}

fn item(
    (start, end, progress, status, milestone): (i64, i64, Option<u8>, TimelineStatus, bool),
) -> TimelineItem<'static> {
    let item = if milestone {
        TimelineItem::milestone(start)
    } else {
        TimelineItem::span(start, end)
    };
    let item = item.status(status);
    match progress {
        Some(percent) => item.progress(percent),
        None => item,
    }
}

pub(super) fn native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let items: [[TimelineItem<'static>; 3]; LANES] =
        std::array::from_fn(|lane| plan(lane, tick).map(item));
    let lanes: [TimelineLane<'_>; LANES] =
        std::array::from_fn(|lane| TimelineLane::new("", &items[lane]));
    let ticks: [TimelineTick<'static>; 20] = std::array::from_fn(|index| {
        TimelineTick::new(i64::try_from(index).unwrap_or(0) * 24, TICK_LABELS[index])
    });
    let start = view_start(tick);
    Timeline::new(&lanes)
        .ticks(&ticks)
        .view(TimelineView::new(start, start + VIEW))
        .now(Some(NOW))
        .render(area, &mut RenderCx::new(grid, clusters, theme));
}

pub(super) fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    let ground = Style::new()
        .fg(color(Token::TextPrimary))
        .bg(color(Token::Surface0));
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            buffer[(x, y)].set_char(' ').set_style(ground);
        }
    }
    let width = i128::from(area.width);
    let start = i128::from(view_start(tick));
    let floor = |at: i64| ((i128::from(at) - start) * width).div_euclid(i128::from(VIEW));
    let ceil = |at: i64| -((start - i128::from(at)) * width).div_euclid(i128::from(VIEW));
    let column = |value: i128| u16::try_from(value).unwrap_or(0);
    let header = area.height.min(2);
    if header == 2 {
        for x in 0..area.width {
            buffer[(area.x + x, area.y + 1)].set_char('─');
        }
    }
    let ticks: Vec<(i128, &str)> = TICK_LABELS
        .iter()
        .enumerate()
        .map(|(index, label)| (floor(i64::try_from(index).unwrap_or(0) * 24), *label))
        .filter(|(at, _)| (0..width).contains(at))
        .collect();
    for (index, &(at, label)) in ticks.iter().enumerate() {
        let next = ticks.get(index + 1).map_or(width, |&(next, _)| next - 1);
        let room = usize::try_from(next - at).unwrap_or(0);
        if header == 2 {
            buffer[(area.x + column(at), area.y + 1)].set_char('┬');
        }
        for (offset, glyph) in label.chars().take(room).enumerate() {
            let x = area.x + column(at) + u16::try_from(offset).unwrap_or(0);
            buffer[(x, area.y)].set_char(glyph);
        }
    }
    let now = floor(NOW);
    let now_visible = (0..width).contains(&now);
    if header == 2 && now_visible {
        buffer[(area.x + column(now), area.y + 1)]
            .set_char('▼')
            .set_style(ground.add_modifier(Modifier::BOLD));
    }
    for lane in 0..LANES {
        let Some(y) = u16::try_from(lane)
            .ok()
            .map(|row| area.y + header + row)
            .filter(|&y| y < area.y + area.height)
        else {
            break;
        };
        if now_visible {
            buffer[(area.x + column(now), y)].set_char('┊');
        }
        for (at, end, progress, status, milestone) in plan(lane, tick) {
            let token = match status {
                TimelineStatus::Planned => Token::Accent,
                TimelineStatus::Done => Token::Ok,
                TimelineStatus::AtRisk => Token::Warn,
                TimelineStatus::Blocked => Token::Fail,
            };
            if milestone {
                let x = floor(at);
                if (0..width).contains(&x) {
                    buffer[(area.x + column(x), y)]
                        .set_char('◆')
                        .set_style(ground.fg(color(token)));
                }
                continue;
            }
            let first = floor(at);
            let last = ceil(end).max(first + 1);
            let unstarted = if status == TimelineStatus::Planned {
                first
            } else {
                last
            };
            let split = progress.map_or(unstarted, |percent| {
                first + (last - first) * i128::from(percent) / 100
            });
            for x in first.max(0)..last.min(width) {
                let style = if x < split {
                    Style::new().fg(color(Token::Surface0)).bg(color(token))
                } else {
                    Style::new()
                        .fg(color(Token::TextPrimary))
                        .bg(color(Token::Surface1))
                };
                buffer[(area.x + column(x), y)]
                    .set_char(' ')
                    .set_style(style);
            }
        }
    }
}
