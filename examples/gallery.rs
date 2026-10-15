// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Paragraph, RenderCx, TextWrap, Widget};

const BLURB: &str = "PR 30: Paragraph in the Phase 4 widget gallery.\n\
Wrapped body text inside a bordered block.\n";

fn main() -> io::Result<()> {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let width = 60_u16;
    let height = 12_u16;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let block = Block::bordered()
        .title("gallery")
        .style(Style::new().fg(cx.token(Token::TextPrimary)));
    block.render(Rect::new(0, 0, width, height), &mut cx);
    let inner = block.inner(Rect::new(0, 0, width, height));
    let paragraph = Paragraph::new(BLURB)
        .wrap(TextWrap { trim: true })
        .style(Style::new().fg(cx.token(Token::TextPrimary)));
    paragraph.render(inner, &mut cx);
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
    io::stdout().write_all(&bytes)?;
    Ok(())
}
