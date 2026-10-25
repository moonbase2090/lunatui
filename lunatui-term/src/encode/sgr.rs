// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Style, UnderlineStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pen {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Default for Pen {
    fn default() -> Self {
        Self {
            fg: Color::Default,
            bg: Color::Default,
            attrs: Attrs::new(),
        }
    }
}

pub(crate) fn write_sgr_delta(
    buffer: &mut Vec<u8>,
    current: &mut Pen,
    target: Style,
    colors: bool,
    token_palette: &[Color],
) {
    let effective = if colors {
        Style {
            fg: resolve_token_color(target.fg, token_palette),
            bg: resolve_token_color(target.bg, token_palette),
            attrs: target.attrs,
        }
    } else {
        Style::new().with_attrs(target.attrs)
    };
    if current.fg == effective.fg && current.bg == effective.bg && current.attrs == effective.attrs
    {
        return;
    }
    let mut params = String::new();
    if drops_attrs(current.attrs, effective.attrs) {
        params.push('0');
        *current = Pen::default();
    }
    append_attr_params(&mut params, effective.attrs, current.attrs);
    if colors {
        if effective.fg != current.fg {
            append_color_params(&mut params, true, effective.fg);
        }
        if effective.bg != current.bg {
            append_color_params(&mut params, false, effective.bg);
        }
    }
    if params.is_empty() {
        return;
    }
    buffer.extend_from_slice(b"\x1b[");
    buffer.extend_from_slice(params.as_bytes());
    buffer.push(b'm');
    current.attrs = effective.attrs;
    if colors {
        current.fg = effective.fg;
        current.bg = effective.bg;
    }
}

fn drops_attrs(have: Attrs, want: Attrs) -> bool {
    (have.bold() && !want.bold())
        || (have.dim() && !want.dim())
        || (have.italic() && !want.italic())
        || (have.blink() && !want.blink())
        || (have.reverse() && !want.reverse())
        || (have.hidden() && !want.hidden())
        || (have.strikethrough() && !want.strikethrough())
        || (have.overline() && !want.overline())
        || (have.underline() != UnderlineStyle::None && have.underline() != want.underline())
}

fn append_attr_params(params: &mut String, attrs: Attrs, have: Attrs) {
    if attrs.bold() && !have.bold() {
        push_param(params, "1");
    }
    if attrs.dim() && !have.dim() {
        push_param(params, "2");
    }
    if attrs.italic() && !have.italic() {
        push_param(params, "3");
    }
    if have.underline() == UnderlineStyle::None {
        match attrs.underline() {
            UnderlineStyle::None => {}
            UnderlineStyle::Single => push_param(params, "4"),
            UnderlineStyle::Double => push_param(params, "21"),
            UnderlineStyle::Curly => push_param(params, "4:3"),
            UnderlineStyle::Dotted => push_param(params, "4:4"),
            UnderlineStyle::Dashed => push_param(params, "4:5"),
        }
    }
    if attrs.blink() && !have.blink() {
        push_param(params, "5");
    }
    if attrs.reverse() && !have.reverse() {
        push_param(params, "7");
    }
    if attrs.hidden() && !have.hidden() {
        push_param(params, "8");
    }
    if attrs.strikethrough() && !have.strikethrough() {
        push_param(params, "9");
    }
    if attrs.overline() && !have.overline() {
        push_param(params, "53");
    }
}

fn append_color_params(params: &mut String, fg: bool, color: Color) {
    match color {
        Color::Default => {
            if fg {
                push_param(params, "39");
            } else {
                push_param(params, "49");
            }
        }
        Color::Indexed(index) => {
            if fg {
                push_param(params, &format!("38;5;{index}"));
            } else {
                push_param(params, &format!("48;5;{index}"));
            }
        }
        Color::Rgb(red, green, blue) => {
            if fg {
                push_param(params, &format!("38;2;{red};{green};{blue}"));
            } else {
                push_param(params, &format!("48;2;{red};{green};{blue}"));
            }
        }
        Color::Token(id) => {
            let index = id % 256;
            if fg {
                push_param(params, &format!("38;5;{index}"));
            } else {
                push_param(params, &format!("48;5;{index}"));
            }
        }
    }
}

fn resolve_token_color(color: Color, palette: &[Color]) -> Color {
    match color {
        Color::Token(id) => palette.get(id as usize).copied().unwrap_or(color),
        other => other,
    }
}

fn push_param(params: &mut String, value: &str) {
    if !params.is_empty() {
        params.push(';');
    }
    params.push_str(value);
}
