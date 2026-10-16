// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Style, UnderlineStyle};
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
