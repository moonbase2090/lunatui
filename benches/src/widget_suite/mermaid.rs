// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{MermaidDiagram, MermaidError, MermaidView, RenderCx, Widget};
use ratatui::{buffer::Buffer, layout::Rect as RatRect, style::Style};
use std::sync::OnceLock;

static DIAGRAMS: OnceLock<[Result<MermaidDiagram, MermaidError>; 2]> = OnceLock::new();
const DOWN: [&str; 8] = [
    "┌───┐",
    "│ A │",
    "└─┬─┘",
    "  │  ",
    "  ▼  ",
    "┌───┐",
    "│ B │",
    "└───┘",
];
const RIGHT: [&str; 3] = ["┌───┐  ┌───┐", "│ A ├─▶│ B │", "└───┘  └───┘"];

pub(super) fn native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let diagrams = DIAGRAMS.get_or_init(|| {
        [
            MermaidDiagram::parse("flowchart TD\n A --> B"),
            MermaidDiagram::parse("flowchart LR\n A --> B"),
        ]
    });
    if let Ok(diagram) = &diagrams[usize::try_from(tick % 2).unwrap_or(0)] {
        MermaidView::new(diagram).render(area, &mut RenderCx::new(grid, clusters, theme));
    }
}

pub(super) fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| super::rat_color(theme.resolve(token));
    let base = Style::new()
        .fg(color(Token::TextPrimary))
        .bg(color(Token::Surface0));
    buffer.set_style(area, base);
    let rows: &[&str] = if tick.is_multiple_of(2) {
        &DOWN
    } else {
        &RIGHT
    };
    for (row, text) in rows.iter().enumerate() {
        for (column, ch) in text.chars().enumerate() {
            let node = if tick.is_multiple_of(2) {
                !(3..5).contains(&row)
            } else {
                !(5..7).contains(&column)
            };
            let border = ch != ' ' && ch != 'A' && ch != 'B';
            let style = if node {
                let style = base.bg(color(Token::Surface1));
                if border {
                    style.fg(color(Token::Accent))
                } else {
                    style
                }
            } else if ch != ' ' {
                base.fg(color(Token::TextMuted))
            } else {
                base
            };
            let x = area.x + u16::try_from(column).unwrap_or(0);
            let y = area.y + u16::try_from(row).unwrap_or(0);
            if x < area.right() && y < area.bottom() {
                buffer[(x, y)].set_char(ch).set_style(style);
            }
        }
    }
}
