// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::graphite::{BundledText, Face, TextMetrics, UiFont};
use lunatui_widgets::{PixelCx, RenderCx};

const CELL: (u32, u32) = (8, 16);
const RED: [u8; 4] = [220, 40, 30, 255];
const BACK: [u8; 4] = [10, 20, 30, 255];

struct Capture {
    background: Option<[u8; 3]>,
    images: Vec<ImageSource>,
}

impl ImageSink for Capture {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        Some(PixelTarget::new(
            PixelSize::new(
                u32::from(area.width) * CELL.0,
                u32::from(area.height) * CELL.1,
            )?,
            true,
            self.background,
        ))
    }
    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }
    fn submit_raster(&mut self, source: &ImageSource, _: Rect, _: i32) -> bool {
        self.images.push(source.clone());
        true
    }
}

fn paint(
    background: Option<[u8; 3]>,
    ui: Option<UiFont>,
    draw: impl FnOnce(&mut PixelCx<'_>),
) -> ImageSource {
    let area = Rect::new(0, 0, 16, 4);
    let mut grid = Grid::new(16, 4);
    let mut clusters = ClusterTable::new();
    let [r, g, b, _] = BACK;
    grid.fill(
        area,
        Style::new().bg(Color::Rgb(r, g, b)),
        ' ',
        &mut clusters,
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut images = Capture {
        background,
        images: Vec::new(),
    };
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme)
        .image_sink(&mut images)
        .render_mode(RenderMode::Graphite);
    if let Some(font) = ui {
        cx = cx.ui_font(font);
    }
    assert!(cx.paint_pixels(area, 0, draw));
    assert_eq!(images.images.len(), 1);
    images.images.swap_remove(0)
}

fn pixel(source: &ImageSource, x: u32, y: u32) -> [u8; 4] {
    let index = usize::try_from((y * source.size().width() + x) * 4).unwrap_or(usize::MAX);
    let bytes = source.rgba8_pixels();
    [
        bytes[index],
        bytes[index + 1],
        bytes[index + 2],
        bytes[index + 3],
    ]
}

#[test]
fn monospaced_text_advances_six_tenths_of_the_size_per_character() {
    paint(Some([0, 0, 0]), None, |px| {
        assert_eq!(px.measure(Face::Mono, 20.0, "abcd"), 48.0);
        assert_eq!(px.measure(Face::Mono, 20.0, "iiii"), 48.0);
    });
}

#[test]
fn proportional_text_is_measured_by_glyph_width_by_default() {
    paint(Some([0, 0, 0]), None, |px| {
        let narrow = px.measure(Face::Regular, 20.0, "iiii");
        let wide = px.measure(Face::Regular, 20.0, "MMMM");
        assert!(narrow > 0.0 && narrow * 2.0 < wide, "{narrow} {wide}");
        assert!(
            px.measure(Face::SemiBold, 20.0, "hello") > px.measure(Face::Regular, 20.0, "hello")
        );
    });
}

#[test]
fn the_ui_font_setting_changes_the_proportional_font_only() {
    let mut inter = (0.0, 0.0);
    let mut plex = (0.0, 0.0);
    paint(Some([0, 0, 0]), None, |px| {
        inter = (
            px.measure(Face::Regular, 20.0, "hello"),
            px.measure(Face::Mono, 20.0, "hello"),
        );
    });
    paint(Some([0, 0, 0]), Some(UiFont::IbmPlexSans), |px| {
        plex = (
            px.measure(Face::Regular, 20.0, "hello"),
            px.measure(Face::Mono, 20.0, "hello"),
        );
    });
    assert_ne!(inter.0, plex.0);
    assert_eq!(inter.1, 60.0);
    assert_eq!(plex.1, 60.0);
}

#[test]
fn invisible_formatting_characters_take_no_space() {
    let text = BundledText::shared(UiFont::Inter);
    let plain = text.width(Face::Regular, 20.0, "ab");
    assert!(plain > 0.0);
    assert_eq!(
        text.width(Face::Regular, 20.0, "a\u{FE0F}\u{200D}\u{7}b"),
        plain
    );
}

#[test]
fn emoji_fall_back_to_the_emoji_font_instead_of_the_missing_glyph_box() {
    let text = BundledText::shared(UiFont::Inter);
    let missing = text.width(Face::Regular, 20.0, "\u{10FFFD}");
    let emoji = text.width(Face::Regular, 20.0, "\u{1F600}");
    assert!(missing > 0.0);
    assert!(
        emoji > 0.0 && (emoji - missing).abs() > 1.0,
        "{emoji} {missing}"
    );
}

#[test]
fn a_size_that_is_not_positive_measures_and_draws_nothing() {
    let image = paint(Some([0, 0, 0]), None, |px| {
        assert_eq!(px.measure(Face::Regular, 0.0, "hello"), 0.0);
        assert_eq!(px.measure(Face::Regular, f32::NAN, "hello"), 0.0);
        assert!(!px.text(
            Face::Regular,
            0.0,
            (4.0, 40.0),
            "hello",
            Color::Rgb(220, 40, 30)
        ));
        assert!(px.line_metrics(Face::Regular, -1.0).is_none());
    });
    assert_eq!(pixel(&image, 10, 34), BACK);
}

#[test]
fn line_metrics_come_from_the_face_font() {
    let text = BundledText::shared(UiFont::Inter);
    let mono = text.line_metrics(Face::Mono, 20.0);
    assert!(
        mono.is_some_and(|line| (line.ascent - 20.4).abs() < 1e-3
            && (line.descent - 6.0).abs() < 1e-3
            && line.line_gap == 0.0),
        "{mono:?}"
    );
}

#[test]
fn text_is_drawn_in_its_color_and_stands_on_the_baseline() {
    let image = paint(Some([0, 0, 0]), None, |px| {
        assert!(px.text(Face::Mono, 40.0, (10.0, 48.0), "I", Color::Rgb(220, 40, 30)));
    });
    assert_eq!(pixel(&image, 22, 34), RED, "the stem of the I");
    assert_eq!(pixel(&image, 4, 34), BACK, "left of the glyph");
    assert_eq!(pixel(&image, 22, 52), BACK, "below the baseline");
    let lowest = (0..64).rev().find(|&y| pixel(&image, 22, y) != BACK);
    assert_eq!(lowest, Some(47));
}

#[test]
fn text_in_a_color_with_no_rgb_value_draws_nothing() {
    let image = paint(None, None, |px| {
        assert!(!px.text(Face::Mono, 40.0, (10.0, 48.0), "I", Color::Default));
    });
    assert_eq!(pixel(&image, 22, 34), BACK);
}

#[test]
fn block_elements_and_arrows_the_ui_font_lacks_come_from_the_monospaced_font() {
    let inter = BundledText::shared(UiFont::Inter);
    let noto = BundledText::shared(UiFont::NotoSans);
    assert_eq!(inter.width(Face::Regular, 20.0, "\u{2588}"), 12.0);
    assert_eq!(noto.width(Face::Regular, 20.0, "\u{2190}"), 12.0);
    assert_eq!(noto.width(Face::SemiBold, 20.0, "\u{2500}"), 12.0);
}

struct Halves;

impl TextMetrics for Halves {
    fn width(&self, _face: Face, px: f32, text: &str) -> f32 {
        px * 0.5 * f32::from(u16::try_from(text.chars().count()).unwrap_or(u16::MAX))
    }
}

#[test]
fn supplied_metrics_measure_but_bundled_text_is_not_drawn_with_them() {
    let area = Rect::new(0, 0, 16, 4);
    let mut grid = Grid::new(16, 4);
    let mut clusters = ClusterTable::new();
    let [r, g, b, _] = BACK;
    grid.fill(
        area,
        Style::new().bg(Color::Rgb(r, g, b)),
        ' ',
        &mut clusters,
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut images = Capture {
        background: Some([0, 0, 0]),
        images: Vec::new(),
    };
    let metrics = Halves;
    let painted = RenderCx::new(&mut grid, &mut clusters, &theme)
        .image_sink(&mut images)
        .render_mode(RenderMode::Graphite)
        .text_metrics(&metrics)
        .paint_pixels(area, 0, |px| {
            assert_eq!(px.measure(Face::Mono, 20.0, "abcd"), 40.0);
            assert!(!px.text(Face::Mono, 40.0, (10.0, 48.0), "I", Color::Rgb(220, 40, 30)));
            assert!(px.line_metrics(Face::Mono, 40.0).is_none());
        });
    assert!(painted);
    assert_eq!(pixel(&images.images[0], 22, 34), BACK);
}

#[test]
fn origins_far_outside_the_surface_draw_nothing_and_do_not_overflow() {
    let image = paint(Some([0, 0, 0]), None, |px| {
        let red = Color::Rgb(220, 40, 30);
        for origin in [
            (0.0, f32::MAX),
            (0.0, -f32::MAX),
            (f32::MAX, 40.0),
            (-f32::MAX, 40.0),
            (0.0, 1.0e30),
            (1_048_577.0, 40.0),
            (f32::INFINITY, 40.0),
            (0.0, f32::NAN),
        ] {
            assert!(
                !px.text(Face::Regular, 40.0, origin, "gjpqy", red),
                "{origin:?}"
            );
        }
        assert!(px.text(Face::Regular, 40.0, (0.0, 1_048_576.0), "gjpqy", red));
        assert!(px.text(
            Face::Regular,
            40.0,
            (-1_048_576.0, -1_048_576.0),
            "gjpqy",
            red
        ));
    });
    let painted = (0..64).any(|y| (0..128).any(|x| pixel(&image, x, y) != BACK));
    assert!(!painted);
}

#[test]
fn sizes_above_4096_are_rejected() {
    paint(Some([0, 0, 0]), None, |px| {
        assert_eq!(px.measure(Face::Regular, 4097.0, "a"), 0.0);
        assert!(px.measure(Face::Regular, 4096.0, "a") > 0.0);
        assert!(!px.text(
            Face::Regular,
            4097.0,
            (0.0, 40.0),
            "a",
            Color::Rgb(220, 40, 30)
        ));
        assert!(px.line_metrics(Face::Regular, 4097.0).is_none());
    });
}
