// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{
    Attrs, ClusterTable, Color, Grid, Rect, Style, clip_display_prefix, width as display_width,
};
use lunatui_layout::{Constraint, Layout};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, BorderType, Paragraph, RenderCx, TextWrap, Widget};

use crate::scenes::ratatui::dashboard::Model;

const TAB_TITLES: [&str; 4] = ["● Overview", "○ Lints", "● Tests", "○ Deps"];
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const ACCENT: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const DIM: Color = Color::Rgb(0x56, 0x5f, 0x89);
const BG: Color = Color::Rgb(0x1a, 0x1b, 0x26);
const RAIL_WIDTH: u16 = 3;
const COLUMN_WIDTHS: [u16; 3] = [12, 6, 7];

fn base() -> Style {
    Style::new().bg(BG)
}

fn bold() -> Attrs {
    Attrs::default().set_bold(true)
}

fn put(cx: &mut RenderCx<'_>, x: u16, y: u16, limit: u16, text: &str, style: Style) -> u16 {
    if x >= limit {
        return x;
    }
    let draw = clip_display_prefix(text, limit - x);
    let drawn = u16::try_from(display_width(draw)).unwrap_or(0);
    if drawn == 0 {
        return x;
    }
    cx.put_str(x, y, draw, style);
    if style.attrs != Attrs::default() {
        cx.set_style(Rect::new(x, y, drawn, 1), style);
    }
    x + drawn
}

/// Paints `model` into `grid` / `clusters` at `width`×`height` (clears the grid first).
pub fn render(
    model: &Model,
    width: u16,
    height: u16,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
) {
    let theme = Theme::builtin(Variant::Dark);
    grid.fill(Rect::new(0, 0, width, height), base(), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, &theme);
    if width > RAIL_WIDTH {
        for y in 0..height {
            let color = if (u32::from(y) + model.tick).is_multiple_of(12) {
                ACCENT
            } else {
                DIM
            };
            cx.put_str(1, y, "│", base().fg(color));
        }
    }
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
    if area.width <= RAIL_WIDTH || area.height < 3 {
        return;
    }
    let main = Rect::new(
        area.x + RAIL_WIDTH,
        area.y,
        area.width - RAIL_WIDTH,
        area.height,
    );
    let status_area = Rect::new(main.x, main.y + main.height - 1, main.width, 1);
    let body = Rect::new(main.x, main.y + 1, main.width, main.height - 2);
    paint_tabs(model, Rect::new(main.x, main.y, main.width, 1), cx);
    let left_w = body.width * 3 / 5;
    let left = Rect::new(body.x, body.y, left_w, body.height);
    let right = Rect::new(
        body.x + left_w + 1,
        body.y,
        body.width.saturating_sub(left_w + 1),
        body.height,
    );
    paint_checks(model, left, cx);
    paint_side(model, right, cx);
    paint_status(model, status_area, cx);
}

fn paint_tabs(model: &Model, area: Rect, cx: &mut RenderCx<'_>) {
    let selected = usize::try_from(model.tick / 30 % 4).unwrap_or(0);
    let limit = area.x + area.width;
    let mut x = area.x;
    for (index, title) in TAB_TITLES.iter().enumerate() {
        x = put(cx, x, area.y, limit, " ", base());
        let (bullet_style, rest_style) = if index == selected {
            let style = base().fg(ACCENT).with_attrs(bold());
            (style, style)
        } else {
            (base().fg(Color::Indexed(2)), base())
        };
        let split = title
            .char_indices()
            .nth(1)
            .map_or(title.len(), |(at, _)| at);
        x = put(cx, x, area.y, limit, &title[..split], bullet_style);
        x = put(cx, x, area.y, limit, &title[split..], rest_style);
        x = put(cx, x, area.y, limit, " ", base());
        if index + 1 < TAB_TITLES.len() {
            x = put(cx, x, area.y, limit, "│", base());
        }
    }
}

fn paint_checks(model: &Model, left: Rect, cx: &mut RenderCx<'_>) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Checks ")
        .style(base())
        .title_style(base().fg(DIM))
        .border_style(base().fg(DIM));
    let inner = block.inner(left);
    block.render(left, cx);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let columns = Layout::horizontal([
        Constraint::Length(COLUMN_WIDTHS[0]),
        Constraint::Length(COLUMN_WIDTHS[1]),
        Constraint::Length(COLUMN_WIDTHS[2]),
        Constraint::Fill(1),
    ])
    .spacing(1)
    .split(Rect::new(inner.x, inner.y, inner.width, 1))
    .segments;
    let head = base().with_attrs(bold());
    for (segment, text) in columns.iter().zip(["check", "state", "time", "file"]) {
        put(
            cx,
            segment.x,
            inner.y,
            segment.x + segment.width,
            text,
            head,
        );
    }
    cx.set_style(Rect::new(inner.x, inner.y, inner.width, 1), head);
    let rows = model
        .check_rows()
        .iter()
        .skip(model.scroll)
        .take(usize::from(inner.height - 1));
    for (offset, row) in rows.enumerate() {
        let y = inner.y + 1 + u16::try_from(offset).unwrap_or(0);
        let state = if row[1] == "FAIL" {
            base().fg(Color::Indexed(1)).with_attrs(bold())
        } else {
            base().fg(Color::Indexed(2))
        };
        for (index, segment) in columns.iter().enumerate() {
            let style = if index == 1 { state } else { base() };
            put(
                cx,
                segment.x,
                y,
                segment.x + segment.width,
                &row[index],
                style,
            );
        }
    }
}

fn paint_side(model: &Model, right: Rect, cx: &mut RenderCx<'_>) {
    if right.width == 0 || right.height <= 3 {
        return;
    }
    let log_h = right.height - 3;
    let log_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Log ")
        .style(base())
        .title_style(base())
        .border_style(base());
    let log_area = Rect::new(right.x, right.y, right.width, log_h);
    let log_inner = log_block.inner(log_area);
    log_block.render(log_area, cx);
    Paragraph::new(model.log_text())
        .wrap(TextWrap { trim: true })
        .scroll((u16::try_from(model.scroll).unwrap_or(u16::MAX), 0))
        .style(base())
        .render(log_inner, cx);
    paint_gauge(
        model,
        Rect::new(right.x, right.y + log_h, right.width, 3),
        cx,
    );
}

fn paint_gauge(model: &Model, area: Rect, cx: &mut RenderCx<'_>) {
    let block = Block::bordered()
        .title(" Progress ")
        .style(base())
        .title_style(base())
        .border_style(base());
    let inner = block.inner(area);
    block.render(area, cx);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let percent = model.tick % 100;
    let filled = u16::try_from((u32::from(inner.width) * percent + 50) / 100).unwrap_or(0);
    let bar = base().fg(ACCENT);
    cx.set_style(inner, bar);
    let label = format!("{percent}%");
    let label_x = inner.x
        + inner
            .width
            .saturating_sub(u16::try_from(label.len()).unwrap_or(0))
            / 2;
    let label_y = inner.y + inner.height / 2;
    let label_width = u16::try_from(label.len()).unwrap_or(0);
    for y in inner.y..inner.y + inner.height {
        for x in inner.x..inner.x + filled {
            if y == label_y && x >= label_x && x <= label_x + label_width {
                cx.put_str(x, y, " ", Style::new().bg(ACCENT));
            } else {
                cx.put_str(x, y, "█", Style::new().fg(ACCENT));
            }
        }
    }
    for (offset, ch) in label.chars().enumerate() {
        let x = label_x + u16::try_from(offset).unwrap_or(0);
        let style = if x < inner.x + filled {
            Style::new().bg(ACCENT)
        } else {
            bar
        };
        put(
            cx,
            x,
            label_y,
            inner.x + inner.width,
            &ch.to_string(),
            style,
        );
    }
}

fn paint_status(model: &Model, area: Rect, cx: &mut RenderCx<'_>) {
    let limit = area.x + area.width;
    let spinner = SPINNER[usize::try_from(model.tick % 10).unwrap_or(0)];
    let mut x = put(
        cx,
        area.x,
        area.y,
        limit,
        spinner,
        base().fg(Color::Indexed(6)),
    );
    x = put(cx, x, area.y, limit, " scorecard • 3 failing • ", base());
    let dim = base().with_attrs(Attrs::default().set_dim(true));
    put(cx, x, area.y, limit, &format!("frame {}", model.tick), dim);
}
