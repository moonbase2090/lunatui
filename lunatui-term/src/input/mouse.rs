// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::keys::{Modifiers, decimal};
use crate::CellPixels;

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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

/// Where a pixel-precise pointer event fell inside its cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubCell {
    x: u16,
    y: u16,
    cell: CellPixels,
}

impl SubCell {
    /// The pixel column inside the cell, from 0 to one less than the cell width.
    #[must_use]
    pub const fn x(self) -> u16 {
        self.x
    }

    /// The pixel row inside the cell, from 0 to one less than the cell height.
    #[must_use]
    pub const fn y(self) -> u16 {
        self.y
    }

    /// The size of a cell in pixels.
    #[must_use]
    pub const fn cell(self) -> CellPixels {
        self.cell
    }

    /// How far across the cell the pixel's centre is, above 0.0 and below 1.0.
    #[must_use]
    pub fn across(self) -> f64 {
        (f64::from(self.x) + 0.5) / f64::from(self.cell.width())
    }

    /// How far down the cell the pixel's centre is, above 0.0 and below 1.0.
    #[must_use]
    pub fn down(self) -> f64 {
        (f64::from(self.y) + 0.5) / f64::from(self.cell.height())
    }
}

/// One decoded pointer event.
///
/// Downstream code cannot list every field without `..`, because fields can be added:
///
/// ```compile_fail,E0638
/// use lunatui_term::MouseEvent;
///
/// fn destructures(value: MouseEvent) {
///     let MouseEvent { kind: _, column: _, row: _, modifiers: _, unit: _, inside: _ } = value;
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
    /// For an event converted from pixels by [`in_cells`](Self::in_cells), where inside the cell
    /// the pointer is.
    pub inside: Option<SubCell>,
}

impl MouseEvent {
    /// A cell-unit event with no modifiers at a zero-based column and row.
    #[must_use]
    pub const fn new(kind: MouseKind, column: u16, row: u16) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers: Modifiers::new(),
            unit: MouseUnit::Cells,
            inside: None,
        }
    }

    /// This event in cells. A pixel report becomes the cell its pixel falls in, with the pixel's
    /// place inside that cell in [`inside`](Self::inside). A cell report is returned unchanged.
    #[must_use]
    pub const fn in_cells(self, cell: CellPixels) -> Self {
        if !matches!(self.unit, MouseUnit::Pixels) {
            return self;
        }
        let (width, height) = (cell.width(), cell.height());
        Self {
            column: self.column / width,
            row: self.row / height,
            unit: MouseUnit::Cells,
            inside: Some(SubCell {
                x: self.column % width,
                y: self.row % height,
                cell,
            }),
            ..self
        }
    }

    /// Sets the modifiers carried in the button code.
    #[must_use]
    pub const fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Sets whether the position is in cells or pixels.
    #[must_use]
    pub const fn unit(mut self, unit: MouseUnit) -> Self {
        self.unit = unit;
        self
    }
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
        inside: None,
    })
}

pub(crate) fn mouse_from_x10(bytes: [u8; 3], pixels: bool) -> Option<MouseEvent> {
    let code = u32::from(bytes[0].saturating_sub(32));
    let column = u16::from(bytes[1].saturating_sub(32)).saturating_sub(1);
    let row = u16::from(bytes[2].saturating_sub(32)).saturating_sub(1);
    let (kind, modifiers) = decode_cb(code, false)?;
    Some(MouseEvent {
        kind,
        column,
        row,
        modifiers,
        unit: unit(pixels),
        inside: None,
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
