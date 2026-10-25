// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Block, BorderType, Constraint, Gauge, Paragraph, RenderCx, Spinner, StatusBar, Table, Tabs,
    TextWrap, VecSource, Widget,
};

use crate::scenes::ratatui::dashboard::Model;

const TAB_TITLES: [&str; 4] = ["● Overview", "○ Lints", "● Tests", "○ Deps"];

/// Paints `model` into `grid` / `clusters` at `width`×`height` (clears the grid first).
pub fn render(
    model: &Model,
    width: u16,
    height: u16,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
) {
    let theme = Theme::builtin(Variant::Dark);
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    grid.fill(
        Rect::new(0, 0, width, height),
        Style::new().bg(bg),
        ' ',
        clusters,
    );
    if width > 3 {
        for y in 0..height {
            let color = if (u32::from(y) + model.tick).is_multiple_of(12) {
                accent
            } else {
                dim
            };
            grid.put_str(1, y, Style::new().fg(color).bg(bg), "│", clusters);
        }
    }
    let mut cx = RenderCx::new(grid, clusters, &theme);
    paint_main(model, Rect::new(0, 0, width, height), &mut cx);
}

/// Allocates a fresh grid+clusters and paints one frame (snapshot/helpers).
#[must_use]
pub fn render_new(model: &Model, width: u16, height: u16) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    render(model, width, height, &mut grid, &mut clusters);
    (grid, clusters)
}

fn paint_main(model: &Model, area: Rect, cx: &mut RenderCx<'_>) {
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let text = Style::new().fg(Color::Rgb(0xc0, 0xca, 0xf5)).bg(bg);
    let accent_style = Style::new().fg(accent).bg(bg);
    let dim_style = Style::new().fg(dim).bg(bg);
    let bold = Attrs::default().set_bold(true);
    let cyan = Color::Indexed(14);
    let rail_w = 3_u16;
    if area.width <= rail_w + 10 || area.height < 5 {
        return;
    }
    let main = Rect::new(area.x + rail_w, area.y, area.width - rail_w, area.height);
    let tabs_area = Rect::new(main.x, main.y, main.width, 1);
    let status_area = Rect::new(main.x, main.y + main.height - 1, main.width, 1);
    let body = Rect::new(
        main.x,
        main.y + 1,
        main.width,
        main.height.saturating_sub(2),
    );
    Tabs::new(TAB_TITLES)
        .select(usize::try_from(model.tick / 30 % 4).unwrap_or(0))
        .style(text)
        .highlight_style(Style::new().fg(accent).bg(bg).with_attrs(bold))
        .render(tabs_area, cx);
    let left_w = (body.width * 60) / 100;
    let left = Rect::new(body.x, body.y, left_w, body.height);
    let right = Rect::new(
        body.x + left_w + 1,
        body.y,
        body.width.saturating_sub(left_w + 1),
        body.height,
    );
    paint_checks(model, left, text, bold, dim, bg, cx);
    paint_side(model, right, text, accent_style, dim_style, cx);
    StatusBar::new(format!(" scorecard • 3 failing • frame {}", model.tick))
        .style(text)
        .render(status_area, cx);
    Spinner::new(model.tick)
        .style(Style::new().fg(cyan).bg(bg))
        .render(Rect::new(status_area.x, status_area.y, 1, 1), cx);
}

fn paint_checks(
    model: &Model,
    left: Rect,
    text: Style,
    bold: Attrs,
    dim: Color,
    bg: Color,
    cx: &mut RenderCx<'_>,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Checks ")
        .style(text)
        .border_style(Style::new().fg(dim).bg(bg));
    let inner = block.inner(left);
    block.render(left, cx);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let header = vec![vec![
        "check".into(),
        "state".into(),
        "time".into(),
        "file".into(),
    ]];
    Table::new(
        VecSource::from_rows(header),
        vec![
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Fill(1),
        ],
    )
    .style(text.with_attrs(bold))
    .render(Rect::new(inner.x, inner.y, inner.width, 1), cx);
    let pass = Color::Rgb(0x9e, 0xce, 0x6a);
    let fail = Color::Rgb(0xf7, 0x76, 0x8e);
    let mut y = inner.y.saturating_add(1);
    for row in model
        .rows
        .iter()
        .skip(model.scroll)
        .take(usize::from(inner.height.saturating_sub(1)))
    {
        let state_style = if row[1] == "FAIL" {
            Style::new().fg(fail).bg(bg).with_attrs(bold)
        } else {
            Style::new().fg(pass).bg(bg)
        };
        cx.put_str(inner.x, y, &row[0], text);
        cx.put_str(inner.x + 12, y, &row[1], state_style);
        cx.put_str(inner.x + 18, y, &row[2], text);
        cx.put_str(inner.x + 25, y, &row[3], text);
        y = y.saturating_add(1);
    }
}

fn paint_side(
    model: &Model,
    right: Rect,
    text: Style,
    accent_style: Style,
    dim_style: Style,
    cx: &mut RenderCx<'_>,
) {
    if right.width == 0 || right.height <= 3 {
        return;
    }
    let log_h = right.height.saturating_sub(3);
    let log_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Log ")
        .style(text)
        .border_style(dim_style);
    let log_area = Rect::new(right.x, right.y, right.width, log_h);
    let log_inner = log_block.inner(log_area);
    log_block.render(log_area, cx);
    Paragraph::new(model.log.as_str())
        .wrap(TextWrap { trim: true })
        .scroll((u16::try_from(model.scroll).unwrap_or(u16::MAX), 0))
        .style(text)
        .render(log_inner, cx);
    let ratio = f32::from(u8::try_from(model.tick % 100).unwrap_or(0)) / 100.0;
    Gauge::new(ratio)
        .block(
            Block::bordered()
                .title(" Progress ")
                .style(text)
                .border_style(dim_style),
        )
        .gauge_style(accent_style)
        .render(Rect::new(right.x, right.y + log_h, right.width, 3), cx);
}
