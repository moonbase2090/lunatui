// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

const KIND_SHIFT: u32 = 30;
const RESERVED_MASK: u32 = 0x3F00_0000;
const PAYLOAD_MASK: u32 = 0x00FF_FFFF;

const KIND_DEFAULT: u32 = 0;
const KIND_INDEXED: u32 = 1 << KIND_SHIFT;
const KIND_RGB: u32 = 2 << KIND_SHIFT;
const KIND_TOKEN: u32 = 3 << KIND_SHIFT;

/// A foreground or background color in the packed cell format (ADR 0001 §4.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Color {
    /// The terminal's default color for this role (foreground or background).
    #[default]
    Default,
    /// A palette index: 0–15 are the ANSI colors, 16–255 are the 256-color cube.
    Indexed(u8),
    /// An RGB triplet.
    Rgb(u8, u8, u8),
    /// A theme token ID, resolved at encode time (PRD D5).
    Token(u32),
}

impl Color {
    /// Encodes this color into the 32-bit `fg` or `bg` field. Reserved bits are zero.
    #[must_use]
    pub fn to_bits(self) -> u32 {
        match self {
            Self::Default => KIND_DEFAULT,
            Self::Indexed(index) => KIND_INDEXED | u32::from(index),
            Self::Rgb(red, green, blue) => {
                KIND_RGB | u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue)
            }
            Self::Token(id) => KIND_TOKEN | (id & PAYLOAD_MASK),
        }
    }

    /// Decodes a `fg` or `bg` field. Returns `None` when reserved bits are set or the kind is invalid.
    #[must_use]
    pub fn from_bits(bits: u32) -> Option<Self> {
        if bits & RESERVED_MASK != 0 {
            return None;
        }
        match bits & (3 << KIND_SHIFT) {
            KIND_DEFAULT if bits == KIND_DEFAULT => Some(Self::Default),
            KIND_INDEXED if bits & 0x00FF_FF00 == 0 => Some(Self::Indexed((bits & 0xFF) as u8)),
            KIND_RGB => Some(Self::Rgb(
                ((bits >> 16) & 0xFF) as u8,
                ((bits >> 8) & 0xFF) as u8,
                (bits & 0xFF) as u8,
            )),
            KIND_TOKEN => Some(Self::Token(bits & PAYLOAD_MASK)),
            _ => None,
        }
    }
}
