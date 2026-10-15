// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, Constraint, List, Paragraph, RenderCx, Table, TextWrap, VecSource, Widget,
};

const BLURB: &str = "PR 30–31: Paragraph, List, and Table in the widget gallery.\n";

fn main() -> io::Result<()> {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let width = 72_u16;
    let height = 18_u16;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let text_style = Style::new().fg(cx.token(Token::TextPrimary));
    let block = Block::bordered().title("gallery").style(text_style);
    block.render(Rect::new(0, 0, width, height), &mut cx);
    let inner = block.inner(Rect::new(0, 0, width, height));
    let paragraph = Paragraph::new(BLURB)
        .wrap(TextWrap { trim: true })
        .style(text_style);
    paragraph.render(Rect::new(inner.x, inner.y, inner.width, 3), &mut cx);
    let mut list_source = VecSource::new();
    for item in ["gates", "layout", "widgets"] {
        list_source.push_row(vec![item.to_owned()]);
    }
    let list = List::new(list_source).style(text_style).selected(Some(1));
    list.render(
        Rect::new(inner.x, inner.y + 4, 14, inner.height.saturating_sub(5)),
        &mut cx,
    );
    let table_rows: Vec<Vec<String>> = (0..8)
        .map(|i| {
            vec![
                format!("check-{i:02}"),
                "pass".to_owned(),
                format!("{i}ms"),
                format!("src/{i}.rs"),
            ]
        })
        .collect();
    let table = Table::new(
        VecSource::from_rows(table_rows),
        vec![
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Fill(1),
        ],
    )
    .selected(Some(2))
    .style(text_style);
    table.render(
        Rect::new(
            inner.x + 16,
            inner.y + 4,
            inner.width.saturating_sub(16),
            inner.height.saturating_sub(5),
        ),
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
