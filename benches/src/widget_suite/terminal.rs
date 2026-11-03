// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{CellWidth, ClusterTable, Color, Grid, Rect};
use lunatui_term::PtySize;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{PseudoTerminal, RenderCx, TerminalScreen, Widget};
use ratatui::{
    buffer::Buffer,
    layout::Rect as RatRect,
    style::{Modifier, Style},
};

fn screen(width: u16, height: u16, tick: u32) -> Option<TerminalScreen> {
    let mut screen = TerminalScreen::new(PtySize::new(width, height).ok()?);
    let output = format!(
        "orbit % cargo test\r\n\x1b[1;34mCompiling\x1b[0m orbit v0.1.0\r\n\
         running 2 tests\r\n\x1b[32mtest\x1b[0m model::saves ... \x1b[32mok\x1b[0m\r\n\
         \x1b[32mtest\x1b[0m model::restores ... \x1b[32mok\x1b[0m\r\n\
         \r\n\x1b[1;32mtest result: ok.\x1b[0m 2 passed; 0 failed\r\n\
         finished in 0.{:02}s\r\norbit % ",
        tick % 100,
    );
    screen.feed(output.as_bytes());
    Some(screen)
}

pub(super) fn native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    if let Some(screen) = screen(area.width, area.height, tick) {
        PseudoTerminal::new(&screen).render(area, &mut RenderCx::new(grid, clusters, theme));
    }
}

pub(super) fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    let Some(screen) = screen(area.width, area.height, tick) else {
        return;
    };
    let theme = Theme::builtin(Variant::Dark);
    let foreground = theme.resolve(Token::TextPrimary);
    let background = theme.resolve(Token::Surface0);
    for row in 0..area.height {
        for col in 0..area.width {
            let Some(cell) = screen.cell(col, row) else {
                continue;
            };
            if cell.width == CellWidth::Continuation {
                continue;
            }
            let resolve = |color, fallback| {
                super::rat_color(if color == Color::Default {
                    fallback
                } else {
                    color
                })
            };
            let mut style = Style::new()
                .fg(resolve(cell.style.fg, foreground))
                .bg(resolve(cell.style.bg, background));
            for (on, modifier) in [
                (cell.style.attrs.bold(), Modifier::BOLD),
                (cell.style.attrs.dim(), Modifier::DIM),
                (cell.style.attrs.italic(), Modifier::ITALIC),
                (cell.style.attrs.reverse(), Modifier::REVERSED),
            ] {
                if on {
                    style = style.add_modifier(modifier);
                }
            }
            let text = if cell.text.is_empty() { " " } else { cell.text };
            buffer[(area.x + col, area.y + row)]
                .set_symbol(text)
                .set_style(style);
        }
    }
}
