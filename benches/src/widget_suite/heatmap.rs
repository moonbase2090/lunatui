// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Heatmap, HeatmapCell, HeatmapRow, RenderCx, Widget};
use ratatui::{buffer::Buffer, layout::Rect as RatRect, style::Style};

use super::rat_color;

fn readings(tick: u32) -> [Option<u64>; 48] {
    std::array::from_fn(|index| {
        let step = u64::try_from(index).unwrap_or(0);
        (!step.is_multiple_of(11)).then_some((step * 17 + u64::from(tick)) % 101)
    })
}

pub(super) fn native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let values = readings(tick);
    let rows = [
        HeatmapRow::new("", &values[..12]),
        HeatmapRow::new("", &values[12..24]),
        HeatmapRow::new("", &values[24..36]),
        HeatmapRow::new("", &values[36..]),
    ];
    let selected = usize::try_from(tick / 12).unwrap_or(0) % 48;
    Heatmap::new(&rows)
        .range(0..=100)
        .selected(Some(HeatmapCell {
            row: selected / 12,
            column: selected % 12,
        }))
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
    let selected = usize::try_from(tick / 12).unwrap_or(0) % 48;
    for (index, reading) in readings(tick).iter().enumerate() {
        let row = u16::try_from(index / 12).unwrap_or(0);
        let column = u16::try_from(index % 12).unwrap_or(0) * 2;
        if row >= area.height || column >= area.width {
            continue;
        }
        let token = match reading {
            None => Token::Surface0,
            Some(0..=19) => Token::Surface1,
            Some(20..=39) => Token::Accent,
            Some(40..=59) => Token::Ok,
            Some(60..=79) => Token::Warn,
            Some(_) => Token::Fail,
        };
        let style = ground.bg(color(token));
        let x = area.x + column;
        let y = area.y + row;
        buffer[(x, y)]
            .set_style(style)
            .set_char(if index == selected {
                '◆'
            } else if reading.is_none() {
                '·'
            } else {
                ' '
            });
        if column + 1 < area.width {
            buffer[(x + 1, y)].set_style(style);
        }
    }
}
