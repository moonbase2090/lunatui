// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid, Rect};
use lunatui_image::{Fit, ImageError, ImageSource};
use lunatui_theme::{Profile, Theme, Variant};
use lunatui_widgets::{Image, RenderCx, Widget, WidgetState};

#[test]
fn half_block_preserves_top_and_bottom_pixel_colors() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(
        1,
        2,
        vec![255, 0, 0, 255, 0, 0, 255, 255].into_boxed_slice(),
    )?;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source).render(Rect::new(0, 0, 1, 1), &mut cx);

    let cell = grid.get(0, 0).ok_or(ImageError::Empty)?;
    assert_eq!(cell.glyph().decode(), DecodedGlyph::Inline('▀'));
    assert_eq!(cell.style().fg, Color::Rgb(255, 0, 0));
    assert_eq!(cell.style().bg, Color::Rgb(0, 0, 255));
    Ok(())
}

#[test]
fn contain_leaves_letterbox_in_surface_token() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(1, 1, vec![0, 255, 0, 255].into_boxed_slice())?;
    let theme = Theme::builtin(Variant::Dark);
    let surface = theme.resolve(lunatui_theme::Token::Surface0);
    let mut grid = Grid::new(1, 2);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source).render(Rect::new(0, 0, 1, 2), &mut cx);

    let top = grid.get(0, 0).ok_or(ImageError::Empty)?;
    let bottom = grid.get(0, 1).ok_or(ImageError::Empty)?;
    assert_eq!(top.style().fg, surface);
    assert_eq!(top.style().bg, Color::Rgb(0, 255, 0));
    assert_eq!(bottom.style().fg, surface);
    assert_eq!(bottom.style().bg, surface);
    Ok(())
}

#[test]
fn transparent_pixels_composite_over_surface_and_fit_is_configurable() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(1, 2, vec![255, 0, 0, 128, 0, 0, 0, 0].into_boxed_slice())?;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source)
        .fit(Fit::Stretch)
        .render(Rect::new(0, 0, 1, 1), &mut cx);

    let cell = grid.get(0, 0).ok_or(ImageError::Empty)?;
    assert_eq!(cell.style().fg, Color::Rgb(143, 15, 23));
    assert_eq!(
        cell.style().bg,
        theme.resolve(lunatui_theme::Token::Surface0)
    );
    Ok(())
}

#[test]
fn focused_image_uses_theme_focus_ring_and_insets_the_pixels() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(
        1,
        2,
        vec![255, 0, 0, 255, 0, 0, 255, 255].into_boxed_slice(),
    )?;
    let theme = Theme::builtin(Variant::Dark);
    let focus_ring = theme.resolve(lunatui_theme::Token::FocusRing);
    let mut grid = Grid::new(3, 3);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source)
        .state(WidgetState::Focused)
        .render(Rect::new(0, 0, 3, 3), &mut cx);

    let corner = grid.get(0, 0).ok_or(ImageError::Empty)?;
    let center = grid.get(1, 1).ok_or(ImageError::Empty)?;
    assert_eq!(corner.style().fg, focus_ring);
    assert_ne!(corner.glyph().decode(), DecodedGlyph::Inline('▀'));
    assert_eq!(center.style().fg, Color::Rgb(255, 0, 0));
    Ok(())
}

#[test]
fn disabled_image_is_dimmed_with_theme_surface_compositing() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(
        1,
        2,
        vec![255, 0, 0, 255, 255, 0, 0, 255].into_boxed_slice(),
    )?;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source)
        .state(WidgetState::Disabled)
        .render(Rect::new(0, 0, 1, 1), &mut cx);

    let cell = grid.get(0, 0).ok_or(ImageError::Empty)?;
    assert_eq!(cell.style().fg, Color::Rgb(165, 12, 18));
    assert_eq!(cell.style().bg, Color::Rgb(165, 12, 18));
    Ok(())
}

#[test]
fn image_colors_follow_the_prepared_ansi16_profile() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(
        1,
        2,
        vec![255, 0, 0, 255, 0, 0, 255, 255].into_boxed_slice(),
    )?;
    let theme = Theme::builtin(Variant::Dark);
    let palette = theme.prepare(Profile::Ansi16);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).palette(&palette);
    Image::new(source).render(Rect::new(0, 0, 1, 1), &mut cx);

    let cell = grid.get(0, 0).ok_or(ImageError::Empty)?;
    assert_eq!(cell.style().fg, Color::Indexed(9));
    assert_eq!(cell.style().bg, Color::Indexed(4));
    Ok(())
}
