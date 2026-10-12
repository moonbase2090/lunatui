// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::attr::Attrs;
use crate::color::Color;
use crate::grapheme::GlyphSlot;
use crate::style::Style;

const WIDTH_SHIFT: u32 = 12;
const FLAGS_SHIFT: u32 = 14;
const EXT_SHIFT: u32 = 16;
const WIDTH_MASK: u32 = 0x3;
const FLAGS_MASK: u32 = 0x3;
const EXT_MASK: u32 = 0xFFFF;

/// Terminal column width stored in `meta` (ADR 0001 §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CellWidth {
    /// A continuation cell (width 0).
    #[default]
    Continuation,
    /// A single-column cell.
    One,
    /// The lead cell of a width-2 grapheme.
    Two,
}

impl CellWidth {
    #[must_use]
    fn to_bits(self) -> u32 {
        match self {
            Self::Continuation => 0,
            Self::One => 1,
            Self::Two => 2,
        }
    }

    #[must_use]
    fn from_bits(bits: u32) -> Option<Self> {
        match bits {
            0 => Some(Self::Continuation),
            1 => Some(Self::One),
            2 => Some(Self::Two),
            _ => None,
        }
    }
}

/// Encoder flags in `meta` bits 14–15 (ADR 0001 §4.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellFlags {
    /// The encoder leaves this cell unchanged (image placement).
    pub skip: bool,
    /// Reserved for future use.
    pub protected: bool,
}

impl CellFlags {
    #[must_use]
    fn to_bits(self) -> u32 {
        u32::from(self.skip) | (u32::from(self.protected) << 1)
    }

    #[must_use]
    fn from_bits(bits: u32) -> Option<Self> {
        if bits & !FLAGS_MASK != 0 {
            None
        } else {
            Some(Self {
                skip: bits & 1 != 0,
                protected: (bits >> 1) & 1 != 0,
            })
        }
    }
}

/// The `meta` field: attributes, width, flags and an extension-table index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Meta {
    /// Text attributes.
    pub attrs: Attrs,
    /// Stored terminal width.
    pub width: CellWidth,
    /// Encoder flags.
    pub flags: CellFlags,
    /// Index into the frame `ExtTable` (0 = none).
    pub ext: u16,
}

impl Meta {
    /// Encodes `meta` for a [`Cell`].
    #[must_use]
    pub fn to_bits(self) -> u32 {
        let attrs = self.attrs.to_bits();
        let width = self.width.to_bits() << WIDTH_SHIFT;
        let flags = self.flags.to_bits() << FLAGS_SHIFT;
        let ext = u32::from(self.ext) << EXT_SHIFT;
        attrs | width | flags | ext
    }

    /// Decodes the `meta` field. Returns `None` when any subfield is invalid.
    #[must_use]
    pub fn from_bits(bits: u32) -> Option<Self> {
        let attrs = Attrs::from_bits(bits & 0xFFF)?;
        let width = CellWidth::from_bits((bits >> WIDTH_SHIFT) & WIDTH_MASK)?;
        let flags = CellFlags::from_bits((bits >> FLAGS_SHIFT) & FLAGS_MASK)?;
        let ext = ((bits >> EXT_SHIFT) & EXT_MASK) as u16;
        Some(Self {
            attrs,
            width,
            flags,
            ext,
        })
    }
}

/// A 16-byte terminal cell (ADR 0001 §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C, align(16))]
pub struct Cell {
    glyph: u32,
    fg: u32,
    bg: u32,
    meta: u32,
}

impl Cell {
    /// A blank cell: ASCII space, default colors, width 1 (ADR 0001 §4.2).
    pub const BLANK: Self = Self {
        glyph: 0x20,
        fg: 0,
        bg: 0,
        meta: 0x1000,
    };

    /// Builds a cell from its packed fields.
    #[must_use]
    pub fn from_parts(glyph: GlyphSlot, style: Style, meta: Meta) -> Self {
        Self {
            glyph: glyph.bits(),
            fg: style.fg.to_bits(),
            bg: style.bg.to_bits(),
            meta: meta.to_bits(),
        }
    }

    /// The glyph slot.
    #[must_use]
    pub fn glyph(self) -> GlyphSlot {
        GlyphSlot::from_bits(self.glyph)
    }

    /// Foreground, background and attributes.
    #[must_use]
    pub fn style(self) -> Style {
        Style {
            fg: color_or_default(self.fg),
            bg: color_or_default(self.bg),
            attrs: Attrs::from_bits(self.meta & 0xFFF).unwrap_or_default(),
        }
    }

    /// Width, flags and extension index.
    #[must_use]
    pub fn meta(self) -> Meta {
        Meta::from_bits(self.meta).unwrap_or_default()
    }

    /// The cell as one 128-bit word (glyph, fg, bg, meta).
    #[must_use]
    pub fn to_bits(self) -> u128 {
        u128::from(self.glyph)
            | u128::from(self.fg) << 32
            | u128::from(self.bg) << 64
            | u128::from(self.meta) << 96
    }

    /// Decodes a cell from a 128-bit word. Returns `None` when colors or `meta` are invalid.
    #[must_use]
    pub fn from_bits(bits: u128) -> Option<Self> {
        let glyph = field(bits, 0);
        let fg = field(bits, 32);
        let bg = field(bits, 64);
        let meta = field(bits, 96);
        Color::from_bits(fg)?;
        Color::from_bits(bg)?;
        Meta::from_bits(meta)?;
        Some(Self {
            glyph,
            fg,
            bg,
            meta,
        })
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::BLANK
    }
}

fn color_or_default(bits: u32) -> Color {
    match Color::from_bits(bits) {
        Some(color) => color,
        None => Color::Default,
    }
}

fn field(bits: u128, shift: u32) -> u32 {
    let mask = u128::from(u32::MAX);
    let Ok(value) = u32::try_from((bits >> shift) & mask) else {
        return 0;
    };
    value
}
