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
    let mut params = Params::new(buffer);
    if drops_attrs(current.attrs, effective.attrs) {
        params.text("0");
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
    if !params.finish() {
        return;
    }
    current.attrs = effective.attrs;
    if colors {
        current.fg = effective.fg;
        current.bg = effective.bg;
    }
}

struct Params<'a> {
    buffer: &'a mut Vec<u8>,
    wrote: bool,
}

impl<'a> Params<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self {
            buffer,
            wrote: false,
        }
    }

    fn separator(&mut self) {
        if self.wrote {
            self.buffer.push(b';');
        } else {
            self.buffer.extend_from_slice(b"\x1b[");
            self.wrote = true;
        }
    }

    fn text(&mut self, value: &str) {
        self.separator();
        self.buffer.extend_from_slice(value.as_bytes());
    }

    fn number(&mut self, value: u32) {
        self.separator();
        push_decimal(self.buffer, value);
    }

    fn extend_number(&mut self, value: u32) {
        self.buffer.push(b';');
        push_decimal(self.buffer, value);
    }

    fn finish(self) -> bool {
        if self.wrote {
            self.buffer.push(b'm');
        }
        self.wrote
    }
}

fn push_decimal(buffer: &mut Vec<u8>, value: u32) {
    let mut digits = [0_u8; 10];
    let mut len = 0;
    let mut rest = value;
    loop {
        digits[len] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        len += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    while len > 0 {
        len -= 1;
        buffer.push(digits[len]);
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

fn append_attr_params(params: &mut Params<'_>, attrs: Attrs, have: Attrs) {
    if attrs.bold() && !have.bold() {
        params.text("1");
    }
    if attrs.dim() && !have.dim() {
        params.text("2");
    }
    if attrs.italic() && !have.italic() {
        params.text("3");
    }
    if have.underline() == UnderlineStyle::None {
        match attrs.underline() {
            UnderlineStyle::None => {}
            UnderlineStyle::Single => params.text("4"),
            UnderlineStyle::Double => params.text("21"),
            UnderlineStyle::Curly => params.text("4:3"),
            UnderlineStyle::Dotted => params.text("4:4"),
            UnderlineStyle::Dashed => params.text("4:5"),
        }
    }
    if attrs.blink() && !have.blink() {
        params.text("5");
    }
    if attrs.reverse() && !have.reverse() {
        params.text("7");
    }
    if attrs.hidden() && !have.hidden() {
        params.text("8");
    }
    if attrs.strikethrough() && !have.strikethrough() {
        params.text("9");
    }
    if attrs.overline() && !have.overline() {
        params.text("53");
    }
}

fn append_color_params(params: &mut Params<'_>, fg: bool, color: Color) {
    let selector = if fg { 38 } else { 48 };
    match color {
        Color::Default => params.text(if fg { "39" } else { "49" }),
        Color::Indexed(index) => {
            params.number(selector);
            params.extend_number(5);
            params.extend_number(u32::from(index));
        }
        Color::Rgb(red, green, blue) => {
            params.number(selector);
            params.extend_number(2);
            params.extend_number(u32::from(red));
            params.extend_number(u32::from(green));
            params.extend_number(u32::from(blue));
        }
        Color::Token(id) => {
            params.number(selector);
            params.extend_number(5);
            params.extend_number(id % 256);
        }
    }
}

fn resolve_token_color(color: Color, palette: &[Color]) -> Color {
    match color {
        Color::Token(id) => palette.get(id as usize).copied().unwrap_or(color),
        other => other,
    }
}
