// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_runtime::Ease;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, GraphNode, NodeConnection, NodeGraph, Paragraph, RenderCx, Widget, blend_rgb,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect as RatRect,
    style::{Modifier, Style as RatStyle},
    widgets::{
        Block as RatBlock, BorderType, Paragraph as RatParagraph, StatefulWidget,
        Widget as RatWidget,
    },
};
use std::sync::OnceLock;

const EDGES: [NodeConnection; 3] = [
    NodeConnection::new(0, 0, 2, 0),
    NodeConnection::new(1, 0, 2, 4),
    NodeConnection::new(2, 2, 3, 2),
];
const FOOTERS: [&str; 4] = [
    "01  Repository synced",
    "02  Checks passed",
    "03  Package prepared",
    "04  Release published",
];

fn dimensions(area: Rect) -> Option<(Rect, u16)> {
    if area.width < 50 || area.height < 18 {
        return None;
    }
    let width = ((area.width - 20) / 3).min(40);
    let total = width * 3 + 12;
    Some((
        Rect::new(
            area.x + (area.width - total) / 2,
            area.y + (area.height - 10) / 2 + 1,
            total,
            10,
        ),
        width,
    ))
}

fn nodes(width: u16, tick: u32) -> [GraphNode<'static>; 4] {
    [
        GraphNode::new("Repository", Size::new(width, 4)).body("main\nsynced"),
        GraphNode::new("Checks", Size::new(width, 4)).body("24 checks\npassed"),
        GraphNode::new("Package", Size::new(width, 8))
            .body("repository\n\nbundle\n\nchecks\nready"),
        GraphNode::new("Release", Size::new(width, 8)).body(if tick % 240 >= 180 {
            "stable\n\npublished"
        } else {
            "stable\n\nqueued"
        }),
    ]
}

fn selected(tick: u32) -> usize {
    usize::try_from((tick / 60) % 4).unwrap_or(0)
}

fn ink(theme: &Theme, fg: Token, bg: Token) -> Style {
    Style::new().fg(theme.resolve(fg)).bg(theme.resolve(bg))
}

fn highlight(theme: &Theme, tick: u32) -> Style {
    let ring = theme.resolve(Token::FocusRing);
    let border = theme.resolve(Token::TextMuted);
    let local = u16::try_from(tick % 60).unwrap_or(0);
    let level = Ease::InOutCubic.apply(f32::from(local.min(59 - local)) / 29.0);
    let color = match (border, ring) {
        (Color::Rgb(red, green, blue), Color::Rgb(target_red, target_green, target_blue)) => {
            let mixed = blend_rgb(
                (red, green, blue),
                (target_red, target_green, target_blue),
                0.45 + 0.55 * level,
            );
            Color::Rgb(mixed.0, mixed.1, mixed.2)
        }
        _ => ring,
    };
    ink(theme, Token::FocusRing, Token::Surface1)
        .fg(color)
        .with_attrs(Attrs::new().set_bold(true))
}

fn rat_style(style: Style) -> RatStyle {
    let mut value = RatStyle::new()
        .fg(super::rat_color(style.fg))
        .bg(super::rat_color(style.bg));
    if style.attrs.bold() {
        value = value.add_modifier(Modifier::BOLD);
    }
    value
}

/// Renders the actual native graph. Construction and layout are included for fair timing.
pub fn native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    draw(&mut RenderCx::new(grid, clusters, theme), area, tick);
}

/// Draws the native scene using the caller's theme, prepared palette and optional pixel sink.
///
/// # Panics
/// Panics if the fixed, valid fixture is rejected by the graph layout implementation.
pub fn draw(cx: &mut RenderCx<'_>, area: Rect, tick: u32) {
    let theme = cx.theme();
    let page = ink(theme, Token::TextPrimary, Token::Surface0);
    Block::new().style(page).render(area, cx);
    let Some((graph_area, width)) = dimensions(area) else {
        return;
    };
    let data = nodes(width, tick);
    let graph = NodeGraph::new(&data, &EDGES);
    assert!(
        graph.is_ok(),
        "the fixed graph fixture must have valid topology"
    );
    if let Ok(graph) = graph {
        graph
            .view()
            .selected(Some(selected(tick)))
            .highlight_style(highlight(theme, tick))
            .render(graph_area, cx);
    }
    native_labels(cx, theme, area, tick);
}

fn native_labels(cx: &mut RenderCx<'_>, theme: &Theme, area: Rect, tick: u32) {
    let page = ink(theme, Token::TextPrimary, Token::Surface0);
    for (y, text, style) in [
        (
            2,
            "RELEASE PIPELINE",
            ink(theme, Token::Accent, Token::Surface0),
        ),
        (3, "Repository and checks feed the release package", page),
        (area.height - 3, FOOTERS[selected(tick)], page),
    ] {
        let row = Rect::new(area.x + 3, area.y + y, area.width - 6, 1);
        Block::new().style(style).render(row, cx);
        Paragraph::new(text).style(style).render(row, cx);
    }
}

/// Renders the same scene through tui-nodes' public layout and widget APIs.
pub fn baseline_themed(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let page = ink(theme, Token::TextPrimary, Token::Surface0);
    RatBlock::new().style(rat_style(page)).render(area, buffer);
    let Some((graph_area, width)) = dimensions(Rect::new(area.x, area.y, area.width, area.height))
    else {
        return;
    };
    let data = nodes(width, tick);
    let layouts = data
        .iter()
        .enumerate()
        .map(|(index, _)| {
            tui_nodes::NodeLayout::new((width, if index < 2 { 4 } else { 8 }))
                .with_title(["Repository", "Checks", "Package", "Release"][index])
                .with_border_type(BorderType::Rounded)
                .with_border_style(rat_style(if selected(tick) == index {
                    highlight(theme, tick)
                } else {
                    ink(theme, Token::TextMuted, Token::Surface1)
                }))
        })
        .collect();
    let line = rat_style(ink(theme, Token::Accent, Token::Surface0));
    let edges = vec![
        tui_nodes::Connection::new(0, 0, 2, 0).with_line_style(line),
        tui_nodes::Connection::new(1, 0, 2, 4).with_line_style(line),
        tui_nodes::Connection::new(2, 2, 3, 2).with_line_style(line),
    ];
    let inner = RatRect::new(
        graph_area.x + 1,
        graph_area.y + 1,
        graph_area.width - 2,
        graph_area.height - 2,
    );
    let mut graph = tui_nodes::NodeGraph::new(
        layouts,
        edges,
        usize::from(inner.width),
        usize::from(inner.height),
    );
    graph.calculate();
    let bodies = [
        "main\nsynced",
        "24 checks\npassed",
        "repository\n\nbundle\n\nchecks\nready",
        if tick % 240 >= 180 {
            "stable\n\npublished"
        } else {
            "stable\n\nqueued"
        },
    ];
    for (rect, body) in graph.split(inner).into_iter().zip(bodies) {
        RatParagraph::new(body)
            .style(rat_style(ink(theme, Token::TextPrimary, Token::Surface1)))
            .render(rect, buffer);
    }
    StatefulWidget::render(graph, inner, buffer, &mut ());
    for (y, text, style) in [
        (
            2,
            "RELEASE PIPELINE",
            ink(theme, Token::Accent, Token::Surface0),
        ),
        (3, "Repository and checks feed the release package", page),
        (area.height - 3, FOOTERS[selected(tick)], page),
    ] {
        RatParagraph::new(text).style(rat_style(style)).render(
            RatRect::new(area.x + 3, area.y + y, area.width - 6, 1),
            buffer,
        );
    }
}

/// Draws the suite's dark-theme baseline with a reused theme, like the native runner.
pub fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    static THEME: OnceLock<Theme> = OnceLock::new();
    baseline_themed(
        buffer,
        area,
        THEME.get_or_init(|| Theme::builtin(Variant::Dark)),
        tick,
    );
}
