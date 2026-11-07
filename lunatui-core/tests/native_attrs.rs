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
    assert_eq!(attrs.dx(), MAX_SUBCELL_OFFSET);
    assert_eq!(attrs.dy(), -MAX_SUBCELL_OFFSET);
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
fn seeding_keeps_live_entries_at_their_index_and_frees_the_rest() {
    let mut previous = ExtTable::new();
    let first = previous.intern(glass());
    let second = previous.intern(shifted());
    let third = previous.intern(NativeAttrs::new(10, 20, 1, 1));
    let mut next = ExtTable::new();
    next.intern(shifted());
    next.seed_from(&previous, |index| index != second);
    assert_eq!(next.len(), 2);
    assert_eq!(next.slot_count(), 3);
    assert_eq!(next.get(first), Some(glass()));
    assert_eq!(next.get(second), None);
    assert_eq!(next.get(third), Some(NativeAttrs::new(10, 20, 1, 1)));
    assert_eq!(next.intern(glass()), first);
    assert_eq!(next.intern(shifted()), second);
    assert_eq!(next.slot_count(), 3);
    next.clear();
    assert!(next.is_empty());
    assert_eq!(next.slot_count(), 0);
}

#[test]
fn seeding_trims_unused_slots_at_the_end() {
    let mut previous = ExtTable::new();
    previous.intern(glass());
    previous.intern(shifted());
    let mut next = ExtTable::new();
    next.seed_from(&previous, |index| index == 1);
    assert_eq!(next.slot_count(), 1);
    assert_eq!(next.intern(shifted()), 2);
    next.seed_from(&previous, |_| false);
    assert_eq!(next.slot_count(), 0);
    assert!(next.is_empty());
    assert_eq!(next.intern(shifted()), 1);
}

#[test]
fn a_long_run_of_changing_attributes_never_saturates_the_table() {
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new(), "abcd", &mut clusters);
    let mut previous = grid.clone();
    for frame in 0..3000u32 {
        let mut next = Grid::new(4, 1);
        next.put_str(0, 0, Style::new(), "abcd", &mut clusters);
        next.seed_native_from(&previous);
        let alpha = u8::try_from(frame % 250).unwrap_or(0);
        let wobble = i8::try_from(frame % 5).unwrap_or(0);
        next.set_native(0, 0, NativeAttrs::new(alpha, 100, wobble, 0));
        next.set_native(1, 0, NativeAttrs::new(alpha, 50, 0, wobble));
        assert!(next.ext_table().slot_count() <= 4, "frame {frame}");
        assert_ne!(next.native(0, 0), NativeAttrs::OPAQUE);
        previous = next;
    }
}

#[test]
fn attributes_can_only_be_built_inside_the_offset_range() {
    let attrs = NativeAttrs::OPAQUE.with_offset(100, -100);
    assert_eq!(
        (attrs.dx(), attrs.dy()),
        (MAX_SUBCELL_OFFSET, -MAX_SUBCELL_OFFSET)
    );
    let attrs = NativeAttrs::OPAQUE.with_fg_alpha(10).with_bg_alpha(20);
    assert_eq!((attrs.fg_alpha(), attrs.bg_alpha()), (10, 20));
    assert_eq!((attrs.dx(), attrs.dy()), (0, 0));
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
        prop_assert!(attrs.dx().abs() <= MAX_SUBCELL_OFFSET && attrs.dy().abs() <= MAX_SUBCELL_OFFSET);
    }
}
