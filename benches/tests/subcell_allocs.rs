// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_core::subcell::GlyphSet;
use std::hint::black_box;

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn measured_glyph_lookups_return_literal_results_without_allocating() {
    let (glyphs, stats) = ALLOC.measure(|| {
        [
            black_box(GlyphSet::Octants).glyph(black_box(4)),
            black_box(GlyphSet::Octants).glyph(black_box(0x55)),
            black_box(GlyphSet::Braille).glyph(black_box(2)),
            black_box(GlyphSet::Blocks).glyph(black_box(0x5a)),
            black_box(GlyphSet::Ascii).glyph(black_box(0xff)),
        ]
    });
    assert_eq!(glyphs, ['\u{1cd00}', '▌', '⠈', '▞', '#']);
    assert_eq!(stats.allocations, 0);
}
