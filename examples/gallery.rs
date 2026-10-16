// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, Constraint, Gauge, List, Paragraph, RenderCx, Scrollbar, Spinner, StatusBar, Table,
    Tabs, TextInput, TextWrap, VecSource, Widget,
};

const BLURB: &str = "PR 32: Tabs, Gauge, Scrollbar, Spinner, StatusBar, TextInput.\n";

fn main() -> io::Result<()> {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let width = 72_u16;
    let height = 22_u16;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let text_style = Style::new().fg(cx.token(Token::TextPrimary));
    let accent = Style::new().fg(cx.token(Token::Accent));
    let block = Block::bordered().title("gallery").style(text_style);
    block.render(Rect::new(0, 0, width, height), &mut cx);
    let inner = block.inner(Rect::new(0, 0, width, height));
    Tabs::new(["Overview", "Lints", "Tests"])
        .select(1)
        .style(text_style)
        .highlight_style(accent)
        .render(Rect::new(inner.x, inner.y, inner.width, 1), &mut cx);
    Paragraph::new(BLURB)
        .wrap(TextWrap { trim: true })
        .style(text_style)
        .render(Rect::new(inner.x, inner.y + 2, inner.width, 2), &mut cx);
    let mut list_source = VecSource::new();
    for item in ["gates", "layout", "widgets", "chrome"] {
        list_source.push_row(vec![item.to_owned()]);
    }
    let list_h = 6_u16;
    List::new(list_source)
        .style(text_style)
        .selected(Some(2))
        .render(Rect::new(inner.x, inner.y + 5, 12, list_h), &mut cx);
    Scrollbar::vertical(4, usize::from(list_h), 0)
        .track_style(text_style)
        .thumb_style(accent)
        .render(Rect::new(inner.x + 12, inner.y + 5, 1, list_h), &mut cx);
    TextInput::new("lunatui", 4)
        .block(Block::bordered().title("filter"))
        .style(text_style)
        .cursor_style(accent)
        .render(Rect::new(inner.x + 14, inner.y + 5, 20, 3), &mut cx);
    Gauge::new(0.62)
        .block(Block::bordered().title("progress"))
        .gauge_style(accent)
        .render(
            Rect::new(inner.x + 36, inner.y + 5, inner.width.saturating_sub(36), 3),
            &mut cx,
        );
    let table_rows: Vec<Vec<String>> = (0..6)
        .map(|i| vec![format!("check-{i:02}"), "pass".to_owned(), format!("{i}ms")])
        .collect();
    Table::new(
        VecSource::from_rows(table_rows),
        vec![
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Fill(1),
        ],
    )
    .selected(Some(1))
    .style(text_style)
    .render(Rect::new(inner.x, inner.y + 12, inner.width, 6), &mut cx);
    StatusBar::new("⠴ scorecard • 3 failing • frame 42")
        .style(text_style)
        .render(
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
            &mut cx,
        );
    Spinner::new(4).style(accent).render(
        Rect::new(inner.x + inner.width - 2, inner.y + inner.height - 1, 1, 1),
        &mut cx,
    );
    if snapshot {
        let mut out = String::new();
        for y in 0..height {
            out.push_str(&grid.row_display(y, &clusters));
            out.push('\n');
        }
        io::stdout().write_all(out.as_bytes())?;
        return Ok(());
    }
    let mut encoder = Encoder::new();
    let prev = Grid::new(width, height);
    let bytes = encoder.encode_diff(&prev, &grid, &clusters, ColorMode::NoColor);
    io::stdout().write_all(bytes)?;
    Ok(())
}
