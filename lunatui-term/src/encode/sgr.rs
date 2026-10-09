// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
) {
    let effective = if colors {
        target
    } else {
        Style::new().with_attrs(target.attrs)
    };
    if current.fg == effective.fg && current.bg == effective.bg && current.attrs == effective.attrs
    {
        return;
    }
    let mut params = String::new();
    if effective.attrs != current.attrs
        || (colors && (effective.fg != current.fg || effective.bg != current.bg))
    {
        params.push('0');
        current.attrs = Attrs::new();
        current.fg = Color::Default;
        current.bg = Color::Default;
    }
    append_attr_params(&mut params, effective.attrs);
    if colors {
        append_color_params(&mut params, true, effective.fg);
        append_color_params(&mut params, false, effective.bg);
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

fn append_attr_params(params: &mut String, attrs: Attrs) {
    if attrs.bold() {
        push_param(params, "1");
    }
    if attrs.dim() {
        push_param(params, "2");
    }
    if attrs.italic() {
        push_param(params, "3");
    }
    match attrs.underline() {
        UnderlineStyle::None => {}
        UnderlineStyle::Single => push_param(params, "4"),
        UnderlineStyle::Double => push_param(params, "21"),
        UnderlineStyle::Curly => push_param(params, "4:3"),
        UnderlineStyle::Dotted => push_param(params, "4:4"),
        UnderlineStyle::Dashed => push_param(params, "4:5"),
    }
    if attrs.blink() {
        push_param(params, "5");
    }
    if attrs.reverse() {
        push_param(params, "7");
    }
    if attrs.hidden() {
        push_param(params, "8");
    }
    if attrs.strikethrough() {
        push_param(params, "9");
    }
    if attrs.overline() {
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

fn push_param(params: &mut String, value: &str) {
    if !params.is_empty() {
        params.push(';');
    }
    params.push_str(value);
}
