// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid, Rect};
use lunatui_image::{Fit, ImageError, ImageId, ImageRequest, ImageSink, ImageSource};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{Image, RenderCx, Widget, WidgetState};

const SURFACE: Color = Color::Rgb(20, 40, 60);

fn surface_theme() -> Theme {
    Theme::with_overrides(Variant::Dark, &[(Token::Surface0, SURFACE)])
}

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
    let surface = theme.resolve(Token::Surface0);
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
    let theme = surface_theme();
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source)
        .fit(Fit::Stretch)
        .render(Rect::new(0, 0, 1, 1), &mut cx);

    let cell = grid.get(0, 0).ok_or(ImageError::Empty)?;
    assert_eq!(cell.style().fg, Color::Rgb(138, 20, 30));
    assert_eq!(cell.style().bg, SURFACE);
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
    let focus_ring = theme.resolve(Token::FocusRing);
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
    let theme = surface_theme();
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Image::new(source)
        .state(WidgetState::Disabled)
        .render(Rect::new(0, 0, 1, 1), &mut cx);

    let cell = grid.get(0, 0).ok_or(ImageError::Empty)?;
    assert_eq!(cell.style().fg, Color::Rgb(161, 16, 24));
    assert_eq!(cell.style().bg, Color::Rgb(161, 16, 24));
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

struct CaptureSink {
    accepts_pixels: bool,
    request: Option<CapturedRequest>,
}

struct CapturedRequest {
    id: u32,
    generation: u64,
    source: ImageSource,
    area: Rect,
    fit: Fit,
    depth: i32,
    opacity: u8,
}

impl ImageSink for CaptureSink {
    fn supports_pixels(&self) -> bool {
        self.accepts_pixels
    }

    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        self.request = Some(CapturedRequest {
            id: request.id().get(),
            generation: request.generation(),
            source: request.source().clone(),
            area: request.area(),
            fit: request.fit(),
            depth: request.depth(),
            opacity: request.opacity(),
        });
        true
    }
}

#[test]
fn pixel_sink_receives_stable_identity_geometry_and_disabled_opacity() -> Result<(), ImageError> {
    let source = ImageSource::rgba8(1, 1, vec![20, 40, 60, 255].into_boxed_slice())?;
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(3, 3);
    let mut clusters = ClusterTable::new();
    let mut sink = CaptureSink {
        accepts_pixels: true,
        request: None,
    };
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut sink);
        Image::new(source.clone())
            .id(ImageId::new(7).ok_or(ImageError::Empty)?)
            .generation(12)
            .fit(Fit::Cover)
            .depth(-2)
            .state(WidgetState::Disabled)
            .render(Rect::new(0, 0, 3, 3), &mut cx);
    }

    let request = sink.request.ok_or(ImageError::Empty)?;
    assert_eq!(request.id, 7);
    assert_eq!(request.generation, 12);
    assert_eq!(request.source, source);
    assert_eq!(request.area, Rect::new(0, 0, 3, 3));
    assert_eq!(request.fit, Fit::Cover);
    assert_eq!(request.depth, -2);
    assert_eq!(request.opacity, 153);
    assert_eq!(
        grid.get(1, 1).map(|cell| cell.glyph().decode()),
        Some(DecodedGlyph::Inline(' '))
    );
    Ok(())
}
