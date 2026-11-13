// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io::{self, Write};

use lunatui_core::{Cell, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Button, RenderCx, Widget, WidgetState};

fn main() -> io::Result<()> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(44, 2);
    let mut clusters = lunatui_core::ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let page = Color::Rgb(12, 14, 20);
    cx.fill(Rect::new(0, 0, 44, 2), Style::new().bg(page), ' ');
    let states = [
        WidgetState::Hover,
        WidgetState::Pressed,
        WidgetState::Invalid,
        WidgetState::Open,
    ];
    let names = ["Hover", "Pressed", "Invalid", "Open"];
    for (index, (state, name)) in states.into_iter().zip(names).enumerate() {
        let x = u16::try_from(index).unwrap_or(0) * 11;
        cx.put_str(
            x,
            0,
            name,
            Style::new().fg(theme.resolve(state.token())).bg(page),
        );
        Button::new(name)
            .state(state)
            .render(Rect::new(x, 1, 10, 1), &mut cx);
    }
    let mut out = io::stdout().lock();
    write_ppm(&grid, 10, 18, &mut out)?;
    Ok(())
}

fn write_ppm(grid: &Grid, cell_w: u32, cell_h: u32, out: &mut impl Write) -> io::Result<()> {
    let width = u32::from(grid.width()) * cell_w;
    let height = u32::from(grid.height()) * cell_h;
    writeln!(out, "P6 {width} {height} 255")?;
    for y in 0..grid.height() {
        let row: Vec<([u8; 3], [u8; 3])> = (0..grid.width())
            .map(|x| {
                let style = grid.get(x, y).map(Cell::style).unwrap_or_default();
                (rgb(style.fg), rgb(style.bg))
            })
            .collect();
        let band = cell_h / 3;
        for line in 0..cell_h {
            let foreground = line < band;
            for (fg, bg) in &row {
                let color = if foreground { fg } else { bg };
                for _ in 0..cell_w {
                    out.write_all(color)?;
                }
            }
        }
    }
    Ok(())
}

fn rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(red, green, blue) => [red, green, blue],
        _ => [0, 0, 0],
    }
}
