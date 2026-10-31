// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    BigText, Block, BorderType, Gauge, Heatmap, HeatmapCell, HeatmapRow, LogBuffer, LogEntry,
    LogLevel, LogView, Paragraph, PixelSize, RenderCx, Widget, WidgetState,
};

const SERVICES: [&str; 8] = [
    "edge", "auth", "search", "billing", "ingest", "queue", "workers", "storage",
];
const ZONES: [&str; 12] = [
    "AZ01", "AZ02", "AZ03", "AZ04", "AZ05", "AZ06", "AZ07", "AZ08", "AZ09", "AZ10", "AZ11", "AZ12",
];

fn reading(row: usize, column: usize, tick: u32) -> Option<u64> {
    if row == 6 && column == 2 {
        return None;
    }
    let phase = (tick / 3 + u32::try_from(row * 11 + column * 7).unwrap_or(0)) % 80;
    let wave = phase.min(80 - phase);
    if (row == 2 || row == 3) && (5..9).contains(&column) && (75..210).contains(&tick) {
        Some(84 + u64::from((tick / 6 + u32::try_from(column).unwrap_or(0)) % 16))
    } else {
        Some(12 + u64::from(wave))
    }
}

fn selected(tick: u32) -> HeatmapCell {
    let column = match tick {
        0..60 => 5,
        60..120 => 6,
        120..180 => 7,
        _ => 8,
    };
    HeatmapCell {
        row: if tick < 150 { 2 } else { 3 },
        column,
    }
}

fn label(cx: &mut RenderCx<'_>, area: Rect, text: impl Into<String>, style: Style) {
    Paragraph::new(text).style(style).render(area, cx);
}

fn panel(cx: &mut RenderCx<'_>, area: Rect, title: &str, style: Style) -> Rect {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(style)
        .border_style(style.fg(cx.token(Token::Accent)));
    block.render(area, cx);
    block.inner(area)
}

pub fn render(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ground = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0));
    let raised = ground.bg(cx.token(Token::Surface1));
    Block::new().style(ground).render(area, &mut cx);
    Block::new()
        .style(raised)
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    label(
        &mut cx,
        Rect::new(area.x, area.y, area.width, 1),
        format!(
            " capacity / production{:>width$}",
            format!("LIVE  14:02:{:02} ", tick / 30),
            width = usize::from(area.width).saturating_sub(22)
        ),
        raised.with_attrs(Attrs::new().set_bold(true)),
    );
    label(
        &mut cx,
        Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 1),
        "REGION CAPACITY     8 services / 12 zones     refresh 100 ms",
        ground,
    );
    if area.width < 100 || area.height < 36 {
        matrix(
            &mut cx,
            Rect::new(
                area.x + 1,
                area.y + 4,
                area.width.saturating_sub(2),
                area.height.saturating_sub(5),
            ),
            tick,
            false,
        );
        return;
    }
    let body_height = area.height - 13;
    let matrix_area = Rect::new(area.x + 2, area.y + 4, area.width - 34, body_height);
    let inner = panel(
        &mut cx,
        matrix_area,
        " Utilization by availability zone ",
        ground,
    );
    matrix(
        &mut cx,
        Rect::new(
            inner.x + 1,
            inner.y + 1,
            inner.width.saturating_sub(2),
            inner.height.saturating_sub(4),
        ),
        tick,
        true,
    );
    legend(
        &mut cx,
        Rect::new(
            inner.x + 1,
            inner.y + inner.height - 2,
            inner.width.saturating_sub(2),
            2,
        ),
    );
    let inspector_area = Rect::new(area.x + area.width - 30, area.y + 4, 28, body_height);
    let inner = panel(&mut cx, inspector_area, " Inspector ", ground);
    inspector(&mut cx, inner, tick, ground);
    let event_area = Rect::new(area.x + 2, area.y + area.height - 8, area.width - 4, 6);
    let inner = panel(&mut cx, event_area, " Capacity events ", ground);
    activity(&mut cx, inner, tick);
    label(
        &mut cx,
        Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        " h/l zone   j/k service                            scope: production    sample stream connected",
        raised,
    );
}

fn matrix(cx: &mut RenderCx<'_>, area: Rect, tick: u32, large: bool) {
    let samples: [[Option<u64>; 12]; 8] =
        std::array::from_fn(|row| std::array::from_fn(|column| reading(row, column, tick)));
    let rows: [HeatmapRow<'_>; 8] =
        std::array::from_fn(|row| HeatmapRow::new(SERVICES[row], &samples[row]));
    Heatmap::new(&rows)
        .column_labels(&ZONES)
        .cell_size(Size::new(
            if large { 6 } else { 4 },
            if large { 2 } else { 1 },
        ))
        .range(0..=100)
        .selected(Some(selected(tick)))
        .state(WidgetState::Focused)
        .render(area, cx);
}

fn legend(cx: &mut RenderCx<'_>, area: Rect) {
    let values = [Some(0), Some(20), Some(40), Some(60), Some(80)];
    let rows = [HeatmapRow::new("scale", &values)];
    Heatmap::new(&rows)
        .column_labels(&["0%", "20%", "40%", "60%", "80%+"])
        .cell_size(Size::new(6, 1))
        .range(0..=100)
        .render(area, cx);
}

fn inspector(cx: &mut RenderCx<'_>, area: Rect, tick: u32, ground: Style) {
    let cell = selected(tick);
    let value = reading(cell.row, cell.column, tick).unwrap_or(0);
    let token = if value >= 80 {
        Token::Fail
    } else if value >= 60 {
        Token::Warn
    } else {
        Token::Ok
    };
    let tone = cx.token(token);
    label(
        cx,
        Rect::new(area.x + 1, area.y + 1, area.width - 2, 1),
        format!("{} / {}", SERVICES[cell.row], ZONES[cell.column]),
        ground.with_attrs(Attrs::new().set_bold(true)),
    );
    BigText::new(&format!("{value:03}"))
        .pixel_size(PixelSize::Quadrant)
        .style(ground.fg(tone))
        .render(Rect::new(area.x + 2, area.y + 4, area.width - 4, 4), cx);
    label(
        cx,
        Rect::new(area.x + 2, area.y + 8, area.width - 4, 1),
        "percent utilized",
        ground,
    );
    label(
        cx,
        Rect::new(area.x + 1, area.y + 11, area.width - 2, 1),
        if value >= 80 {
            "CRITICAL / rebalancing"
        } else {
            "HEALTHY / within target"
        },
        ground.fg(tone),
    );
    Gauge::new(f32::from(u16::try_from(value).unwrap_or(0)) / 100.0)
        .gauge_style(ground.fg(tone))
        .track_style(ground.bg(cx.token(Token::Surface1)))
        .sub_cell(true)
        .render(Rect::new(area.x + 1, area.y + 13, area.width - 2, 1), cx);
    label(
        cx,
        Rect::new(area.x + 1, area.y + 15, area.width - 2, 1),
        "capacity target < 80%",
        ground,
    );
    label(
        cx,
        Rect::new(area.x + 1, area.y + 18, area.width - 2, 2),
        if tick < 210 {
            "watching shard pressure\nauto rebalance armed"
        } else {
            "hot shard moved\ncapacity recovered"
        },
        ground,
    );
}

fn activity(cx: &mut RenderCx<'_>, area: Rect, tick: u32) {
    let mut log = LogBuffer::new(6);
    for (at, level, message) in [
        (
            0,
            LogLevel::Info,
            "Capacity stream connected; all zones reporting",
        ),
        (
            60,
            LogLevel::Info,
            "Inspecting search replicas across zones",
        ),
        (
            90,
            LogLevel::Warn,
            "Search shard pressure above 80%; rebalance queued",
        ),
        (
            150,
            LogLevel::Info,
            "Rebalancing hot shards from AZ08 to AZ03",
        ),
        (
            210,
            LogLevel::Info,
            "Shard migration complete; utilization recovered",
        ),
        (
            270,
            LogLevel::Info,
            "All capacity checks stable; incident resolved",
        ),
    ] {
        if tick >= at {
            log.push(LogEntry::new(level, "", message).stamp(format!("14:02:{:02}", at / 30)));
        }
    }
    LogView::new(&log)
        .target_width(0)
        .follow(true)
        .render(area, cx);
}
