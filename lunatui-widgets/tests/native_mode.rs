// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, NativeAttrs, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::RenderCx;

const W: u16 = 24;
const H: u16 = 8;

fn scene(native: bool) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(W, H);
    let mut clusters = ClusterTable::new();
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).native(native);
        let base = Style::new().bg(cx.token(Token::Surface0));
        cx.fill(Rect::new(0, 0, W, H), base, ' ');
        cx.put_str(2, 1, "日本 panel title", base);
        cx.translucent_fill(Rect::new(1, 1, 14, 4), cx.token(Token::Surface1), 0.6);
        cx.fade_glyphs(Rect::new(2, 3, 6, 1), 0.5);
        cx.put_glyph_at(5.25, 6.0, '●', base);
        cx.put_glyph_at(9.5, 6.4, '●', base);
        cx.put_glyph_at(20.0, 7.0, '日', base);
    }
    (grid, clusters)
}

fn layout(grid: &Grid) -> Vec<(u16, u16, u128)> {
    let mut cells = Vec::new();
    for y in 0..H {
        for x in 0..W {
            let word = grid.cell_bits(x, y);
            let glyph_and_width = word & (u128::from(u32::MAX) | (0xF000_u128 << 96));
            cells.push((x, y, glyph_and_width));
        }
    }
    cells
}

fn touched(grid: &Grid) -> Vec<(u16, u16)> {
    let plain = Grid::new(W, H);
    let mut cells = Vec::new();
    for y in 0..H {
        for x in 0..W {
            let a = grid.cell_bits(x, y) & u128::from(u32::MAX);
            if a != plain.cell_bits(x, y) & u128::from(u32::MAX) {
                cells.push((x, y));
            }
        }
    }
    cells
}

#[test]
fn native_and_ansi_frames_share_glyphs_widths_and_touched_cells() {
    let (ansi, ansi_clusters) = scene(false);
    let (native, native_clusters) = scene(true);
    assert_eq!(layout(&native), layout(&ansi));
    assert_eq!(touched(&native), touched(&ansi));
    assert_eq!(
        native.display_text(&native_clusters),
        ansi.display_text(&ansi_clusters)
    );
}

#[test]
fn ansi_frames_carry_no_native_attributes() {
    let (ansi, _clusters) = scene(false);
    assert!(ansi.ext_table().is_empty());
    for y in 0..H {
        for x in 0..W {
            assert_eq!(ansi.native(x, y), NativeAttrs::OPAQUE);
        }
    }
}

#[test]
fn native_frames_stripped_of_attributes_match_ansi_cells_except_colors() {
    let (native, _c) = scene(true);
    let stripped = native.without_native();
    assert!(stripped.ext_table().is_empty());
    assert_eq!(layout(&stripped), layout(&scene(false).0));
}

#[test]
fn native_translucent_fill_marks_exactly_the_filled_cells() {
    let (native, _c) = scene(true);
    for y in 0..H {
        for x in 0..W {
            let inside = (1..15).contains(&x) && (1..5).contains(&y);
            let alpha = native.native(x, y).bg_alpha;
            assert_eq!(alpha != 255, inside, "cell {x},{y}");
            if inside {
                assert_eq!(alpha, 153);
            }
        }
    }
}

#[test]
fn native_translucent_fill_keeps_the_tint_not_the_blend() {
    let theme = Theme::builtin(Variant::Dark);
    let tint = theme.resolve(Token::Surface1);
    let (native, _c) = scene(true);
    let (ansi, _c) = scene(false);
    let native_bg = native.get(1, 1).map(|cell| cell.style().bg);
    let ansi_bg = ansi.get(1, 1).map(|cell| cell.style().bg);
    assert_ne!(native_bg, ansi_bg);
    assert_eq!(native_bg, Some(tint));
}

#[test]
fn glyph_alpha_applies_only_in_native_mode() {
    let (native, _c) = scene(true);
    for x in 2..8 {
        assert_eq!(native.native(x, 3).fg_alpha, 128);
    }
    assert_eq!(native.native(8, 3).fg_alpha, 255);
    let (ansi, _c) = scene(false);
    assert_eq!(ansi.native(2, 3).fg_alpha, 255);
}

#[test]
fn glyphs_land_in_the_nearest_cell_with_the_remaining_eighths() {
    let (native, _c) = scene(true);
    assert_eq!(native.native(5, 6), NativeAttrs::new(255, 255, 2, 0));
    assert_eq!(native.native(10, 6), NativeAttrs::new(255, 255, -4, 3));
    assert_eq!(native.native(20, 7), NativeAttrs::OPAQUE);
    let (ansi, clusters) = scene(false);
    assert_eq!(ansi.row_display(6, &clusters).chars().nth(5), Some('●'));
    assert_eq!(ansi.row_display(6, &clusters).chars().nth(10), Some('●'));
}

#[test]
fn off_screen_and_non_finite_glyph_positions_draw_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    let blank = grid.clone();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).native(true);
    cx.put_glyph_at(-1.0, 0.0, 'x', Style::new());
    cx.put_glyph_at(f32::NAN, 0.0, 'x', Style::new());
    cx.put_glyph_at(0.0, f32::INFINITY, 'x', Style::new());
    cx.put_glyph_at(70_000.0, 0.0, 'x', Style::new());
    cx.put_glyph_at(9.0, 9.0, 'x', Style::new());
    assert_eq!(grid, blank);
}

#[test]
fn native_flag_defaults_off_and_survives_the_builder() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    assert!(!cx.is_native());
    assert!(cx.native(true).is_native());
}

#[test]
fn positions_just_left_of_the_first_cell_round_into_it() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).native(true);
        cx.put_glyph_at(-0.5, 0.0, 'a', Style::new());
        cx.put_glyph_at(2.0, -0.25, 'b', Style::new());
        cx.put_glyph_at(-0.75, 1.0, 'c', Style::new());
    }
    assert_eq!(grid.native(0, 0), NativeAttrs::new(255, 255, -4, 0));
    assert_eq!(grid.native(2, 0), NativeAttrs::new(255, 255, 0, -2));
    assert_eq!(grid.row_display(1, &clusters).trim(), "");
}
