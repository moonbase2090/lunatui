// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_ratatui::{RatatuiStatefulWidget, RatatuiWidget};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Widget};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    BarChart, Block, BorderType, Gauge, List, ListItem, ListState, Paragraph, Row,
    Sparkline, Table, Tabs,
};

fn bridge_lines<W>(widget: &RatatuiWidget<W>, width: u16, height: u16) -> Vec<String>
where
    W: ratatui::widgets::Widget + Clone,
{
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, width, height), &mut cx);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn ratatui_lines<W>(widget: W, width: u16, height: u16) -> Vec<String>
where
    W: ratatui::widgets::Widget + Clone,
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| frame.render_widget(widget.clone(), RatRect::new(0, 0, width, height)))
        .expect("draw");
    let buf: &Buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
        .collect()
}

fn assert_parity<W>(name: &str, widget: W, width: u16, height: u16)
where
    W: ratatui::widgets::Widget + Clone,
{
    let wrapped = RatatuiWidget::new(widget.clone());
    let ours = bridge_lines(&wrapped, width, height);
    let theirs = ratatui_lines(widget, width, height);
    assert_eq!(ours, theirs, "{name}");
}

#[test]
fn ten_widgets_match_ratatui() {
    assert_parity("block", Block::bordered().title("L2"), 14, 6);
    assert_parity(
        "paragraph",
        Paragraph::new(Line::from(vec![
            Span::styled("hi", Style::default().fg(Color::Green)),
            Span::raw(" there"),
        ])),
        12,
        3,
    );
    assert_parity(
        "tabs",
        Tabs::new(vec!["One", "Two", "Three"]).select(1),
        18,
        3,
    );
    assert_parity("gauge", Gauge::default().percent(42).label("42%"), 16, 3);
    assert_parity(
        "sparkline",
        Sparkline::default().data(&[1, 3, 2, 4, 0, 5]),
        16,
        3,
    );
    assert_parity(
        "barchart",
        BarChart::default()
            .data(&[("a", 3_u64), ("bb", 5_u64)])
            .bar_width(3)
            .bar_gap(1),
        18,
        5,
    );
    assert_parity(
        "table",
        Table::new(
            vec![
                Row::new(vec!["a", "b"]),
                Row::new(vec!["c", "d"]).style(Style::default().add_modifier(Modifier::BOLD)),
            ],
            [4, 4],
        )
        .header(Row::new(vec!["H1", "H2"]).style(Style::default().fg(Color::Yellow))),
        12,
        5,
    );
    assert_parity(
        "paragraph_block",
        Paragraph::new("body").block(
            Block::bordered()
                .title("t")
                .border_type(BorderType::Rounded),
        ),
        14,
        5,
    );
    assert_parity(
        "block_double_border",
        Block::bordered()
            .border_type(BorderType::Double)
            .title_bottom("b"),
        12,
        4,
    );
    assert_parity(
        "sparkline_max",
        Sparkline::default().data(&[0, 10, 5]).max(10),
        10,
        2,
    );
}

#[test]
fn list_stateful_matches_ratatui() {
    let width = 12;
    let height = 5;
    let items = vec![
        ListItem::new("alpha"),
        ListItem::new("beta"),
        ListItem::new("gamma"),
    ];
    let rat_widget = List::new(items.clone()).highlight_symbol("> ");
    let mut rat_state = ListState::default().with_selected(Some(1));

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            ratatui::widgets::StatefulWidget::render(
                rat_widget.clone(),
                RatRect::new(0, 0, width, height),
                frame.buffer_mut(),
                &mut rat_state,
            );
        })
        .expect("draw");
    let buf: &Buffer = terminal.backend().buffer();
    let theirs: Vec<String> = (0..height)
        .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
        .collect();

    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let bridge = RatatuiStatefulWidget::new(List::new(items).highlight_symbol("> "), {
        let mut state = ListState::default();
        state.select(Some(1));
        state
    });
    bridge.render(Rect::new(0, 0, width, height), &mut cx);
    let ours: Vec<String> = (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect();
    assert_eq!(ours, theirs);
}

#[test]
fn memo_reuses_conversion() {
    let widget = RatatuiWidget::new(Block::bordered().title("memo"));
    let theme = Theme::builtin(Variant::Dark);
    let area = Rect::new(0, 0, 10, 4);
    for _ in 0..2 {
        let mut grid = Grid::new(area.width, area.height);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        widget.render(area, &mut cx);
    }
    assert_eq!(widget.memo_conversions(), 1);
    assert_eq!(widget.memo_hits(), 1);
}
