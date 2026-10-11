// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::keys::{Modifiers, decimal};

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    /// The primary button.
    Left,
    /// The middle button.
    Middle,
    /// The secondary button.
    Right,
    /// A button number outside the three main buttons.
    Extra(u8),
}

/// Wheel direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scroll {
    /// Wheel away from the user.
    Up,
    /// Wheel toward the user.
    Down,
    /// Wheel to the left.
    Left,
    /// Wheel to the right.
    Right,
}

/// What the pointer did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseKind {
    /// A button went down.
    Down(MouseButton),
    /// A button went up.
    Up(MouseButton),
    /// A button is held while the pointer moves.
    Drag(MouseButton),
    /// The pointer moved with no button held.
    Move,
    /// The wheel moved.
    Scroll(Scroll),
}

/// Cell coordinates or SGR pixel coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MouseUnit {
    /// `CSI <` values are cells. This is mode 1006.
    #[default]
    Cells,
    /// `CSI <` values are pixels. This is mode 1016.
    Pixels,
}

/// One decoded pointer event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    /// Button, motion or wheel.
    pub kind: MouseKind,
    /// Zero-based column.
    pub column: u16,
    /// Zero-based row.
    pub row: u16,
    /// Shift, alt and control carried in the button code.
    pub modifiers: Modifiers,
    /// Cells or pixels, from the parser's mode.
    pub unit: MouseUnit,
}

pub(crate) fn mouse_from_sgr(body: &str, release: bool, pixels: bool) -> Option<MouseEvent> {
    let mut parts = body.split(';');
    let code = decimal(parts.next()?)?;
    let column = narrow_u16(decimal(parts.next()?)?).saturating_sub(1);
    let row = narrow_u16(decimal(parts.next()?)?).saturating_sub(1);
    let (kind, modifiers) = decode_cb(code, release)?;
    Some(MouseEvent {
        kind,
        column,
        row,
        modifiers,
        unit: unit(pixels),
    })
}

pub(crate) fn mouse_from_x10(raw: [u8; 3], pixels: bool) -> Option<MouseEvent> {
    let code = u32::from(raw[0].saturating_sub(32));
    let column = u16::from(raw[1].saturating_sub(32)).saturating_sub(1);
    let row = u16::from(raw[2].saturating_sub(32)).saturating_sub(1);
    let (kind, modifiers) = decode_cb(code, false)?;
    Some(MouseEvent {
        kind,
        column,
        row,
        modifiers,
        unit: unit(pixels),
    })
}

pub(crate) fn encode_mouse(event: &MouseEvent) -> Vec<u8> {
    let (mut code, release) = pack_kind(event.kind);
    if event.modifiers.is_shift() {
        code |= 4;
    }
    if event.modifiers.is_alt() {
        code |= 8;
    }
    if event.modifiers.is_control() {
        code |= 16;
    }
    let mark = if release { 'm' } else { 'M' };
    let column = u32::from(event.column).saturating_add(1);
    let row = u32::from(event.row).saturating_add(1);
    format!("\u{1b}[<{code};{column};{row}{mark}").into_bytes()
}

fn unit(pixels: bool) -> MouseUnit {
    if pixels {
        MouseUnit::Pixels
    } else {
        MouseUnit::Cells
    }
}

fn narrow_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn decode_cb(code: u32, release: bool) -> Option<(MouseKind, Modifiers)> {
    let button = (code & 0b11) | ((code & 0b1100_0000) >> 4);
    let drag = code & 0b10_0000 != 0;
    let mut kind = match (button, drag) {
        (0, false) => MouseKind::Down(MouseButton::Left),
        (1, false) => MouseKind::Down(MouseButton::Middle),
        (2, false) => MouseKind::Down(MouseButton::Right),
        (0, true) => MouseKind::Drag(MouseButton::Left),
        (1, true) => MouseKind::Drag(MouseButton::Middle),
        (2, true) => MouseKind::Drag(MouseButton::Right),
        (3, false) => MouseKind::Up(MouseButton::Left),
        (3..=5, true) => MouseKind::Move,
        (4, false) => MouseKind::Scroll(Scroll::Up),
        (5, false) => MouseKind::Scroll(Scroll::Down),
        (6, false) => MouseKind::Scroll(Scroll::Left),
        (7, false) => MouseKind::Scroll(Scroll::Right),
        (number, true) => MouseKind::Drag(MouseButton::Extra(narrow_button(number)?)),
        (number, false) => MouseKind::Down(MouseButton::Extra(narrow_button(number)?)),
    };
    if release && let MouseKind::Down(button) = kind {
        kind = MouseKind::Up(button);
    }
    Some((kind, mouse_modifiers(code)))
}

fn narrow_button(number: u32) -> Option<u8> {
    u8::try_from(number).ok()
}

fn mouse_modifiers(code: u32) -> Modifiers {
    let mut modifiers = Modifiers::new();
    if code & 0b100 != 0 {
        modifiers = modifiers.shift();
    }
    if code & 0b1000 != 0 {
        modifiers = modifiers.alt();
    }
    if code & 0b1_0000 != 0 {
        modifiers = modifiers.control();
    }
    modifiers
}

fn pack_kind(kind: MouseKind) -> (u32, bool) {
    match kind {
        MouseKind::Down(button) => (pack_button(button_number(button), false), false),
        MouseKind::Up(button) => (pack_button(button_number(button), false), true),
        MouseKind::Drag(button) => (pack_button(button_number(button), true), false),
        MouseKind::Move => (pack_button(3, true), false),
        MouseKind::Scroll(Scroll::Up) => (pack_button(4, false), false),
        MouseKind::Scroll(Scroll::Down) => (pack_button(5, false), false),
        MouseKind::Scroll(Scroll::Left) => (pack_button(6, false), false),
        MouseKind::Scroll(Scroll::Right) => (pack_button(7, false), false),
    }
}

fn button_number(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::Extra(number) => u32::from(number),
    }
}

fn pack_button(number: u32, drag: bool) -> u32 {
    let low = number & 0b11;
    let high = (number & 0b1100) << 4;
    let mut code = low | high;
    if drag {
        code |= 32;
    }
    code
}
