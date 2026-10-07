// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use core::mem::{align_of, size_of};

use lunatui_core::{
    Attrs, Cell, CellWidth, Color, DecodedGlyph, GlyphSlot, Meta, Style, UnderlineStyle,
};

#[test]
fn cell_is_sixteen_bytes_and_sixteen_aligned() {
    assert_eq!(size_of::<Cell>(), 16);
    assert_eq!(align_of::<Cell>(), 16);
}

#[test]
fn blank_cell_matches_adr_space_and_default_style() {
    assert_eq!(Cell::BLANK.glyph().decode(), DecodedGlyph::Inline(' '));
    assert_eq!(Cell::BLANK.style().fg, Color::Default);
    assert_eq!(Cell::BLANK.style().bg, Color::Default);
    assert_eq!(Cell::BLANK.meta().width, CellWidth::One);
}

#[test]
fn cleared_row_repeats_the_blank_word() {
    let word = Cell::BLANK.to_bits();
    assert_eq!(Cell::from_bits(word).map(Cell::to_bits), Some(word));
    assert_eq!(word, Cell::default().to_bits());
}

#[test]
fn indexed_and_rgb_colors_encode_expected_bits() {
    assert_eq!(Color::Indexed(1).to_bits(), 0x4000_0001);
    assert_eq!(Color::Rgb(1, 2, 3).to_bits(), 0x8001_0203);
    assert_eq!(Color::from_bits(0x4000_0001), Some(Color::Indexed(1)));
    assert_eq!(Color::from_bits(0x8001_0203), Some(Color::Rgb(1, 2, 3)),);
}

#[test]
fn attrs_and_meta_pack_underline_and_width() {
    let attrs = Attrs::new()
        .set_bold(true)
        .set_underline(UnderlineStyle::Double);
    assert_eq!(attrs.to_bits(), 0x11);
    let meta = Meta {
        attrs,
        width: CellWidth::Two,
        ..Meta::default()
    };
    assert_eq!(meta.to_bits(), 0x2011);
    assert_eq!(Meta::from_bits(0x2011), Some(meta));
}

#[test]
fn from_bits_round_trips_a_styled_glyph() {
    let glyph = GlyphSlot::from_bits(u32::from('日'));
    let style = Style::new()
        .fg(Color::Indexed(12))
        .bg(Color::Rgb(0x10, 0x20, 0x30))
        .with_attrs(Attrs::new().set_italic(true));
    let meta = Meta {
        width: CellWidth::Two,
        ..Meta::default()
    };
    let cell = Cell::from_parts(glyph, style, meta);
    let bits = cell.to_bits();
    assert_eq!(Cell::from_bits(bits), Some(cell));
}

#[test]
fn from_bits_rejects_reserved_color_bits() {
    assert!(Cell::from_bits(Cell::BLANK.to_bits() | (1 << 56)).is_none());
}
