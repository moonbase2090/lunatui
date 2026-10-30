// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::Cell;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{ProgressBar, ProgressTone, RenderCx, Widget, WidgetState};

struct Capture {
    size: Option<PixelSize>,
    supported: bool,
    accept: bool,
    queries: Cell<usize>,
    attempts: usize,
    images: Vec<(ImageSource, Rect, i32)>,
}

impl ImageSink for Capture {
    fn supports_pixels(&self) -> bool {
        self.supported
    }
    fn pixel_target(&self, _: Rect) -> Option<PixelTarget> {
        self.queries.set(self.queries.get() + 1);
        self.size
            .map(|size| PixelTarget::new(size, false, Some([99, 98, 97])))
    }
    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }
    fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
        self.attempts += 1;
        if self.accept {
            self.images.push((source.clone(), area, depth));
        }
        self.accept
    }
}

fn capture(width: u32, height: u32) -> Capture {
    Capture {
        size: PixelSize::new(width, height),
        supported: true,
        accept: true,
        queries: Cell::new(0),
        attempts: 0,
        images: Vec::new(),
    }
}

fn theme() -> Theme {
    let mut theme = Theme::builtin(Variant::Dark);
    for (token, rgb) in [
        (Token::Accent, [200, 100, 50]),
        (Token::Ok, [0, 200, 0]),
        (Token::Warn, [200, 200, 0]),
        (Token::Fail, [200, 0, 0]),
        (Token::Surface1, [20, 40, 60]),
    ] {
        theme.set_token(token, Color::Rgb(rgb[0], rgb[1], rgb[2]));
    }
    theme
}

fn draw(
    bar: &ProgressBar<'_>,
    width: u16,
    height: u16,
    images: &mut Capture,
) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    bar.render(
        Rect::new(0, 0, width, height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(images),
    );
    (grid, clusters)
}

fn pixel(source: &ImageSource, x: usize, y: usize) -> &[u8] {
    let offset = (y * usize::try_from(source.size().width()).unwrap_or(0) + x) * 4;
    &source.rgba8_pixels()[offset..offset + 4]
}

fn mixed() -> ProgressBar<'static> {
    ProgressBar::new()
        .segment(0.125, ProgressTone::Ok)
        .segment(0.125, ProgressTone::Warn)
        .segment(0.5, ProgressTone::Fail)
}

#[test]
fn native_keeps_the_third_color_hidden_by_one_legacy_cell() {
    let mut images = capture(8, 3);
    let (grid, clusters) = draw(&mixed(), 1, 1, &mut images);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(0, 0, 1, 1));
    assert_eq!(images.images[0].2, 0);
    assert_eq!(grid.row_display(0, &clusters), " ");
    let image = &images.images[0].0;
    for y in 0..3 {
        assert_eq!(pixel(image, 0, y), [0, 200, 0, 255]);
        assert_eq!(pixel(image, 1, y), [200, 200, 0, 255]);
        assert_eq!(pixel(image, 2, y), [200, 0, 0, 255]);
        assert_eq!(pixel(image, 5, y), [200, 0, 0, 255]);
        assert_eq!(pixel(image, 6, y), [20, 40, 60, 255]);
    }
    let (grid, clusters) = draw(&mixed().pixels(false), 1, 1, &mut capture(8, 3));
    assert_eq!(grid.row_display(0, &clusters), "▏");
    assert_eq!(
        grid.get(0, 0).map(|c| (c.style().fg, c.style().bg)),
        Some((Color::Rgb(0, 200, 0), Color::Rgb(200, 200, 0)))
    );
}

#[test]
fn several_edges_in_one_physical_pixel_mix_all_tones_without_a_dark_seam() {
    let mut images = capture(1, 1);
    draw(&mixed(), 1, 1, &mut images);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [130, 60, 15, 255]);
    let mut images = capture(1, 1);
    draw(&mixed().state(WidgetState::Disabled), 1, 1, &mut images);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [65, 30, 8, 255]);
}

#[test]
fn tiny_adjacent_bands_keep_their_area_instead_of_disappearing_on_a_mask_grid() {
    let mut bar = ProgressBar::new();
    for index in 0..256 {
        bar = bar.segment(
            1.0 / 256.0,
            if index % 2 == 0 {
                ProgressTone::Ok
            } else {
                ProgressTone::Fail
            },
        );
    }
    let mut images = capture(1, 1);
    draw(&bar, 1, 1, &mut images);
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [100, 100, 0, 255]);
}

#[test]
fn fractional_boundary_is_antialiased_and_repeated_on_every_row() {
    let mut images = capture(5, 6);
    draw(&ProgressBar::new().ratio(0.5), 2, 2, &mut images);
    for y in 0..6 {
        assert_eq!(pixel(&images.images[0].0, 1, y), [200, 100, 50, 255]);
        assert_eq!(pixel(&images.images[0].0, 2, y), [110, 70, 55, 255]);
        assert_eq!(pixel(&images.images[0].0, 3, y), [20, 40, 60, 255]);
    }
}

#[test]
fn raw_ratios_with_the_same_cell_glyph_keep_different_native_edges() {
    for (ratio, colored) in [(0.01, false), (0.03, true)] {
        let bar = ProgressBar::new().ratio(ratio);
        let mut images = capture(128, 1);
        draw(&bar, 1, 1, &mut images);
        assert_eq!(
            pixel(&images.images[0].0, 2, 0),
            if colored {
                [200, 100, 50, 255]
            } else {
                [20, 40, 60, 255]
            }
        );
        let (grid, clusters) = draw(&bar.pixels(false), 1, 1, &mut capture(128, 1));
        assert_eq!(grid.row_display(0, &clusters), "█");
        assert_eq!(
            grid.get(0, 0).map(|c| c.style().fg),
            Some(Color::Rgb(20, 40, 60))
        );
    }
}

#[test]
fn normalization_and_overfull_segments_clip_in_input_order() {
    for (ratio, expected) in [
        (f64::NAN, [20, 40, 60, 255]),
        (f64::NEG_INFINITY, [20, 40, 60, 255]),
        (-1.0, [20, 40, 60, 255]),
        (f64::INFINITY, [200, 100, 50, 255]),
        (2.0, [200, 100, 50, 255]),
    ] {
        let mut images = capture(8, 1);
        draw(&ProgressBar::new().ratio(ratio), 2, 1, &mut images);
        assert_eq!(pixel(&images.images[0].0, 7, 0), expected);
    }
    let mut images = capture(8, 1);
    draw(
        &ProgressBar::new()
            .segment(f64::NAN, ProgressTone::Accent)
            .segment(-1.0, ProgressTone::Warn)
            .segment(0.75, ProgressTone::Ok)
            .segment(0.75, ProgressTone::Fail),
        2,
        1,
        &mut images,
    );
    assert_eq!(pixel(&images.images[0].0, 5, 0), [0, 200, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 6, 0), [200, 0, 0, 255]);
}

#[test]
fn sweep_uses_the_same_eighth_clock_bounce_and_minimum_length() {
    for (tick, first, last) in [(0, 0, 7), (48, 24, 31), (96, 0, 7), (72, 12, 19)] {
        let mut images = capture(32, 1);
        draw(&ProgressBar::new().indeterminate(tick), 8, 1, &mut images);
        for x in 0..32 {
            assert_eq!(
                pixel(&images.images[0].0, x, 0),
                if (first..=last).contains(&x) {
                    [200, 100, 50, 255]
                } else {
                    [20, 40, 60, 255]
                }
            );
        }
    }
    for tick in [0, 1, u32::MAX] {
        let mut images = capture(8, 1);
        draw(&ProgressBar::new().indeterminate(tick), 1, 1, &mut images);
        assert_eq!(pixel(&images.images[0].0, 7, 0), [200, 100, 50, 255]);
    }
    let mut images = capture(16, 1);
    draw(&ProgressBar::new().indeterminate(8), 2, 1, &mut images);
    assert_eq!(pixel(&images.images[0].0, 7, 0), [20, 40, 60, 255]);
    assert_eq!(pixel(&images.images[0].0, 8, 0), [200, 100, 50, 255]);
}

#[test]
fn focused_changes_only_text_and_disabled_dims_every_semantic_color() {
    for state in [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ] {
        let mut images = capture(8, 1);
        let (grid, clusters) = draw(
            &ProgressBar::new()
                .ratio(0.5)
                .label("L")
                .show_percent(true)
                .state(state),
            11,
            1,
            &mut images,
        );
        assert_eq!(images.images[0].1, Rect::new(2, 0, 4, 1));
        assert_eq!(grid.row_display(0, &clusters), "L      50% ");
        let text = grid
            .get(0, 0)
            .map(lunatui_core::Cell::style)
            .unwrap_or_default();
        assert_eq!(text.attrs.bold(), state == WidgetState::Focused);
        assert_eq!(text.attrs.dim(), state == WidgetState::Disabled);
        assert_eq!(
            pixel(&images.images[0].0, 0, 0),
            if state == WidgetState::Disabled {
                [100, 50, 25, 255]
            } else {
                [200, 100, 50, 255]
            }
        );
        assert_eq!(
            pixel(&images.images[0].0, 7, 0),
            if state == WidgetState::Disabled {
                [10, 20, 30, 255]
            } else {
                [20, 40, 60, 255]
            }
        );
    }
}

#[test]
fn explicit_style_uses_only_filled_foreground() {
    let mut images = capture(8, 1);
    draw(
        &ProgressBar::new().segment(0.5, ProgressTone::Fail).style(
            Style::new()
                .fg(Color::Rgb(11, 22, 33))
                .bg(Color::Default)
                .with_attrs(Attrs::new().set_reverse(true).set_italic(true)),
        ),
        2,
        1,
        &mut images,
    );
    assert_eq!(pixel(&images.images[0].0, 0, 0), [11, 22, 33, 255]);
    assert_eq!(pixel(&images.images[0].0, 7, 0), [20, 40, 60, 255]);
}

#[test]
fn indexed_token_and_prepared_palette_colors_resolve_as_foreground_ink() {
    for (color, expected) in [
        (Color::Indexed(196), [255, 0, 0, 255]),
        (Color::Token(Token::Ok.id()), [0, 200, 0, 255]),
    ] {
        let mut images = capture(8, 1);
        draw(
            &ProgressBar::new().ratio(0.5).style(Style::new().fg(color)),
            2,
            1,
            &mut images,
        );
        assert_eq!(pixel(&images.images[0].0, 0, 0), expected);
    }
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    let mut images = capture(8, 1);
    let mut theme = theme();
    theme.set_token(Token::Accent, Color::Rgb(255, 0, 0));
    let palette = theme.prepare(Profile::Ansi256);
    theme.set_token(Token::Accent, Color::Rgb(0, 0, 255));
    ProgressBar::new().ratio(1.0).render(
        Rect::new(0, 0, 2, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme)
            .palette(&palette)
            .image_sink(&mut images),
    );
    assert_eq!(pixel(&images.images[0].0, 7, 0), [255, 0, 0, 255]);
}

fn seeded(
    bar: &ProgressBar<'_>,
    area: Rect,
    images: &mut Capture,
    theme: &Theme,
) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(14, 7);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 14, 7),
        Style::new().fg(Color::Rgb(7, 8, 9)),
        '.',
        &mut clusters,
    );
    for x in [2, 4, 7] {
        grid.put_str(x, 3, Style::new(), "界", &mut clusters);
    }
    grid.put_str(0, 6, Style::new(), "e\u{301}", &mut clusters);
    grid.clear_dirty();
    grid.set_style(Rect::new(12, 0, 1, 1), Style::new().bg(Color::Rgb(1, 2, 3)));
    bar.render(
        area,
        &mut RenderCx::new(&mut grid, &mut clusters, theme).image_sink(images),
    );
    (grid, clusters)
}

fn same_clusters(a: &ClusterTable, b: &ClusterTable) {
    assert_eq!(a.referenced(), b.referenced());
    assert_eq!(a.overflowed(), b.overflowed());
    for &id in a.referenced() {
        assert_eq!(a.cluster_text(id), b.cluster_text(id));
        assert_eq!(a.cluster_width(id), b.cluster_width(id));
    }
}

#[test]
fn every_early_and_late_decline_keeps_exact_grid_damage_and_clusters() {
    for mode in 0..12 {
        let mut images = capture(20, 8);
        let mut bar = mixed().label("e\u{301}").show_percent(true);
        let mut area = Rect::new(1, 1, 12, 4);
        let mut theme = theme();
        match mode {
            0 => images.accept = false,
            1 => images.supported = false,
            2 => images.size = None,
            3 => images.size = PixelSize::new(4097, 8),
            4 => images.size = PixelSize::new(20, 4097),
            5 => images.size = PixelSize::new(2048, 1024),
            6 => images.size = PixelSize::new(4, 8),
            7 => images.size = PixelSize::new(20, 3),
            8 => bar = bar.style(Style::new()),
            9 => theme.set_token(Token::Surface1, Color::Default),
            10 => area = Rect::new(10, 1, 12, 4),
            _ => area = Rect::new(1, 5, 12, 4),
        }
        let (actual, clusters) = seeded(&bar, area, &mut images, &theme);
        let (expected, other) = seeded(&bar.pixels(false), area, &mut capture(20, 8), &theme);
        assert_eq!(images.images.len(), 0, "mode {mode}");
        assert_eq!(images.attempts, usize::from(mode == 0), "mode {mode}");
        assert_eq!(actual, expected, "mode {mode}");
        same_clusters(&clusters, &other);
    }
}

#[test]
fn accepted_projection_changes_only_track_ink_and_keeps_wide_repairs() {
    let bar = mixed()
        .label("e\u{301}")
        .show_percent(true)
        .state(WidgetState::Disabled);
    let area = Rect::new(1, 1, 12, 4);
    let mut images = capture(20, 8);
    let (actual, clusters) = seeded(&bar, area, &mut images, &theme());
    let (expected, other) = seeded(&bar.pixels(false), area, &mut capture(20, 8), &theme());
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(3, 1, 5, 4));
    same_clusters(&clusters, &other);
    for y in 0..7 {
        assert_eq!(actual.row_dirty(y), expected.row_dirty(y));
        for x in 0..14 {
            let a = actual.get(x, y);
            let e = expected.get(x, y);
            assert_eq!(
                a.map(lunatui_core::Cell::style),
                e.map(lunatui_core::Cell::style)
            );
            assert_eq!(
                a.map(lunatui_core::Cell::meta),
                e.map(lunatui_core::Cell::meta)
            );
            if (3..8).contains(&x) && (1..5).contains(&y) {
                assert_eq!(
                    a.map(|c| c.glyph().decode()),
                    Some(lunatui_core::DecodedGlyph::Inline(' '))
                );
            } else {
                assert_eq!(a, e, "{x},{y}");
            }
        }
    }
}

#[test]
fn only_used_default_colors_decline_and_terminal_background_never_guesses_ink() {
    for ratio in [0.0, 0.5, 1.0] {
        let mut images = capture(8, 1);
        let (grid, clusters) = draw(
            &ProgressBar::new().ratio(ratio).style(Style::new()),
            2,
            1,
            &mut images,
        );
        assert_eq!(images.images.len(), usize::from(ratio == 0.0));
        if ratio != 0.0 {
            assert_eq!(grid.row_display(0, &clusters), "██");
        }
    }
    let mut theme = theme();
    theme.set_token(Token::Surface1, Color::Default);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    let mut images = capture(8, 1);
    ProgressBar::new().ratio(1.0).render(
        Rect::new(0, 0, 2, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut images),
    );
    assert_eq!(pixel(&images.images[0].0, 7, 0), [200, 100, 50, 255]);
}

#[test]
fn geometry_segment_and_work_caps_decline_before_submission() {
    for (width, height) in [(4097, 1), (1, 4097), (257, 256)] {
        let mut images = capture(4096, 1);
        let (actual, _) = draw(&ProgressBar::new().ratio(0.5), width, height, &mut images);
        let (expected, _) = draw(
            &ProgressBar::new().ratio(0.5).pixels(false),
            width,
            height,
            &mut capture(1, 1),
        );
        assert_eq!(images.queries.get(), 0);
        assert_eq!(actual, expected);
    }
    for count in [1100, 65_537] {
        let mut bar = ProgressBar::new();
        for _ in 0..count {
            bar = bar.segment(0.0, ProgressTone::Ok);
        }
        let mut images = capture(4096, 1);
        let (actual, _) = draw(&bar, 1, 1, &mut images);
        let (expected, _) = draw(&bar.pixels(false), 1, 1, &mut capture(1, 1));
        assert_eq!(images.attempts, 0);
        assert_eq!(images.images.len(), 0);
        assert_eq!(actual, expected);
    }
}

#[test]
fn exact_physical_caps_are_supported() {
    for (width, height) in [(4096, 1), (1, 4096), (1024, 1024)] {
        let mut images = capture(width, height);
        draw(&ProgressBar::new().ratio(1.0), 1, 1, &mut images);
        assert_eq!(images.images.len(), 1);
        assert_eq!(pixel(&images.images[0].0, 0, 0), [200, 100, 50, 255]);
    }
}

#[test]
fn off_no_sink_and_empty_areas_keep_cells_without_native_queries() {
    let mut images = capture(8, 1);
    let (expected, clusters) = draw(&mixed().pixels(false), 2, 1, &mut images);
    assert_eq!(images.queries.get(), 0);
    assert_eq!(expected.row_display(0, &clusters), "▎▌");
    let mut actual = Grid::new(2, 1);
    let mut other = ClusterTable::new();
    mixed().render(
        Rect::new(0, 0, 2, 1),
        &mut RenderCx::new(&mut actual, &mut other, &theme()),
    );
    assert_eq!(actual, expected);
    for (width, height) in [(0, 1), (1, 0)] {
        draw(&mixed(), width, height, &mut images);
        assert_eq!(images.queries.get(), 0);
    }
}
