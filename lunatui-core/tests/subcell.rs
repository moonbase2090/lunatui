// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Literal subcell glyph and fallback reference tests.

use lunatui_core::subcell::{GlyphCapabilities, GlyphSet, horizontal_eighths, vertical_eighths};

#[test]
fn startup_selection_obeys_font_support_order() {
    for bits in 0..8 {
        let capabilities = GlyphCapabilities {
            octants: bits & 4 != 0,
            braille: bits & 2 != 0,
            blocks: bits & 1 != 0,
        };
        let expected = [
            GlyphSet::Ascii,
            GlyphSet::Blocks,
            GlyphSet::Braille,
            GlyphSet::Braille,
            GlyphSet::Octants,
            GlyphSet::Octants,
            GlyphSet::Octants,
            GlyphSet::Octants,
        ][bits];
        assert_eq!(GlyphSet::select(capabilities), expected);
    }
    assert_eq!(GlyphSet::default(), GlyphSet::Ascii);
    assert_eq!(
        GlyphSet::select(GlyphCapabilities::default()),
        GlyphSet::Ascii
    );
}

#[test]
fn named_octants_and_legacy_aliases_use_literal_codepoints() {
    for (mask, expected) in [
        (0, ' '),
        (1, '\u{1cea8}'),
        (2, '\u{1ceab}'),
        (3, '\u{1fb82}'),
        (4, '\u{1cd00}'),
        (5, '▘'),
        (0x14, '\u{1fbe6}'),
        (0x28, '\u{1fbe7}'),
        (0x3f, '\u{1fb85}'),
        (0x40, '\u{1cea3}'),
        (0x55, '▌'),
        (0x5a, '▞'),
        (0x80, '\u{1cea0}'),
        (0xa5, '▚'),
        (0xaa, '▐'),
        (0xc0, '▂'),
        (0xf0, '▄'),
        (0xfc, '▆'),
        (0xff, '█'),
    ] {
        assert_eq!(GlyphSet::Octants.glyph(mask), expected, "mask {mask:02x}");
    }
}

#[test]
fn braille_positions_follow_rows_instead_of_unicode_dot_order() {
    for (mask, expected) in [
        (0, ' '),
        (1, '⠁'),
        (2, '⠈'),
        (4, '⠂'),
        (8, '⠐'),
        (16, '⠄'),
        (32, '⠠'),
        (64, '⡀'),
        (128, '⢀'),
        (0x55, '⡇'),
        (0xaa, '⢸'),
        (0xff, '⣿'),
    ] {
        assert_eq!(GlyphSet::Braille.glyph(mask), expected);
    }
}

#[test]
fn block_fallback_preserves_full_quadrants_and_discards_ties() {
    for (mask, expected) in [
        (1, ' '),
        (2, ' '),
        (0x05, '▘'),
        (0x0a, '▝'),
        (0x0f, '▀'),
        (0x50, '▖'),
        (0xa0, '▗'),
        (0x5a, '▞'),
        (0xa5, '▚'),
        (0xff, '█'),
    ] {
        assert_eq!(GlyphSet::Blocks.glyph(mask), expected);
    }
    assert_eq!(GlyphSet::Ascii.glyph(0), ' ');
    for mask in 1..=255 {
        assert_eq!(GlyphSet::Ascii.glyph(mask), '#');
    }
}

#[test]
fn all_exact_masks_are_unique_and_every_glyph_is_one_cell_wide() {
    for tier in [
        GlyphSet::Octants,
        GlyphSet::Braille,
        GlyphSet::Blocks,
        GlyphSet::Ascii,
    ] {
        let chars: Vec<_> = (0..=255).map(|mask| tier.glyph(mask)).collect();
        for glyph in &chars {
            assert_eq!(lunatui_core::char_width(*glyph), 1, "{glyph:?}");
        }
        if matches!(tier, GlyphSet::Octants | GlyphSet::Braille) {
            let mut unique = chars;
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), 256);
        }
    }
}

#[test]
fn eighth_block_lookups_include_empty_full_and_clamped_values() {
    assert_eq!(
        (0..=8).map(horizontal_eighths).collect::<String>(),
        " ▏▎▍▌▋▊▉█"
    );
    assert_eq!(
        (0..=8).map(vertical_eighths).collect::<String>(),
        " ▁▂▃▄▅▆▇█"
    );
    assert_eq!(horizontal_eighths(255), '█');
    assert_eq!(vertical_eighths(255), '█');
}
