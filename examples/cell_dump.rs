// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io::{self, Write};

use lunatui_core::{Attrs, Cell, CellWidth, Color, GlyphSlot, Meta, Style, UnderlineStyle};

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "16-byte packed cells (PR 06, ADR 0001):")?;
    for (label, cell) in sample_cells() {
        writeln!(stdout, "{label}: {:032x}", cell.to_bits())?;
    }
    Ok(())
}

fn sample_cells() -> [(&'static str, Cell); 4] {
    let border = Style::new()
        .fg(Color::Indexed(14))
        .with_attrs(Attrs::new().set_bold(true));
    let text = Style::new()
        .fg(Color::Rgb(0xCC, 0x88, 0x44))
        .with_attrs(Attrs::new().set_underline(UnderlineStyle::Single));
    [
        ("blank", Cell::BLANK),
        (
            "corner",
            Cell::from_parts(
                GlyphSlot::from_bits(u32::from('┌')),
                border,
                Meta {
                    width: CellWidth::One,
                    ..Meta::default()
                },
            ),
        ),
        (
            "H",
            Cell::from_parts(
                GlyphSlot::from_bits(u32::from('H')),
                text,
                Meta {
                    width: CellWidth::One,
                    ..Meta::default()
                },
            ),
        ),
        (
            "wide_lead",
            Cell::from_parts(
                GlyphSlot::from_bits(u32::from('日')),
                text,
                Meta {
                    width: CellWidth::Two,
                    ..Meta::default()
                },
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use lunatui_core::Cell;

    use super::sample_cells;

    #[test]
    fn each_sample_cell_is_sixteen_bytes() {
        for (_, cell) in sample_cells() {
            assert_eq!(Cell::from_bits(cell.to_bits()), Some(cell));
        }
    }

    #[test]
    fn blank_sample_matches_cell_blank() {
        assert_eq!(sample_cells()[0].1, Cell::BLANK);
    }
}
