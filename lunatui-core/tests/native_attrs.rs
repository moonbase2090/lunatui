// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{
    ClusterTable, Color, ExtTable, Grid, MAX_EXT_ENTRIES, MAX_SUBCELL_OFFSET, NativeAttrs, Rect,
    Style,
};
use proptest::prelude::*;

fn glass() -> NativeAttrs {
    NativeAttrs::new(255, 128, 0, 0)
}

fn shifted() -> NativeAttrs {
    NativeAttrs::new(255, 255, 3, -2)
}

fn text_grid(text: &str) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(8, 2);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), text, &mut clusters);
    (grid, clusters)
}

#[test]
fn defaults_are_opaque_and_unshifted() {
    let attrs = NativeAttrs::default();
    assert_eq!(attrs, NativeAttrs::OPAQUE);
    assert!(attrs.is_default());
    assert!(!glass().is_default());
}

#[test]
fn offsets_clamp_instead_of_wrapping() {
    let attrs = NativeAttrs::new(255, 255, i8::MAX, i8::MIN);
    assert_eq!(attrs.dx, MAX_SUBCELL_OFFSET);
    assert_eq!(attrs.dy, -MAX_SUBCELL_OFFSET);
}

#[test]
fn table_never_stores_the_defaults() {
    let mut table = ExtTable::new();
    assert_eq!(table.intern(NativeAttrs::OPAQUE), 0);
    assert!(table.is_empty());
    assert_eq!(table.get(0), None);
}

#[test]
fn table_shares_one_index_for_equal_attributes() {
    let mut table = ExtTable::new();
    let first = table.intern(glass());
    let second = table.intern(shifted());
    assert_eq!((first, second), (1, 2));
    assert!(!table.is_empty());
    assert_eq!(table.intern(glass()), first);
    assert_eq!(table.len(), 2);
    assert_eq!(table.get(first), Some(glass()));
    assert_eq!(table.get(second), Some(shifted()));
    assert_eq!(table.get(3), None);
}

#[test]
fn full_table_drops_new_attributes_but_keeps_old_ones() {
    let mut table = ExtTable::new();
    let mut step = 0usize;
    while table.len() < MAX_EXT_ENTRIES {
        let low = u8::try_from(step % 256).unwrap_or(0);
        let high = u8::try_from(step / 256).unwrap_or(0);
        assert_ne!(table.intern(NativeAttrs::new(low, high, 1, 1)), 0);
        step += 1;
    }
    assert_eq!(table.len(), MAX_EXT_ENTRIES);
    let known = NativeAttrs::new(1, 0, 1, 1);
    assert_ne!(table.intern(known), 0);
    assert_eq!(table.intern(NativeAttrs::new(7, 7, 2, 2)), 0);
    assert_eq!(table.len(), MAX_EXT_ENTRIES);
}

#[test]
fn clear_and_seed_reset_the_entries() {
    let mut previous = ExtTable::new();
    previous.intern(glass());
    previous.intern(shifted());
    let mut next = ExtTable::new();
    next.intern(shifted());
    next.seed_from(&previous);
    assert_eq!(next, previous);
    next.clear();
    assert!(next.is_empty());
}

#[test]
fn set_native_round_trips_and_clearing_restores_the_cell() {
    let (mut grid, _clusters) = text_grid("hello");
    let before = grid.cell_bits(1, 0);
    grid.set_native(1, 0, glass());
    assert_eq!(grid.native(1, 0), glass());
    assert_ne!(grid.cell_bits(1, 0), before);
    grid.set_native(1, 0, NativeAttrs::OPAQUE);
    assert_eq!(grid.native(1, 0), NativeAttrs::OPAQUE);
    assert_eq!(grid.cell_bits(1, 0), before);
}

#[test]
fn native_is_default_out_of_bounds_and_on_untouched_cells() {
    let (mut grid, _clusters) = text_grid("hello");
    grid.set_native(1, 0, glass());
    assert_eq!(grid.native(0, 0), NativeAttrs::OPAQUE);
    assert_eq!(grid.native(99, 99), NativeAttrs::OPAQUE);
    grid.set_native(99, 99, glass());
    assert_eq!(grid.ext_table().len(), 1);
}

#[test]
fn wide_graphemes_carry_attributes_to_both_cells() {
    let (mut grid, _clusters) = text_grid("日x");
    grid.set_native(0, 0, shifted());
    assert_eq!(grid.native(0, 0), shifted());
    assert_eq!(grid.native(1, 0), shifted());
    assert_eq!(grid.native(2, 0), NativeAttrs::OPAQUE);
    assert_eq!(grid.cell_bits(0, 0) >> 112, grid.cell_bits(1, 0) >> 112);
    grid.set_native(1, 0, NativeAttrs::OPAQUE);
    assert_eq!(grid.native(0, 0), NativeAttrs::OPAQUE);
    assert_eq!(grid.native(1, 0), NativeAttrs::OPAQUE);
}

#[test]
fn overwriting_a_cell_clears_its_attributes() {
    let (mut grid, mut clusters) = text_grid("hello");
    grid.set_native(1, 0, glass());
    grid.put_str(1, 0, Style::new(), "X", &mut clusters);
    assert_eq!(grid.native(1, 0), NativeAttrs::OPAQUE);
    grid.set_native(2, 0, glass());
    grid.put_char(2, 0, Style::new(), 'Y', &mut clusters);
    assert_eq!(grid.native(2, 0), NativeAttrs::OPAQUE);
    grid.set_native(3, 0, glass());
    grid.fill(Rect::new(3, 0, 1, 1), Style::new(), ' ', &mut clusters);
    assert_eq!(grid.native(3, 0), NativeAttrs::OPAQUE);
}

#[test]
fn restyling_keeps_attributes() {
    let (mut grid, _clusters) = text_grid("hello");
    grid.set_native(1, 0, glass());
    grid.set_style(Rect::new(0, 0, 5, 1), Style::new().bg(Color::Rgb(1, 2, 3)));
    assert_eq!(grid.native(1, 0), glass());
}

#[test]
fn without_native_keeps_text_and_widths_and_drops_attributes() {
    let (mut grid, clusters) = text_grid("日ab");
    grid.set_native(0, 0, shifted());
    grid.set_native(2, 0, glass());
    let plain = grid.without_native();
    assert!(plain.ext_table().is_empty());
    assert_eq!(plain.native(0, 0), NativeAttrs::OPAQUE);
    assert_eq!(plain.display_text(&clusters), grid.display_text(&clusters));
    let (reference, _clusters) = text_grid("日ab");
    assert_eq!(plain, reference);
}

#[test]
fn seeded_grids_give_equal_attributes_equal_indices_across_frames() {
    let (mut first, _c) = text_grid("abcd");
    first.set_native(0, 0, glass());
    first.set_native(1, 0, shifted());
    let (mut second, _c) = text_grid("abcd");
    second.seed_native_from(&first);
    second.set_native(0, 0, shifted());
    second.set_native(1, 0, glass());
    assert_eq!(second.cell_bits(0, 0) >> 112, first.cell_bits(1, 0) >> 112);
    assert_eq!(second.cell_bits(1, 0) >> 112, first.cell_bits(0, 0) >> 112);
}

#[cfg(feature = "blit")]
#[test]
fn blit_remaps_attributes_into_the_destination_table() {
    let (mut src, _c) = text_grid("abcd");
    src.set_native(1, 0, glass());
    src.set_native(2, 0, shifted());
    let mut dst = Grid::new(8, 2);
    let mut clusters = ClusterTable::new();
    dst.put_str(0, 1, Style::new(), "zz", &mut clusters);
    dst.set_native(0, 1, shifted());
    dst.blit(&src, Rect::new(1, 0, 2, 1), 4, 0);
    assert_eq!(dst.native(4, 0), glass());
    assert_eq!(dst.native(5, 0), shifted());
    assert_eq!(dst.native(0, 1), shifted());
}

#[test]
fn setting_attributes_marks_the_cell_row_dirty() {
    let (mut grid, _clusters) = text_grid("hello");
    grid.clear_dirty();
    grid.set_native(3, 1, glass());
    let dirty = grid.row_dirty(1).unwrap_or_default();
    assert!(dirty.dirty);
    assert_eq!((dirty.min_x, dirty.max_x), (3, 4));
    assert!(!grid.row_dirty(0).unwrap_or_default().dirty);
}

#[cfg(feature = "blit")]
#[test]
fn blit_remaps_the_last_copied_column() {
    let (mut src, _c) = text_grid("abcd");
    src.set_native(3, 0, glass());
    let mut dst = Grid::new(8, 2);
    dst.blit(&src, Rect::new(1, 0, 3, 1), 1, 1);
    assert_eq!(dst.native(3, 1), glass());
    assert_eq!(dst.native(1, 1), NativeAttrs::OPAQUE);
}

proptest! {
    #[test]
    fn set_then_clear_restores_every_cell(
        x in 0u16..8,
        y in 0u16..2,
        fg in any::<u8>(),
        bg in any::<u8>(),
        dx in any::<i8>(),
        dy in any::<i8>(),
    ) {
        let (mut grid, _clusters) = text_grid("日abcde");
        let before = grid.clone();
        grid.set_native(x, y, NativeAttrs::new(fg, bg, dx, dy));
        grid.set_native(x, y, NativeAttrs::OPAQUE);
        for row in 0..2 {
            for col in 0..8 {
                prop_assert_eq!(grid.cell_bits(col, row), before.cell_bits(col, row));
            }
        }
    }

    #[test]
    fn equal_attributes_always_share_an_index(
        fg in any::<u8>(),
        bg in any::<u8>(),
        dx in any::<i8>(),
        dy in any::<i8>(),
    ) {
        let attrs = NativeAttrs::new(fg, bg, dx, dy);
        let mut table = ExtTable::new();
        let first = table.intern(attrs);
        prop_assert_eq!(table.intern(attrs), first);
        prop_assert!(attrs.dx.abs() <= MAX_SUBCELL_OFFSET && attrs.dy.abs() <= MAX_SUBCELL_OFFSET);
    }
}
