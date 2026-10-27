// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Alignment, BigText, Block, BorderType, PixelSize, RenderCx, Widget, WidgetState,
};

fn state_name(state: WidgetState) -> &'static str {
    if state == WidgetState::Focused {
        "focused"
    } else if state == WidgetState::Disabled {
        "disabled"
    } else {
        "normal"
    }
}

fn border_for(state: WidgetState) -> BorderType {
    match state {
        WidgetState::Focused => BorderType::Thick,
        _ => BorderType::Rounded,
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    let text = cx.token(Token::TextPrimary);
    let accent = cx.token(Token::Accent);
    let surface = cx.token(Token::Surface0);
    let raised = cx.token(Token::Surface1);
    let focus = cx.token(Token::FocusRing);
    let ground = Style::new().fg(text).bg(surface);
    cx.set_style(area, ground);
    let states = [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ];
    let panel_h = area.height.saturating_sub(2) / 3;
    for (index, state) in states.iter().enumerate() {
        let y = 1 + u16::try_from(index).unwrap_or(0) * (panel_h + 1);
        let panel = Rect::new(1, y, area.width.saturating_sub(2), panel_h);
        let tone = match state {
            WidgetState::Focused => focus,
            WidgetState::Disabled => raised,
            _ => accent,
        };
        let frame = Block::bordered()
            .border_type(border_for(*state))
            .title(format!(" {} ", state_name(*state)))
            .style(ground)
            .border_style(Style::new().fg(tone).bg(surface))
            .title_style(Style::new().fg(tone).bg(surface));
        frame.render(panel, &mut cx);
        let inner = frame.inner(panel);
        let (_width, glyph_h) = PixelSize::HalfHeight.cells();
        if inner.height < glyph_h || inner.width < 4 {
            continue;
        }
        BigText::new("LUNATUI")
            .pixel_size(PixelSize::HalfHeight)
            .alignment(Alignment::Center)
            .state(*state)
            .render(Rect::new(inner.x, inner.y, inner.width, glyph_h), &mut cx);
    }
}

fn main() -> io::Result<()> {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let width = 72_u16;
    let height = 30_u16;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    draw(&mut grid, &mut clusters, &theme);
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
