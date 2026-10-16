// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Style, UnderlineStyle};
use ratatui::style::Color as RColor;
use ratatui::style::Modifier;
use ratatui::style::Style as RStyle;

/// Converts a ratatui color into a lunatui [`Color`].
#[must_use]
pub fn color_from_ratatui(color: RColor) -> Color {
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

/// Converts a lunatui [`Color`] into ratatui (common subset; tokens map to reset).
#[must_use]
pub fn color_to_ratatui(color: Color) -> RColor {
    match color {
        Color::Indexed(0) => RColor::Black,
        Color::Indexed(1) => RColor::Red,
        Color::Indexed(2) => RColor::Green,
        Color::Indexed(3) => RColor::Yellow,
        Color::Indexed(4) => RColor::Blue,
        Color::Indexed(5) => RColor::Magenta,
        Color::Indexed(6) => RColor::Cyan,
        Color::Indexed(7) => RColor::Gray,
        Color::Indexed(8) => RColor::DarkGray,
        Color::Indexed(9) => RColor::LightRed,
        Color::Indexed(10) => RColor::LightGreen,
        Color::Indexed(11) => RColor::LightYellow,
        Color::Indexed(12) => RColor::LightBlue,
        Color::Indexed(13) => RColor::LightMagenta,
        Color::Indexed(14) => RColor::LightCyan,
        Color::Indexed(15) => RColor::White,
        Color::Indexed(index) => RColor::Indexed(index),
        Color::Rgb(red, green, blue) => RColor::Rgb(red, green, blue),
        Color::Default | Color::Token(_) => RColor::Reset,
    }
}

/// Converts a ratatui [`RStyle`] into a lunatui [`Style`].
#[must_use]
pub fn style_from_ratatui(style: RStyle) -> Style {
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
        .fg(color_from_ratatui(style.fg.unwrap_or(RColor::Reset)))
        .bg(color_from_ratatui(style.bg.unwrap_or(RColor::Reset)))
        .with_attrs(attrs)
}

/// Converts a lunatui [`Style`] into ratatui (common subset).
#[must_use]
pub fn style_to_ratatui(style: Style) -> RStyle {
    let mut modifier = Modifier::empty();
    let attrs = style.attrs;
    if attrs.bold() {
        modifier |= Modifier::BOLD;
    }
    if attrs.dim() {
        modifier |= Modifier::DIM;
    }
    if attrs.italic() {
        modifier |= Modifier::ITALIC;
    }
    if attrs.underline() != UnderlineStyle::None {
        modifier |= Modifier::UNDERLINED;
    }
    if attrs.reverse() {
        modifier |= Modifier::REVERSED;
    }
    if attrs.strikethrough() {
        modifier |= Modifier::CROSSED_OUT;
    }
    if attrs.hidden() {
        modifier |= Modifier::HIDDEN;
    }
    if attrs.blink() {
        modifier |= Modifier::SLOW_BLINK;
    }
    let mut out = RStyle::default().add_modifier(modifier);
    if style.fg != Color::Default {
        out = out.fg(color_to_ratatui(style.fg));
    }
    if style.bg != Color::Default {
        out = out.bg(color_to_ratatui(style.bg));
    }
    out
}
