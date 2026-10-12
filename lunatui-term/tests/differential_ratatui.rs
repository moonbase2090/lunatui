// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;

use lunatui_bench::scenes::ratatui::{ColorOutput, encode, set_color_output};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Style, UnderlineStyle};
use lunatui_term::{ColorMode, Encoder};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as RColor, Modifier, Style as RStyle};

fn lunatui_style(style: RStyle) -> Style {
    let mut out = Style::new();
    out.fg = map_color(style.fg.unwrap_or(RColor::Reset));
    out.bg = map_color(style.bg.unwrap_or(RColor::Reset));
    out.attrs = map_attrs(style.add_modifier);
    out
}

fn map_color(color: RColor) -> Color {
    match color {
        RColor::Reset => Color::Default,
        RColor::Black => Color::Indexed(0),
        RColor::Red => Color::Indexed(1),
        RColor::Green => Color::Indexed(2),
        RColor::Yellow => Color::Indexed(3),
        RColor::Blue => Color::Indexed(4),
        RColor::Magenta => Color::Indexed(5),
        RColor::Cyan => Color::Indexed(6),
        RColor::Gray => Color::Indexed(7),
        RColor::DarkGray => Color::Indexed(8),
        RColor::LightRed => Color::Indexed(9),
        RColor::LightGreen => Color::Indexed(10),
        RColor::LightYellow => Color::Indexed(11),
        RColor::LightBlue => Color::Indexed(12),
        RColor::LightMagenta => Color::Indexed(13),
        RColor::LightCyan => Color::Indexed(14),
        RColor::White => Color::Indexed(15),
        RColor::Indexed(index) => Color::Indexed(index),
        RColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
    }
}

fn map_attrs(modifier: Modifier) -> Attrs {
    let mut attrs = Attrs::new();
    if modifier.contains(Modifier::BOLD) {
        attrs = attrs.set_bold(true);
    }
    if modifier.contains(Modifier::DIM) {
        attrs = attrs.set_dim(true);
    }
    if modifier.contains(Modifier::ITALIC) {
        attrs = attrs.set_italic(true);
    }
    if modifier.contains(Modifier::UNDERLINED) {
        attrs = attrs.set_underline(UnderlineStyle::Single);
    }
    if modifier.contains(Modifier::SLOW_BLINK) | modifier.contains(Modifier::RAPID_BLINK) {
        attrs = attrs.set_blink(true);
    }
    if modifier.contains(Modifier::REVERSED) {
        attrs = attrs.set_reverse(true);
    }
    if modifier.contains(Modifier::HIDDEN) {
        attrs = attrs.set_hidden(true);
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        attrs = attrs.set_strikethrough(true);
    }
    attrs
}

fn grid_from_buffer(buffer: &Buffer) -> (Grid, ClusterTable) {
    let area = buffer.area;
    let mut grid = Grid::new(area.width, area.height);
    let mut clusters = ClusterTable::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol.is_empty() {
                continue;
            }
            let style = lunatui_style(cell.style());
            grid.put_str(x, y, style, symbol, &mut clusters);
        }
    }
    (grid, clusters)
}

#[test]
fn two_cell_update_matches_ratatui_no_color() -> io::Result<()> {
    set_color_output(ColorOutput::Disabled);
    let area = Rect::new(0, 0, 3, 1);
    let mut prev = Buffer::empty(area);
    prev[(0, 0)].set_symbol("A");
    prev[(1, 0)].set_symbol(" ");
    prev[(2, 0)].set_symbol(" ");
    let mut next = prev.clone();
    next[(1, 0)].set_symbol("B");
    let (grid_prev, _) = grid_from_buffer(&prev);
    let (grid_next, clusters) = grid_from_buffer(&next);
    let mut encoder = Encoder::new();
    let ours = encoder
        .encode_diff(&grid_prev, &grid_next, &clusters, ColorMode::NoColor)
        .to_vec();
    let mut rat = Vec::new();
    let stats = encode(&prev, &next, &mut rat)?;
    assert_eq!(stats.bytes, ours.len());
    assert_eq!(rat, ours);
    Ok(())
}
