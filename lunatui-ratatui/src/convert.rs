// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Style, UnderlineStyle, width};
use ratatui::buffer::Buffer;
use ratatui::buffer::Cell as RCell;
use ratatui::style::Color as RColor;
use ratatui::style::Modifier;
use ratatui::style::Style as RStyle;

pub(crate) fn apply_cell(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    x: u16,
    y: u16,
    cell: &RCell,
) {
    let style = style_from_cell(cell);
    let symbol = cell.symbol();
    if symbol.is_empty() {
        grid.put_str(x, y, style, " ", clusters);
        return;
    }
    grid.put_str(x, y, style, symbol, clusters);
}

pub(crate) fn style_from_cell(cell: &RCell) -> Style {
    map_style(
        RStyle::default()
            .fg(cell.fg)
            .bg(cell.bg)
            .add_modifier(cell.modifier),
    )
}

fn map_style(style: RStyle) -> Style {
    let mut attrs = Attrs::new();
    let modifier = style.add_modifier;
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
    if modifier.contains(Modifier::REVERSED) {
        attrs = attrs.set_reverse(true);
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        attrs = attrs.set_strikethrough(true);
    }
    if modifier.contains(Modifier::HIDDEN) {
        attrs = attrs.set_hidden(true);
    }
    if modifier.contains(Modifier::SLOW_BLINK) || modifier.contains(Modifier::RAPID_BLINK) {
        attrs = attrs.set_blink(true);
    }
    Style::new()
        .fg(map_color(style.fg.unwrap_or(RColor::Reset)))
        .bg(map_color(style.bg.unwrap_or(RColor::Reset)))
        .with_attrs(attrs)
}

/// Maps a ratatui [`RColor`] to [`Color`].
#[must_use]
pub fn map_color(color: RColor) -> Color {
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

/// Builds a lunatui grid from a ratatui buffer (same rules as differential tests).
#[must_use]
pub fn grid_from_buffer(buffer: &Buffer) -> (Grid, ClusterTable) {
    let area = buffer.area;
    let mut grid = Grid::new(area.width, area.height);
    let mut clusters = ClusterTable::new();
    for y in 0..area.height {
        let mut x = 0;
        while x < area.width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol.is_empty() {
                x += 1;
                continue;
            }
            grid.put_str(x, y, style_from_cell(cell), symbol, &mut clusters);
            let span = u16::try_from(width(symbol).max(1)).unwrap_or(1);
            x += span;
        }
    }
    (grid, clusters)
}

/// Visible text per row from a ratatui buffer (for parity checks).
#[must_use]
pub fn buffer_row_text(buffer: &Buffer, row: u16) -> String {
    let area = buffer.area;
    let mut line = String::new();
    let mut col = 0;
    while col < area.width {
        let cell = &buffer[(col, row)];
        line.push_str(cell.symbol());
        let span = u16::try_from(width(cell.symbol()).max(1)).unwrap_or(1);
        col += span;
    }
    line
}
