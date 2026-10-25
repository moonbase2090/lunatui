// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Allocation-free glyph lookup for a canonical 2×4 cell mask.
//!
//! Bit 0 is the top-left pixel, bit 1 the top-right, then subsequent rows
//! continue left to right through bit 7 at the bottom-right. The application
//! probes font support at startup and retains the selected [`GlyphSet`].

#[path = "subcell_octants.rs"]
mod octants;

/// Font support established by the application at startup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphCapabilities {
    /// All octant glyphs and their legacy block aliases render correctly.
    pub octants: bool,
    /// Braille patterns render as single cells.
    pub braille: bool,
    /// Half and quarter block characters render as single cells.
    pub blocks: bool,
}

/// A glyph tier selected once from known font capabilities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlyphSet {
    /// Exact 2×4 coverage with Unicode octants and block aliases.
    Octants,
    /// Exact 2×4 dot positions using Unicode braille patterns.
    Braille,
    /// Quarter-cell coverage, retaining only completely filled quadrants.
    Blocks,
    /// Space for an empty mask and `#` for any nonempty mask.
    #[default]
    Ascii,
}

impl GlyphSet {
    /// Choose the highest supported tier. No environment or font I/O occurs.
    #[must_use]
    pub const fn select(capabilities: GlyphCapabilities) -> Self {
        if capabilities.octants {
            Self::Octants
        } else if capabilities.braille {
            Self::Braille
        } else if capabilities.blocks {
            Self::Blocks
        } else {
            Self::Ascii
        }
    }

    /// Convert a row-major 2×4 coverage mask into one display character.
    #[must_use]
    pub fn glyph(self, mask: u8) -> char {
        match self {
            Self::Octants => octants::OCTANTS[usize::from(mask)],
            Self::Braille => {
                if mask == 0 {
                    return ' ';
                }
                let dots = (mask & 0x01)
                    | ((mask & 0x02) << 2)
                    | ((mask & 0x04) >> 1)
                    | ((mask & 0x08) << 1)
                    | ((mask & 0x10) >> 2)
                    | (mask & 0x20)
                    | (mask & 0x40)
                    | (mask & 0x80);
                // Every 8-bit dot mask is a valid scalar in U+2800..U+28FF.
                char::from_u32(0x2800 + u32::from(dots)).unwrap_or(' ')
            }
            Self::Blocks => {
                const QUADRANTS: [char; 16] = [
                    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
                ];
                let quadrants = u8::from(mask & 0x05 == 0x05)
                    | (u8::from(mask & 0x0a == 0x0a) << 1)
                    | (u8::from(mask & 0x50 == 0x50) << 2)
                    | (u8::from(mask & 0xa0 == 0xa0) << 3);
                QUADRANTS[usize::from(quadrants)]
            }
            Self::Ascii => {
                if mask == 0 {
                    ' '
                } else {
                    '#'
                }
            }
        }
    }
}

/// Horizontal fill from the left, clamped to 0..=8 eighths of a cell.
///
/// The caller must select ASCII separately if block glyphs are unsupported.
#[must_use]
pub const fn horizontal_eighths(filled: u8) -> char {
    const GLYPHS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
    GLYPHS[if filled > 8 { 8 } else { filled as usize }]
}

/// Vertical fill from the bottom, clamped to 0..=8 eighths of a cell.
#[must_use]
pub const fn vertical_eighths(filled: u8) -> char {
    const GLYPHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    GLYPHS[if filled > 8 { 8 } else { filled as usize }]
}
