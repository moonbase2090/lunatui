// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Rect, Size, Style};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};

const UPPER_HALF: &str = "▀";
const KNOB: &str = "●";
const POINTER: &str = "◂";
const SWATCH_WIDTH: u16 = 4;
const HUE_COLUMNS: u16 = 4;
const MIN_WIDTH: u16 = 6;
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// The part of a [`ColorPicker`] the keyboard moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorPickerFocus {
    /// The saturation and value square.
    #[default]
    Square,
    /// The hue bar.
    Hue,
    /// The hex entry.
    Hex,
}

impl ColorPickerFocus {
    const fn next(self) -> Self {
        match self {
            Self::Square => Self::Hue,
            Self::Hue => Self::Hex,
            Self::Hex => Self::Square,
        }
    }

    const fn previous(self) -> Self {
        match self {
            Self::Square => Self::Hex,
            Self::Hue => Self::Square,
            Self::Hex => Self::Hue,
        }
    }
}

/// The color a [`ColorPicker`] shows and what the keyboard and pointer are doing to it, kept by
/// the app between frames.
///
/// The color is held as hue (degrees), saturation and value (zero to one), so dragging inside the
/// square never changes the hue, even at a gray. [`set_rgb`](Self::set_rgb) followed by
/// [`rgb`](Self::rgb) returns the same bytes. Keyboard: [`next_focus`](Self::next_focus) and
/// [`previous_focus`](Self::previous_focus) pick the part, [`nudge`](Self::nudge) moves it, and
/// on the hex entry [`type_digit`](Self::type_digit), [`backspace`](Self::backspace) and
/// [`commit_hex`](Self::commit_hex) edit and apply a typed color. Pointer: [`pointer`](Self::pointer)
/// takes a click or drag position and the area the picker was drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorPickerState {
    hue: f32,
    saturation: f32,
    value: f32,
    focus: ColorPickerFocus,
    typed: [u8; 6],
    typed_len: u8,
    editing: bool,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorPickerState {
    /// Pure red, the square focused.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hue: 0.0,
            saturation: 1.0,
            value: 1.0,
            focus: ColorPickerFocus::Square,
            typed: [0; 6],
            typed_len: 0,
            editing: false,
        }
    }

    /// A picker showing the color `red`, `green`, `blue`.
    #[must_use]
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        let mut state = Self::new();
        state.set_rgb(red, green, blue);
        state
    }

    /// A picker showing hue `hue` in degrees (wrapped into 0 up to 360), `saturation` and `value`
    /// from zero to one (clamped; NaN counts as zero).
    #[must_use]
    pub fn from_hsv(hue: f32, saturation: f32, value: f32) -> Self {
        let mut state = Self::new();
        state.set_hsv(hue, saturation, value);
        state
    }

    /// Shows the given hue, saturation and value, and drops any half-typed hex.
    pub fn set_hsv(&mut self, hue: f32, saturation: f32, value: f32) {
        let unit = |part: f32| {
            if part.is_nan() {
                0.0
            } else {
                part.clamp(0.0, 1.0)
            }
        };
        self.hue = if hue.is_finite() { wrap_hue(hue) } else { 0.0 };
        self.saturation = unit(saturation);
        self.value = unit(value);
        self.cancel_hex();
    }

    /// Shows the color `red`, `green`, `blue` and drops any half-typed hex.
    pub fn set_rgb(&mut self, red: u8, green: u8, blue: u8) {
        let (hue, saturation, value) = rgb_to_hsv(red, green, blue);
        if saturation > 0.0 && value > 0.0 {
            self.hue = hue;
        }
        self.saturation = saturation;
        self.value = value;
        self.cancel_hex();
    }

    /// The color as red, green and blue.
    #[must_use]
    pub fn rgb(&self) -> (u8, u8, u8) {
        hsv_to_rgb(self.hue, self.saturation, self.value)
    }

    /// The color as a cell color.
    #[must_use]
    pub fn color(&self) -> Color {
        let (red, green, blue) = self.rgb();
        Color::Rgb(red, green, blue)
    }

    /// Hue in degrees, from 0 up to but not including 360.
    #[must_use]
    pub const fn hue(&self) -> f32 {
        self.hue
    }

    /// Saturation from zero to one.
    #[must_use]
    pub const fn saturation(&self) -> f32 {
        self.saturation
    }

    /// Value (brightness) from zero to one.
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    /// The color as `#RRGGBB` in ASCII.
    #[must_use]
    pub fn hex(&self) -> [u8; 7] {
        let (red, green, blue) = self.rgb();
        let mut out = *b"#000000";
        for (index, byte) in [red, green, blue].into_iter().enumerate() {
            out[1 + index * 2] = HEX_DIGITS[usize::from(byte >> 4)];
            out[2 + index * 2] = HEX_DIGITS[usize::from(byte & 15)];
        }
        out
    }

    /// The part the keyboard moves.
    #[must_use]
    pub const fn focus(&self) -> ColorPickerFocus {
        self.focus
    }

    /// Moves the keyboard to `focus`, dropping a half-typed hex when it leaves the entry.
    pub fn set_focus(&mut self, focus: ColorPickerFocus) {
        if focus != ColorPickerFocus::Hex {
            self.cancel_hex();
        }
        self.focus = focus;
    }

    /// Moves the keyboard to the next part: square, hue, hex, and round again.
    pub fn next_focus(&mut self) {
        self.set_focus(self.focus.next());
    }

    /// Moves the keyboard to the previous part.
    pub fn previous_focus(&mut self) {
        self.set_focus(self.focus.previous());
    }

    /// Moves the focused part by `columns` and `rows` steps, right and down positive.
    ///
    /// On the square a step is one hundredth: columns change saturation, and rows change value,
    /// with up making it brighter. On the hue bar a step is one degree and either direction
    /// moves it, wrapping past 0 and 360. The hex entry does not move.
    pub fn nudge(&mut self, columns: i16, rows: i16) {
        match self.focus {
            ColorPickerFocus::Square => {
                self.saturation = (self.saturation + f32::from(columns) * 0.01).clamp(0.0, 1.0);
                self.value = (self.value - f32::from(rows) * 0.01).clamp(0.0, 1.0);
            }
            ColorPickerFocus::Hue => {
                let delta = f32::from(columns.saturating_add(rows));
                self.hue = wrap_hue(self.hue + delta);
            }
            ColorPickerFocus::Hex => {}
        }
    }

    /// Whether hex digits are being typed.
    #[must_use]
    pub const fn is_editing(&self) -> bool {
        self.editing
    }

    /// The digits typed so far, without the `#`.
    #[must_use]
    pub fn typed(&self) -> &[u8] {
        &self.typed[..usize::from(self.typed_len)]
    }

    /// Types one hex digit. Anything else, and a seventh digit, is ignored. Returns whether the
    /// character was taken.
    pub fn type_digit(&mut self, digit: char) -> bool {
        if self.focus != ColorPickerFocus::Hex || !digit.is_ascii_hexdigit() {
            return false;
        }
        let Some(slot) = self.typed.get_mut(usize::from(self.typed_len)) else {
            return false;
        };
        *slot = u8::try_from(digit.to_ascii_uppercase()).unwrap_or(b'0');
        self.typed_len += 1;
        self.editing = true;
        true
    }

    /// Removes the last typed digit. Returns whether there was one.
    pub fn backspace(&mut self) -> bool {
        if self.typed_len == 0 {
            return false;
        }
        self.typed_len -= 1;
        self.editing = self.typed_len > 0;
        true
    }

    /// Drops the typed digits and shows the color again.
    pub fn cancel_hex(&mut self) {
        self.typed_len = 0;
        self.editing = false;
    }

    /// Applies the typed digits when they are three (`F80` for `FF8800`) or six. Returns whether
    /// a color was applied; otherwise the digits stay for more typing.
    pub fn commit_hex(&mut self) -> bool {
        let digits = self.typed();
        let byte = |high: u8, low: u8| (hex_value(high) << 4) | hex_value(low);
        let rgb = match *digits {
            [r, g, b] => Some((byte(r, r), byte(g, g), byte(b, b))),
            [r1, r2, g1, g2, b1, b2] => Some((byte(r1, r2), byte(g1, g2), byte(b1, b2))),
            _ => None,
        };
        let Some((red, green, blue)) = rgb else {
            return false;
        };
        self.set_rgb(red, green, blue);
        true
    }

    /// Sets the part under `(column, row)` from a click or drag in a picker drawn in `area`, and
    /// focuses it. Returns whether the position was on the square or the hue bar. A position
    /// outside both is ignored. Dragging past an edge of the part clamps to that edge, so call it
    /// with the position the drag began on to keep a drag in the square.
    pub fn pointer(&mut self, column: u16, row: u16, area: Rect) -> bool {
        let layout = Layout::new(area);
        if let Some(square) = layout.square.filter(|rect| contains(*rect, column, row)) {
            self.set_focus(ColorPickerFocus::Square);
            self.saturation = fraction(column - square.x, square.width, 1);
            self.value = 1.0 - fraction(row - square.y, square.height, 2);
            return true;
        }
        if let Some(bar) = layout.bar.filter(|rect| contains(*rect, column, row)) {
            self.set_focus(ColorPickerFocus::Hue);
            self.hue = hue_at(row - bar.y, bar.height, 0);
            return true;
        }
        if contains(layout.hex, column, row) {
            self.set_focus(ColorPickerFocus::Hex);
        }
        false
    }
}

/// A color picker: a saturation and value square beside a hue bar, with a swatch, the hex code and
/// the numbers below.
///
/// The square and the bar are drawn with `▀` half blocks, two color samples per cell, so a picker
/// four rows tall resolves eight steps of value. The current color is marked with `●` in the
/// square and `◂` beside the bar. The last row is a four-cell swatch of the color, `#RRGGBB`,
/// and as much of `H`, `S` and `V` as fits; the part the keyboard moves is in the focus ring
/// color, and a half-typed hex shows its digits with `_` for those still to come.
///
/// The picker needs at least six columns and two rows; in less it draws what fits. State lives in
/// a [`ColorPickerState`]. [`square_area`](Self::square_area), [`hue_area`](Self::hue_area) and
/// [`hex_area`](Self::hex_area) give the cells for hit testing. A built picker draws without
/// allocating.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorPicker<'s> {
    view: Option<&'s ColorPickerState>,
    state: WidgetState,
}

impl Default for ColorPicker<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'s> ColorPicker<'s> {
    /// A picker showing pure red.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            view: None,
            state: WidgetState::Normal,
        }
    }

    /// Draws the color, focus and typed digits in `state`.
    #[must_use]
    pub const fn view(mut self, state: &'s ColorPickerState) -> Self {
        self.view = Some(state);
        self
    }

    /// Interaction state: disabled dims everything. Focus within the picker comes from
    /// [`ColorPickerState::focus`], and shows only when the widget is [`WidgetState::Focused`].
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The cells of the saturation and value square when drawn in `area`, if any fit.
    #[must_use]
    pub fn square_area(area: Rect) -> Rect {
        Layout::new(area).square.unwrap_or_else(|| empty(area))
    }

    /// The cells of the hue bar when drawn in `area`, if any fit.
    #[must_use]
    pub fn hue_area(area: Rect) -> Rect {
        Layout::new(area).bar.unwrap_or_else(|| empty(area))
    }

    /// The cells of the swatch and hex row when drawn in `area`.
    #[must_use]
    pub fn hex_area(area: Rect) -> Rect {
        Layout::new(area).hex
    }
}

impl Widget for ColorPicker<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let fallback = ColorPickerState::new();
        let view = self.view.unwrap_or(&fallback);
        let layout = Layout::new(area);
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let dim = Attrs::new().set_dim(disabled);
        if let Some(square) = layout.square {
            draw_square(cx, square, view, dim);
        }
        if let Some(bar) = layout.bar {
            draw_bar(cx, bar, view, dim);
        }
        draw_readout(cx, layout.hex, view, focused, disabled);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width.min(24), avail.height.min(9))
    }
}

struct Layout {
    square: Option<Rect>,
    bar: Option<Rect>,
    hex: Rect,
}

impl Layout {
    fn new(area: Rect) -> Self {
        let area = clip_to_coordinates(area);
        let hex_row = area.y.saturating_add(area.height.saturating_sub(1));
        let hex = Rect::new(area.x, hex_row, area.width, area.height.min(1));
        let rows = area.height.saturating_sub(1);
        if rows == 0 || area.width < MIN_WIDTH {
            return Self {
                square: None,
                bar: None,
                hex,
            };
        }
        let square_width = area.width - HUE_COLUMNS;
        Self {
            square: Some(Rect::new(area.x, area.y, square_width, rows)),
            bar: Some(Rect::new(area.x + square_width + 1, area.y, 2, rows)),
            hex,
        }
    }
}

const fn empty(area: Rect) -> Rect {
    Rect::new(area.x, area.y, 0, 0)
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column - area.x < area.width && row >= area.y && row - area.y < area.height
}

fn fraction(index: u16, count: u16, samples: u16) -> f32 {
    let last = count.saturating_mul(samples).saturating_sub(1);
    if last == 0 {
        return 0.0;
    }
    let position = index.saturating_mul(samples);
    f32::from(position.min(last)) / f32::from(last)
}

fn hue_at(index: u16, count: u16, sub: u16) -> f32 {
    let last = count.saturating_mul(2).saturating_sub(1);
    if last == 0 {
        return 0.0;
    }
    let position = index.saturating_mul(2).saturating_add(sub).min(last);
    wrap_hue(f32::from(position) / f32::from(last) * 359.0)
}

fn draw_square(cx: &mut RenderCx<'_>, square: Rect, view: &ColorPickerState, dim: Attrs) {
    let hue = view.hue;
    for row in 0..square.height {
        for column in 0..square.width {
            let saturation = fraction(column, square.width, 1);
            let top = 1.0 - sub_fraction(row, 0, square.height);
            let top_color = rgb_color(hsv_to_rgb(hue, saturation, top));
            let bottom_value = 1.0 - sub_fraction(row, 1, square.height);
            let bottom_color = rgb_color(hsv_to_rgb(hue, saturation, bottom_value));
            let style = Style::new().fg(top_color).bg(bottom_color).with_attrs(dim);
            cx.put_str(square.x + column, square.y + row, UPPER_HALF, style);
        }
    }
    let samples = square.height.saturating_mul(2);
    let knob_column = round_index(view.saturation, square.width);
    let knob_sample = round_index(1.0 - view.value, samples);
    let knob_row = knob_sample / 2;
    let top_half = knob_sample.is_multiple_of(2);
    let (column, row) = (square.x + knob_column, square.y + knob_row);
    let under = hsv_to_rgb(
        hue,
        fraction(knob_column, square.width, 1),
        1.0 - sub_fraction(knob_row, u16::from(!top_half), square.height),
    );
    let ink = contrast(under);
    let style = Style::new()
        .fg(ink)
        .bg(rgb_color(under))
        .with_attrs(dim.set_bold(true));
    cx.put_str(column, row, KNOB, style);
}

fn sub_fraction(row: u16, sub: u16, height: u16) -> f32 {
    fraction(
        row.saturating_mul(2).saturating_add(sub),
        height.saturating_mul(2),
        1,
    )
}

fn round_index(unit: f32, count: u16) -> u16 {
    let last = count.saturating_sub(1);
    let scaled = (unit.clamp(0.0, 1.0) * f32::from(last)).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is rounded and clamped into the target range first"
    )]
    let index = scaled as u16;
    index.min(last)
}

fn draw_bar(cx: &mut RenderCx<'_>, bar: Rect, view: &ColorPickerState, dim: Attrs) {
    for row in 0..bar.height {
        let top = rgb_color(hsv_to_rgb(hue_at(row, bar.height, 0), 1.0, 1.0));
        let bottom = rgb_color(hsv_to_rgb(hue_at(row, bar.height, 1), 1.0, 1.0));
        let style = Style::new().fg(top).bg(bottom).with_attrs(dim);
        for column in 0..bar.width {
            cx.put_str(bar.x + column, bar.y + row, UPPER_HALF, style);
        }
    }
    let samples = bar.height.saturating_mul(2);
    let sample = round_index(view.hue / 359.0, samples);
    let pointer_style = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0))
        .with_attrs(dim);
    cx.put_str(
        bar.x + bar.width,
        bar.y + sample / 2,
        POINTER,
        pointer_style,
    );
}

fn draw_readout(
    cx: &mut RenderCx<'_>,
    hex: Rect,
    view: &ColorPickerState,
    focused: bool,
    disabled: bool,
) {
    if hex.width == 0 || hex.height == 0 {
        return;
    }
    let dim = Attrs::new().set_dim(disabled);
    let text = cx.token(Token::TextPrimary);
    let ring = cx.token(Token::FocusRing);
    let ground = cx.token(Token::Surface0);
    let plain = Style::new().fg(text).bg(ground).with_attrs(dim);
    let marked = |part| {
        if focused && view.focus == part {
            Style::new()
                .fg(ring)
                .bg(ground)
                .with_attrs(Attrs::new().set_bold(true).set_dim(disabled))
        } else {
            plain
        }
    };
    let mut x = hex.x;
    let end = hex.x.saturating_add(hex.width);
    let swatch = Style::new().bg(view.color()).with_attrs(dim);
    let swatch_cells = SWATCH_WIDTH.min(end.saturating_sub(x));
    cx.fill(Rect::new(x, hex.y, swatch_cells, 1), swatch, ' ');
    x = x.saturating_add(swatch_cells + 1);
    let mut buffer = [0_u8; 7];
    if view.editing {
        buffer[0] = b'#';
        for index in 0..6 {
            buffer[index + 1] = view.typed().get(index).copied().unwrap_or(b'_');
        }
    } else {
        buffer = view.hex();
    }
    put_ascii(
        cx,
        &mut x,
        end,
        hex.y,
        &buffer,
        marked(ColorPickerFocus::Hex),
    );
    x = x.saturating_add(1);
    let hue = u32::from(round_degrees(view.hue));
    let saturation = percent(view.saturation);
    let value = percent(view.value);
    let square = marked(ColorPickerFocus::Square);
    let hue_style = marked(ColorPickerFocus::Hue);
    for (label, number, style) in [
        (b'H', hue, hue_style),
        (b'S', saturation, square),
        (b'V', value, square),
    ] {
        let mut text = [0_u8; 5];
        let used = field(label, number, &mut text);
        if x.saturating_add(u16::try_from(used).unwrap_or(0)) > end {
            break;
        }
        put_ascii(cx, &mut x, end, hex.y, &text[..used], style);
        x = x.saturating_add(1);
    }
}

fn put_ascii(cx: &mut RenderCx<'_>, x: &mut u16, end: u16, y: u16, bytes: &[u8], style: Style) {
    for &byte in bytes {
        if *x >= end {
            return;
        }
        let mut one = [0_u8; 4];
        let ch = char::from(byte).encode_utf8(&mut one);
        cx.put_str(*x, y, ch, style);
        *x += 1;
    }
}

fn field(label: u8, number: u32, out: &mut [u8; 5]) -> usize {
    out[0] = label;
    let number = number.min(999);
    let digits = if number >= 100 {
        3
    } else if number >= 10 {
        2
    } else {
        1
    };
    let mut rest = number;
    for slot in (1..=digits).rev() {
        out[slot] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        rest /= 10;
    }
    digits + 1
}

fn round_degrees(hue: f32) -> u16 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is rounded and clamped into the target range first"
    )]
    let degrees = hue.round().clamp(0.0, 359.0) as u16;
    degrees
}

fn percent(unit: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is rounded and clamped into the target range first"
    )]
    let percent = (unit.clamp(0.0, 1.0) * 100.0).round() as u32;
    percent
}

fn rgb_color((red, green, blue): (u8, u8, u8)) -> Color {
    Color::Rgb(red, green, blue)
}

fn contrast((red, green, blue): (u8, u8, u8)) -> Color {
    let luma = 299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue);
    if luma > 128_000 {
        Color::Rgb(0, 0, 0)
    } else {
        Color::Rgb(255, 255, 255)
    }
}

fn hex_value(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'A'..=b'F' => digit - b'A' + 10,
        b'a'..=b'f' => digit - b'a' + 10,
        _ => 0,
    }
}

fn wrap_hue(hue: f32) -> f32 {
    let wrapped = hue.rem_euclid(360.0);
    if wrapped >= 360.0 { 0.0 } else { wrapped }
}

fn rgb_to_hsv(red: u8, green: u8, blue: u8) -> (f32, f32, f32) {
    let (r, g, b) = (
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    (wrap_hue(hue), saturation, max)
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> (u8, u8, u8) {
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    let sector = wrap_hue(hue) / 60.0;
    let chroma = value * saturation;
    let second = chroma * (1.0 - ((sector % 2.0) - 1.0).abs());
    let offset = value - chroma;
    let (r, g, b) = match sector {
        s if s < 1.0 => (chroma, second, 0.0),
        s if s < 2.0 => (second, chroma, 0.0),
        s if s < 3.0 => (0.0, chroma, second),
        s if s < 4.0 => (0.0, second, chroma),
        s if s < 5.0 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let byte = |channel: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is rounded and clamped into the target range first"
        )]
        let byte = ((channel + offset) * 255.0).round().clamp(0.0, 255.0) as u8;
        byte
    };
    (byte(r), byte(g), byte(b))
}
