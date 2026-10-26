// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, BorderType, Date, Monthly, RenderCx, Widget, WidgetState};

fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_ymd(year, month, day).unwrap_or(Date::UNIX_EPOCH)
}

fn state_name(state: WidgetState) -> &'static str {
    if state == WidgetState::Focused {
        "focused"
    } else if state == WidgetState::Disabled {
        "disabled"
    } else {
        "normal"
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
    let ok = cx.token(Token::Ok);
    let warn = cx.token(Token::Warn);
    let ground = Style::new().fg(text).bg(surface);
    cx.set_style(area, ground);
    let states = [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ];
    let gap = 1_u16;
    let panel_w = area.width.saturating_sub(2).saturating_sub(gap * 2) / 3;
    let events = [
        (date(2026, 10, 3), Style::new().fg(warn)),
        (date(2026, 10, 14), Style::new().fg(ok)),
        (date(2026, 10, 26), Style::new().fg(accent)),
    ];
    for (index, state) in states.iter().enumerate() {
        let x = 1 + u16::try_from(index).unwrap_or(0) * (panel_w + gap);
        let panel = Rect::new(x, 1, panel_w, area.height.saturating_sub(2));
        let tone = match state {
            WidgetState::Focused => focus,
            WidgetState::Disabled => raised,
            _ => accent,
        };
        let border = if *state == WidgetState::Focused {
            BorderType::Thick
        } else {
            BorderType::Rounded
        };
        let frame = Block::bordered()
            .border_type(border)
            .title(format!(" October  {}", state_name(*state)))
            .style(ground)
            .border_style(Style::new().fg(tone).bg(surface))
            .title_style(Style::new().fg(tone).bg(surface));
        frame.render(panel, &mut cx);
        Monthly::new(date(2026, 10, 26), events.as_slice())
            .state(*state)
            .render(frame.inner(panel), &mut cx);
    }
}

fn main() -> io::Result<()> {
    let snapshot = env::args().any(|arg| arg == "--snapshot");
    let width = 80_u16;
    let height = 16_u16;
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
