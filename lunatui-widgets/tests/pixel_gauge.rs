// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::Cell;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Gauge, RenderCx, Widget};

struct Capture {
    size: Option<PixelSize>,
    background: Option<[u8; 3]>,
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
            .map(|size| PixelTarget::new(size, false, self.background))
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
        background: Some([20, 40, 60]),
        supported: true,
        accept: true,
        queries: Cell::new(0),
        attempts: 0,
        images: Vec::new(),
    }
}

fn fill() -> Style {
    Style::new()
        .fg(Color::Rgb(200, 100, 50))
        .bg(Color::Rgb(1, 2, 3))
}
fn track() -> Style {
    Style::new()
        .fg(Color::Rgb(3, 240, 7))
        .bg(Color::Rgb(20, 40, 60))
}
fn gauge(ratio: f32) -> Gauge {
    Gauge::new(ratio).gauge_style(fill()).track_style(track())
}

fn draw(gauge: &Gauge, images: &mut Capture, width: u16, height: u16) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut theme = Theme::builtin(Variant::Dark);
    theme.set_token(Token::Accent, Color::Rgb(185, 167, 240));
    gauge.render(
        Rect::new(0, 0, width, height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(images),
    );
    (grid, clusters)
}

fn pixel(source: &ImageSource, x: usize, y: usize) -> &[u8] {
    let offset = (y * usize::try_from(source.size().width()).unwrap_or(0) + x) * 4;
    &source.rgba8_pixels()[offset..offset + 4]
}

#[test]
fn continuous_native_edge_is_antialiased_without_sub_cell() {
    let mut images = capture(8, 2);
    let (grid, clusters) = draw(&gauge(0.3125), &mut images, 4, 1);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(0, 0, 4, 1));
    assert_eq!(images.images[0].2, 0);
    assert_eq!(grid.row_display(0, &clusters), "    ");
    let image = &images.images[0].0;
    assert_eq!(pixel(image, 1, 0), [200, 100, 50, 255]);
    assert_eq!(pixel(image, 2, 0), [110, 70, 55, 255]);
    assert_eq!(pixel(image, 3, 0), [20, 40, 60, 255]);
}

#[test]
fn partial_cell_background_stays_distinct_from_the_track() {
    let mut images = capture(8, 2);
    let (grid, _) = draw(&gauge(0.3125).sub_cell(true), &mut images, 4, 1);
    let image = &images.images[0].0;
    assert_eq!(pixel(image, 2, 1), [101, 51, 27, 255]);
    assert_eq!(pixel(image, 3, 1), [1, 2, 3, 255]);
    assert_eq!(pixel(image, 4, 1), [20, 40, 60, 255]);
    assert_eq!(grid.get(1, 0).map(lunatui_core::Cell::style), Some(fill()));
    assert_eq!(grid.get(2, 0).map(lunatui_core::Cell::style), Some(track()));
}

#[test]
fn raw_fill_uses_fill_ink_even_when_legacy_writes_only_track_cells() {
    let mut images = capture(16, 2);
    let (grid, clusters) = draw(&gauge(0.125), &mut images, 4, 1);
    assert_eq!(grid.row_display(0, &clusters), "    ");
    assert_eq!(grid.get(0, 0).map(lunatui_core::Cell::style), Some(track()));
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [200, 100, 50, 255]);
    assert_eq!(pixel(&images.images[0].0, 2, 0), [20, 40, 60, 255]);
}

#[test]
fn unsupported_fill_attributes_decline_even_without_a_legacy_fill_cell() {
    for attrs in [
        Attrs::new().set_reverse(true),
        Attrs::new().set_bold(true),
        Attrs::new().set_italic(true),
    ] {
        let mut images = capture(16, 2);
        let (grid, clusters) = draw(
            &gauge(0.125).gauge_style(fill().with_attrs(attrs)),
            &mut images,
            4,
            1,
        );
        assert_eq!(grid.row_display(0, &clusters), "    ");
        assert_eq!(images.attempts, 0);
        assert_eq!(images.images.len(), 0);
    }
}

#[test]
fn equal_whole_cell_counts_keep_distinct_raw_widths() {
    let mut first = capture(16, 2);
    let mut second = capture(16, 2);
    draw(&gauge(0.25), &mut first, 4, 1);
    draw(&gauge(0.375), &mut second, 4, 1);
    assert_eq!(pixel(&first.images[0].0, 4, 0), [20, 40, 60, 255]);
    assert_eq!(pixel(&second.images[0].0, 4, 0), [200, 100, 50, 255]);
    let (grid, clusters) = draw(&gauge(0.25).pixels(false), &mut capture(16, 2), 4, 1);
    assert_eq!(grid.row_display(0, &clusters), "█   ");
    let (grid, clusters) = draw(&gauge(0.375).pixels(false), &mut capture(16, 2), 4, 1);
    assert_eq!(grid.row_display(0, &clusters), "█   ");
}

#[test]
fn dim_halves_foreground_and_keeps_backgrounds() {
    let mut images = capture(8, 2);
    draw(
        &gauge(0.5).gauge_style(fill().with_attrs(Attrs::new().set_dim(true))),
        &mut images,
        4,
        1,
    );
    assert_eq!(pixel(&images.images[0].0, 0, 1), [100, 50, 25, 255]);
    assert_eq!(pixel(&images.images[0].0, 5, 1), [20, 40, 60, 255]);
}

#[test]
fn missing_track_uses_terminal_default_background_but_not_default_foreground() {
    let mut images = capture(8, 2);
    draw(&Gauge::new(0.5).gauge_style(fill()), &mut images, 4, 1);
    assert_eq!(pixel(&images.images[0].0, 5, 1), [20, 40, 60, 255]);
    let mut images = capture(8, 2);
    let (grid, clusters) = draw(&Gauge::new(0.5), &mut images, 4, 1);
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), "██  ");
}

fn seeded(gauge: &Gauge, area: Rect, images: &mut Capture) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(14, 7);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 14, 7),
        Style::new().fg(Color::Rgb(7, 8, 9)),
        '.',
        &mut clusters,
    );
    grid.put_str(1, 3, Style::new(), "界", &mut clusters);
    grid.put_str(4, 3, Style::new(), "界", &mut clusters);
    grid.put_str(8, 3, Style::new(), "界", &mut clusters);
    grid.put_str(0, 6, Style::new(), "e\u{301}", &mut clusters);
    grid.clear_dirty();
    grid.set_style(Rect::new(12, 0, 1, 1), track());
    gauge.render(
        area,
        &mut RenderCx::new(&mut grid, &mut clusters, &Theme::builtin(Variant::Dark))
            .image_sink(images),
    );
    (grid, clusters)
}

#[test]
fn every_early_and_late_decline_keeps_exact_cells_clusters_and_dirty_spans() {
    for mode in 0..14 {
        let mut images = capture(28, 4);
        let mut gauge = gauge(0.375).sub_cell(true);
        let mut area = Rect::new(2, 1, 7, 4);
        match mode {
            0 => images.accept = false,
            1 => images.size = None,
            2 => images.supported = false,
            3 => {
                images.background = None;
                gauge = gauge.track_style(Style::new());
            }
            4 => images.size = PixelSize::new(4097, 4),
            5 => images.size = PixelSize::new(28, 4097),
            6 => images.size = PixelSize::new(2048, 1024),
            7 => images.size = PixelSize::new(6, 4),
            8 => gauge = gauge.gauge_style(fill().with_attrs(Attrs::new().set_reverse(true))),
            9 => gauge = gauge.track_style(track().with_attrs(Attrs::new().set_italic(true))),
            10 => gauge = gauge.gauge_style(Style::new()),
            11 => area = Rect::new(9, 1, 7, 4),
            12 => area = Rect::new(2, 5, 7, 4),
            _ => gauge = Gauge::new(f32::NAN).gauge_style(fill()),
        }
        let (actual, clusters) = seeded(&gauge, area, &mut images);
        let (expected, other_clusters) = seeded(&gauge.pixels(false), area, &mut capture(28, 4));
        assert!(images.images.is_empty(), "mode {mode}");
        assert_eq!(images.attempts, usize::from(mode == 0), "mode {mode}");
        for y in 0..7 {
            assert_eq!(
                actual.row_bits(y),
                expected.row_bits(y),
                "mode {mode}, row {y}"
            );
            assert_eq!(
                actual.row_dirty(y),
                expected.row_dirty(y),
                "mode {mode}, row {y}"
            );
            assert_eq!(
                actual.row_display(y, &clusters),
                expected.row_display(y, &other_clusters)
            );
        }
    }
}

#[test]
fn acceptance_changes_only_known_row_ink_and_preserves_wide_repairs() {
    let area = Rect::new(2, 1, 7, 4);
    let gauge = gauge(0.375).sub_cell(true);
    let mut images = capture(28, 4);
    let (actual, clusters) = seeded(&gauge, area, &mut images);
    let (expected, _) = seeded(&gauge.pixels(false), area, &mut capture(28, 4));
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(2, 3, 7, 1));
    assert_eq!(actual.row_display(3, &clusters), ".         ....");
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
            if y == 3 && (2..5).contains(&x) {
                assert_eq!(
                    a.map(|cell| cell.glyph().decode()),
                    Some(lunatui_core::DecodedGlyph::Inline(' '))
                );
            } else {
                assert_eq!(a, e, "{x},{y}");
            }
        }
    }
}

#[test]
fn optional_block_and_other_inner_rows_stay_outside_the_image() {
    let mut images = capture(16, 4);
    let (grid, clusters) = draw(
        &gauge(0.5).block(Block::bordered().title("load")),
        &mut images,
        10,
        6,
    );
    assert_eq!(images.images[0].1, Rect::new(1, 3, 8, 1));
    assert_eq!(grid.row_display(0, &clusters), "┌load────┐");
    assert_eq!(grid.row_display(5, &clusters), "└────────┘");
    assert_eq!(grid.row_display(2, &clusters), "│        │");
}

#[test]
fn zero_nan_and_empty_inner_areas_submit_nothing_and_full_ratio_fills() {
    for ratio in [0.0, f32::NAN, f32::NEG_INFINITY] {
        let mut images = capture(8, 2);
        let (grid, clusters) = draw(&gauge(ratio), &mut images, 4, 1);
        assert_eq!(grid.row_display(0, &clusters), "    ");
        assert_eq!(images.queries.get(), 0);
    }
    let mut images = capture(8, 2);
    draw(&gauge(f32::INFINITY), &mut images, 4, 1);
    assert_eq!(pixel(&images.images[0].0, 7, 1), [200, 100, 50, 255]);
    let mut images = capture(8, 2);
    draw(&gauge(0.5).block(Block::bordered()), &mut images, 2, 2);
    assert_eq!(images.queries.get(), 0);
}

#[test]
fn pixels_false_and_no_sink_keep_legacy_eighths_without_queries() {
    let mut images = capture(8, 2);
    let (grid, clusters) = draw(
        &gauge(0.3125).sub_cell(true).pixels(false),
        &mut images,
        4,
        1,
    );
    assert_eq!(grid.row_display(0, &clusters), "█▎  ");
    assert_eq!(images.queries.get(), 0);
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    gauge(0.3125).sub_cell(true).render(
        Rect::new(0, 0, 4, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &Theme::builtin(Variant::Dark)),
    );
    assert_eq!(grid.row_display(0, &clusters), "█▎  ");
}

#[test]
fn indexed_and_token_foregrounds_resolve_without_changing_cell_styles() {
    for (color, want) in [
        (Color::Indexed(196), [255, 0, 0, 255]),
        (Color::Token(Token::Accent.id()), [185, 167, 240, 255]),
    ] {
        let mut images = capture(8, 2);
        let style = fill().fg(color);
        let (grid, _) = draw(&gauge(0.5).gauge_style(style), &mut images, 4, 1);
        assert_eq!(pixel(&images.images[0].0, 0, 0), want);
        assert_eq!(grid.get(0, 0).map(lunatui_core::Cell::style), Some(style));
    }
}

#[test]
fn exact_target_caps_work_and_wider_logical_rows_decline_before_querying() {
    for (width, height) in [(4096, 1), (1, 4096), (1024, 1024)] {
        let mut images = capture(width, height);
        draw(&gauge(1.0), &mut images, 1, 1);
        assert_eq!(images.images.len(), 1);
        assert_eq!(pixel(&images.images[0].0, 0, 0), [200, 100, 50, 255]);
    }
    let mut images = capture(4096, 1);
    let (grid, clusters) = draw(&gauge(0.5), &mut images, 4097, 1);
    assert_eq!(images.queries.get(), 0);
    assert_eq!(
        grid.row_display(0, &clusters)
            .chars()
            .filter(|ch| *ch == '█')
            .count(),
        2048
    );
}

#[test]
fn shrinking_and_zero_frames_clear_previous_cell_ink_without_touching_other_rows() {
    let mut grid = Grid::new(4, 3);
    let mut clusters = ClusterTable::new();
    grid.fill(Rect::new(0, 0, 4, 3), track(), '.', &mut clusters);
    for (step, ratio) in [1.0, 0.125, 0.0, 1.0].into_iter().enumerate() {
        let mut images = capture(16, 2);
        gauge(ratio).render(
            Rect::new(0, 0, 4, 3),
            &mut RenderCx::new(&mut grid, &mut clusters, &Theme::builtin(Variant::Dark))
                .image_sink(&mut images),
        );
        assert_eq!(grid.row_display(0, &clusters), "....");
        assert_eq!(grid.row_display(1, &clusters), "    ");
        assert_eq!(grid.row_display(2, &clusters), "....");
        assert_eq!(images.images.len(), usize::from(ratio > 0.0));
        if step == 1 {
            assert_eq!(pixel(&images.images[0].0, 1, 0), [200, 100, 50, 255]);
            assert_eq!(pixel(&images.images[0].0, 2, 0), [20, 40, 60, 255]);
        }
    }
}
