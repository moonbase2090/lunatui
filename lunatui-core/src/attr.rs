// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

const ATTR_MASK: u32 = 0xFFF;
const UNDERLINE_SHIFT: u32 = 3;
const UNDERLINE_MASK: u32 = 0x7;
const UNDERLINE_FIELD_MASK: u16 = 0x38;

/// Underline style stored in the attribute bit field (ADR 0001 §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum UnderlineStyle {
    /// No underline.
    #[default]
    None,
    /// A single underline.
    Single,
    /// A double underline.
    Double,
    /// A curly underline.
    Curly,
    /// A dotted underline.
    Dotted,
    /// A dashed underline.
    Dashed,
}

impl UnderlineStyle {
    #[must_use]
    fn to_bits(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Single => 1,
            Self::Double => 2,
            Self::Curly => 3,
            Self::Dotted => 4,
            Self::Dashed => 5,
        }
    }

    #[must_use]
    fn from_bits(bits: u32) -> Option<Self> {
        match bits {
            0 => Some(Self::None),
            1 => Some(Self::Single),
            2 => Some(Self::Double),
            3 => Some(Self::Curly),
            4 => Some(Self::Dotted),
            5 => Some(Self::Dashed),
            _ => None,
        }
    }
}

/// Text attributes stored in the low 12 bits of `meta` (ADR 0001 §4.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Attrs(u16);

impl Attrs {
    /// No attributes set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether bold is set.
    #[must_use]
    pub fn bold(self) -> bool {
        self.0 & 1 != 0
    }

    /// Returns a copy with bold set or cleared.
    #[must_use]
    pub fn set_bold(self, bold: bool) -> Self {
        self.set_bit(0, bold)
    }

    /// Whether dim is set.
    #[must_use]
    pub fn dim(self) -> bool {
        self.0 & (1 << 1) != 0
    }

    /// Returns a copy with dim set or cleared.
    #[must_use]
    pub fn set_dim(self, dim: bool) -> Self {
        self.set_bit(1, dim)
    }

    /// Whether italic is set.
    #[must_use]
    pub fn italic(self) -> bool {
        self.0 & (1 << 2) != 0
    }

    /// Returns a copy with italic set or cleared.
    #[must_use]
    pub fn set_italic(self, italic: bool) -> Self {
        self.set_bit(2, italic)
    }

    /// The underline style.
    #[must_use]
    pub fn underline(self) -> UnderlineStyle {
        match UnderlineStyle::from_bits((u32::from(self.0) >> UNDERLINE_SHIFT) & UNDERLINE_MASK) {
            Some(style) => style,
            None => UnderlineStyle::None,
        }
    }

    /// Returns a copy with `underline` set.
    #[must_use]
    pub fn set_underline(self, underline: UnderlineStyle) -> Self {
        let cleared = self.0 & !UNDERLINE_FIELD_MASK;
        let Ok(style) = u16::try_from(underline.to_bits() << UNDERLINE_SHIFT) else {
            return self;
        };
        Self(cleared | style)
    }

    /// Whether blink is set.
    #[must_use]
    pub fn blink(self) -> bool {
        self.0 & (1 << 6) != 0
    }

    /// Returns a copy with blink set or cleared.
    #[must_use]
    pub fn set_blink(self, blink: bool) -> Self {
        self.set_bit(6, blink)
    }

    /// Whether reverse video is set.
    #[must_use]
    pub fn reverse(self) -> bool {
        self.0 & (1 << 7) != 0
    }

    /// Returns a copy with reverse set or cleared.
    #[must_use]
    pub fn set_reverse(self, reverse: bool) -> Self {
        self.set_bit(7, reverse)
    }

    /// Whether hidden is set.
    #[must_use]
    pub fn hidden(self) -> bool {
        self.0 & (1 << 8) != 0
    }

    /// Returns a copy with hidden set or cleared.
    #[must_use]
    pub fn set_hidden(self, hidden: bool) -> Self {
        self.set_bit(8, hidden)
    }

    /// Whether strikethrough is set.
    #[must_use]
    pub fn strikethrough(self) -> bool {
        self.0 & (1 << 9) != 0
    }

    /// Returns a copy with strikethrough set or cleared.
    #[must_use]
    pub fn set_strikethrough(self, strikethrough: bool) -> Self {
        self.set_bit(9, strikethrough)
    }

    /// Whether overline is set.
    #[must_use]
    pub fn overline(self) -> bool {
        self.0 & (1 << 10) != 0
    }

    /// Returns a copy with overline set or cleared.
    #[must_use]
    pub fn set_overline(self, overline: bool) -> Self {
        self.set_bit(10, overline)
    }

    /// Encodes these attributes into the low 12 bits of `meta`.
    #[must_use]
    pub fn to_bits(self) -> u32 {
        u32::from(self.0)
    }

    /// Decodes the low 12 bits of `meta`. Returns `None` when bit 11 or the underline field is invalid.
    #[must_use]
    pub fn from_bits(bits: u32) -> Option<Self> {
        if bits & !ATTR_MASK != 0 {
            return None;
        }
        if (bits >> 11) & 1 != 0 {
            return None;
        }
        UnderlineStyle::from_bits((bits >> UNDERLINE_SHIFT) & UNDERLINE_MASK)?;
        let Ok(stored) = u16::try_from(bits) else {
            return None;
        };
        Some(Self(stored))
    }

    fn set_bit(self, index: u32, value: bool) -> Self {
        let mask = 1_u16 << index;
        if value {
            Self(self.0 | mask)
        } else {
            Self(self.0 & !mask)
        }
    }
}
