// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use ratatui::buffer::{Buffer, Cell};
use ratatui::style::Color;

/// Tag bit of the glyph slot: set when the glyph is a cluster ID rather than a scalar value.
pub const CLUSTER_TAG: u32 = 1 << 31;

/// Glyph slot value of the trailing cell of a wide grapheme.
pub const CONTINUATION: u32 = 0x11_0000;

const COLOR_INDEXED: u32 = 1 << 30;
const COLOR_RGB: u32 = 2 << 30;
const ATTRIBUTE_BITS: u32 = 0xFFF;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// A 16-byte cell laid out as ADR 0001 proposes: glyph slot, foreground, background, meta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct PackedCell {
    /// Inline Unicode scalar value, or [`CLUSTER_TAG`] plus a cluster ID, or [`CONTINUATION`].
    pub glyph: u32,
    /// Two-bit color kind (default, indexed, RGB) and a 24-bit payload.
    pub fg: u32,
    /// Same encoding as `fg`.
    pub bg: u32,
    /// Attribute bits (the low 12).
    pub meta: u32,
}

impl PackedCell {
    /// The cell as one 128-bit word: glyph in the low 32 bits, then fg, bg and meta.
    #[must_use]
    pub fn to_bits(self) -> u128 {
        u128::from(self.glyph)
            | u128::from(self.fg) << 32
            | u128::from(self.bg) << 64
            | u128::from(self.meta) << 96
    }
}

/// Packs ratatui buffers into [`PackedCell`]s, interning multi-codepoint graphemes.
#[derive(Debug, Default)]
pub struct Packer {
    clusters: HashMap<String, u32>,
}

impl Packer {
    /// Starts with an empty cluster table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Packs every cell of `buffer`, row by row.
    pub fn pack(&mut self, buffer: &Buffer) -> Vec<PackedCell> {
        buffer
            .content
            .iter()
            .map(|cell| self.pack_cell(cell))
            .collect()
    }

    fn pack_cell(&mut self, cell: &Cell) -> PackedCell {
        PackedCell {
            glyph: self.glyph(cell.symbol()),
            fg: color(cell.fg),
            bg: color(cell.bg),
            meta: u32::from(cell.modifier.bits()) & ATTRIBUTE_BITS,
        }
    }

    fn glyph(&mut self, symbol: &str) -> u32 {
        let mut chars = symbol.chars();
        match (chars.next(), chars.next()) {
            (None, _) => CONTINUATION,
            (Some(scalar), None) => u32::from(scalar),
            (Some(_), Some(_)) => {
                let next_id = u32::try_from(self.clusters.len()).unwrap_or(u32::MAX);
                CLUSTER_TAG | *self.clusters.entry(symbol.to_owned()).or_insert(next_id)
            }
        }
    }
}

fn color(color: Color) -> u32 {
    match color {
        Color::Reset => 0,
        Color::Black => indexed(0),
        Color::Red => indexed(1),
        Color::Green => indexed(2),
        Color::Yellow => indexed(3),
        Color::Blue => indexed(4),
        Color::Magenta => indexed(5),
        Color::Cyan => indexed(6),
        Color::Gray => indexed(7),
        Color::DarkGray => indexed(8),
        Color::LightRed => indexed(9),
        Color::LightGreen => indexed(10),
        Color::LightYellow => indexed(11),
        Color::LightBlue => indexed(12),
        Color::LightMagenta => indexed(13),
        Color::LightCyan => indexed(14),
        Color::White => indexed(15),
        Color::Indexed(index) => indexed(u32::from(index)),
        Color::Rgb(red, green, blue) => {
            COLOR_RGB | u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue)
        }
    }
}

fn indexed(index: u32) -> u32 {
    COLOR_INDEXED | index
}

/// Counts the cells that differ between two grids of `width` columns, skipping equal rows
/// with one slice comparison each.
#[must_use]
pub fn changed_cells(previous: &[PackedCell], next: &[PackedCell], width: usize) -> usize {
    previous
        .chunks_exact(width)
        .zip(next.chunks_exact(width))
        .filter(|(before, after)| before != after)
        .map(|(before, after)| {
            before
                .iter()
                .zip(after)
                .filter(|(old, new)| old != new)
                .count()
        })
        .sum()
}

/// Counts the cells that differ between two grids of `width` columns stored as 128-bit
/// words. Equal rows are skipped with one slice comparison, which compiles to `memcmp`.
#[must_use]
pub fn changed_words(previous: &[u128], next: &[u128], width: usize) -> usize {
    previous
        .chunks_exact(width)
        .zip(next.chunks_exact(width))
        .filter(|(before, after)| before != after)
        .map(|(before, after)| {
            before
                .iter()
                .zip(after)
                .filter(|(old, new)| old != new)
                .count()
        })
        .sum()
}

/// Writes an FNV-1a hash of each row of `width` cells into `hashes`, for a dirty-row check.
pub fn row_hashes(cells: &[PackedCell], width: usize, hashes: &mut Vec<u64>) {
    hashes.clear();
    hashes.extend(cells.chunks_exact(width).map(|row| {
        row.iter()
            .flat_map(|cell| [cell.glyph, cell.fg, cell.bg, cell.meta])
            .fold(FNV_OFFSET, |hash, word| {
                (hash ^ u64::from(word)).wrapping_mul(FNV_PRIME)
            })
    }));
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier, Style};

    use super::{CLUSTER_TAG, CONTINUATION, PackedCell, Packer, row_hashes};

    #[test]
    fn a_packed_cell_is_16_bytes_aligned_to_16() {
        assert_eq!(size_of::<PackedCell>(), 16);
        assert_eq!(align_of::<PackedCell>(), 16);
    }

    #[test]
    fn packs_scalar_colors_and_attributes() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer[(0, 0)]
            .set_symbol("é")
            .set_style(Style::new().fg(Color::Rgb(1, 2, 3)).bg(Color::Red))
            .set_style(Style::new().add_modifier(Modifier::BOLD | Modifier::ITALIC));
        let packed = Packer::new().pack(&buffer);
        let expected = PackedCell {
            glyph: 0xE9,
            fg: 0x8001_0203,
            bg: 0x4000_0001,
            meta: 0b101,
        };
        assert_eq!(packed[0], expected);
        assert_eq!(
            packed[1],
            PackedCell {
                glyph: 0x20,
                ..PackedCell::default()
            }
        );
    }

    #[test]
    fn interns_clusters_with_stable_ids_and_marks_continuations() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
        buffer[(0, 0)].set_symbol("e\u{301}");
        buffer[(1, 0)].set_symbol("👩\u{200D}💻");
        buffer[(2, 0)].set_symbol("");
        buffer[(3, 0)].set_symbol("e\u{301}");
        let glyphs: Vec<u32> = Packer::new()
            .pack(&buffer)
            .iter()
            .map(|cell| cell.glyph)
            .collect();
        assert_eq!(
            glyphs,
            [CLUSTER_TAG, CLUSTER_TAG | 1, CONTINUATION, CLUSTER_TAG]
        );
    }

    #[test]
    fn row_hashes_differ_only_for_changed_rows() {
        let first = [PackedCell::default(); 6];
        let mut second = first;
        second[4].glyph = 0x41;
        let (mut before, mut after) = (Vec::new(), Vec::new());
        row_hashes(&first, 3, &mut before);
        row_hashes(&second, 3, &mut after);
        assert_eq!(before.len(), 2);
        assert_eq!(before[0], after[0]);
        assert_ne!(before[1], after[1]);
    }
}
