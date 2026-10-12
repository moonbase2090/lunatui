// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::prototype::{PackedCell, Packer, changed_cells, changed_words};
use lunatui_bench::scenes::ratatui::dashboard::Frames;
use ratatui::layout::Rect;

fn packed_changes(area: Rect) -> [usize; 4] {
    let frames = Frames::new(area);
    let mut packer = Packer::new();
    let empty = packer.pack(&frames.empty);
    let first = packer.pack(&frames.first);
    let tick = packer.pack(&frames.tick);
    let scroll = packer.pack(&frames.scroll);
    let width = usize::from(area.width);
    [
        changed_cells(&first, &tick, width),
        changed_cells(&tick, &scroll, width),
        changed_cells(&empty, &scroll, width),
        changed_cells(&scroll, &scroll, width),
    ]
}

#[test]
fn packed_diff_finds_the_same_changes_as_ratatui_at_200x60() {
    assert_eq!(
        packed_changes(Rect::new(0, 0, 200, 60)),
        [14, 3745, 12_000, 0]
    );
}

#[test]
fn packed_diff_finds_the_same_changes_as_ratatui_at_400x120() {
    assert_eq!(
        packed_changes(Rect::new(0, 0, 400, 120)),
        [25, 1318, 48_000, 0]
    );
}

#[test]
fn the_word_diff_finds_the_same_changes_as_the_struct_diff() {
    let frames = Frames::new(Rect::new(0, 0, 200, 60));
    let mut packer = Packer::new();
    let tick: Vec<u128> = packer
        .pack(&frames.tick)
        .into_iter()
        .map(PackedCell::to_bits)
        .collect();
    let scroll: Vec<u128> = packer
        .pack(&frames.scroll)
        .into_iter()
        .map(PackedCell::to_bits)
        .collect();
    assert_eq!(changed_words(&tick, &scroll, 200), 3745);
    assert_eq!(changed_words(&scroll, &scroll, 200), 0);
}

#[test]
fn to_bits_puts_glyph_low_and_meta_high() {
    let cell = PackedCell {
        glyph: 0x1,
        fg: 0x2,
        bg: 0x3,
        meta: 0x4,
    };
    assert_eq!(cell.to_bits(), 0x0000_0004_0000_0003_0000_0002_0000_0001);
}
