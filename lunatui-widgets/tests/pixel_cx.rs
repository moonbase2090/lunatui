// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::Cell;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::RenderCx;
use lunatui_widgets::graphite::{Face, Rect as PixelRect, TextMetrics};

const CELL: (u32, u32) = (8, 16);
const RED: Color = Color::Rgb(220, 40, 30);
const BACK: [u8; 3] = [10, 20, 30];

struct Capture {
    underlay: bool,
    background: Option<[u8; 3]>,
    attempts: usize,
    images: Vec<(ImageSource, Rect, i32)>,
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
            self.underlay,
            self.background,
        ))
    }
    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }
    fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
        self.attempts += 1;
        self.images.push((source.clone(), area, depth));
        true
    }
}

fn capture() -> Capture {
    Capture {
        underlay: true,
        background: Some([0, 0, 0]),
        attempts: 0,
        images: Vec::new(),
    }
}

fn backdrop(grid: &mut Grid, clusters: &mut ClusterTable, area: Rect, rgb: [u8; 3]) {
    let [r, g, b] = rgb;
    grid.fill(area, Style::new().bg(Color::Rgb(r, g, b)), ' ', clusters);
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

fn snapshot(grid: &Grid) -> Vec<String> {
    (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .map(|(x, y)| format!("{:?}", grid.get(x, y)))
        .collect()
}

fn paint(
    mode: RenderMode,
    images: &mut Capture,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    area: Rect,
    depth: i32,
    draw: impl FnOnce(&mut lunatui_widgets::PixelCx<'_>),
) -> bool {
    let theme = Theme::builtin(Variant::Dark);
    RenderCx::new(grid, clusters, &theme)
        .image_sink(images)
        .render_mode(mode)
        .paint_pixels(area, depth, draw)
}

#[test]
fn cell_mode_paints_nothing_even_with_a_pixel_sink() {
    let area = Rect::new(0, 0, 4, 2);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let before = snapshot(&grid);
    let mut images = capture();
    let called = Cell::new(false);
    let painted = paint(
        RenderMode::Cell,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |_| {
            called.set(true);
        },
    );
    assert!(!painted);
    assert!(!called.get(), "the closure ran in Cell mode");
    assert_eq!(images.attempts, 0);
    assert_eq!(snapshot(&grid), before);
}

#[test]
fn graphite_mode_paints_over_the_cell_backdrop_and_submits_one_image() {
    let area = Rect::new(1, 1, 4, 2);
    let mut grid = Grid::new(6, 4);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    let painted = paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert_eq!((px.width(), px.height()), (32, 32));
            assert_eq!(
                (px.cell_size().width_px(), px.cell_size().height_px()),
                (8, 16)
            );
            assert!(px.fill_rect(
                PixelRect {
                    x: 4,
                    y: 4,
                    w: 16,
                    h: 8
                },
                0.0,
                RED
            ));
        },
    );
    assert!(painted);
    assert_eq!(images.images.len(), 1);
    let (source, placed, depth) = &images.images[0];
    assert_eq!((*placed, *depth), (area, 0));
    assert_eq!((source.size().width(), source.size().height()), (32, 32));
    assert_eq!(pixel(source, 10, 8), [220, 40, 30, 255]);
    assert_eq!(pixel(source, 2, 2), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 25, 8), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 10, 20), [10, 20, 30, 255]);
}

#[test]
fn each_cell_keeps_its_own_background_under_the_pixels() {
    let area = Rect::new(0, 0, 2, 1);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, Rect::new(0, 0, 1, 1), [1, 2, 3]);
    backdrop(
        &mut grid,
        &mut clusters,
        Rect::new(1, 0, 1, 1),
        [200, 201, 202],
    );
    let mut images = capture();
    assert!(paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |_| {}
    ));
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 3, 8), [1, 2, 3, 255]);
    assert_eq!(pixel(source, 12, 8), [200, 201, 202, 255]);
}

#[test]
fn an_area_over_the_pixel_limit_falls_back_with_cells_untouched() {
    let area = Rect::new(0, 0, 300, 100);
    let mut grid = Grid::new(300, 100);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    grid.put_str(3, 3, Style::new().fg(RED), "kept", &mut clusters);
    let before = snapshot(&grid);
    let mut images = capture();
    let called = Cell::new(false);
    let painted = paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        -1,
        |_| {
            called.set(true);
        },
    );
    assert!(!painted);
    assert!(!called.get());
    assert_eq!(images.attempts, 0);
    assert_eq!(snapshot(&grid), before);
}

#[test]
fn text_in_the_area_needs_an_underlay_and_a_negative_depth() {
    let area = Rect::new(0, 0, 4, 1);
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    grid.put_str(
        0,
        0,
        Style::new().bg(Color::Rgb(10, 20, 30)),
        "ab",
        &mut clusters,
    );
    let before = snapshot(&grid);
    let mut images = capture();
    assert!(!paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |_| {}
    ));
    images.underlay = false;
    assert!(!paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        -1,
        |_| {}
    ));
    assert_eq!(images.attempts, 0);
    assert_eq!(snapshot(&grid), before);
    images.underlay = true;
    assert!(paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        -1,
        |_| {}
    ));
    assert_eq!(images.images[0].2, -1);
    assert_eq!(snapshot(&grid), before);
}

#[test]
fn rounded_corners_leave_the_corner_pixel_and_square_ones_fill_it() {
    let area = Rect::new(0, 0, 4, 2);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    let whole = PixelRect {
        x: 0,
        y: 0,
        w: 32,
        h: 32,
    };
    paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert!(px.fill_rect(whole, 8.0, RED));
        },
    );
    paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert!(px.fill_rect(whole, 0.0, RED));
        },
    );
    let (round, square) = (&images.images[0].0, &images.images[1].0);
    assert_eq!(pixel(round, 0, 0), [10, 20, 30, 255]);
    assert_eq!(pixel(round, 31, 31), [10, 20, 30, 255]);
    assert_eq!(pixel(round, 16, 0), [220, 40, 30, 255]);
    assert_eq!(pixel(round, 16, 16), [220, 40, 30, 255]);
    assert_eq!(pixel(square, 0, 0), [220, 40, 30, 255]);
    assert_eq!(pixel(square, 31, 31), [220, 40, 30, 255]);
}

#[test]
fn a_stroke_stays_inside_its_rectangle_and_leaves_the_middle() {
    let area = Rect::new(0, 0, 4, 2);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert!(px.stroke_rect(
                PixelRect {
                    x: 4,
                    y: 4,
                    w: 24,
                    h: 24
                },
                0.0,
                2.0,
                RED
            ));
        },
    );
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 4, 16), [220, 40, 30, 255]);
    assert_eq!(pixel(source, 5, 16), [220, 40, 30, 255]);
    assert_eq!(pixel(source, 27, 16), [220, 40, 30, 255]);
    assert_eq!(pixel(source, 3, 16), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 28, 16), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 16, 16), [10, 20, 30, 255]);
}

#[test]
fn a_line_and_an_area_cover_the_pixels_they_describe() {
    let area = Rect::new(0, 0, 4, 2);
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert!(px.line((0.0, 4.0), (32.0, 4.0), 2.0, RED));
            assert!(px.area(
                &[(0.0, 20.0), (16.0, 12.0), (32.0, 20.0)],
                32.0,
                Color::Rgb(0, 200, 0)
            ));
            assert!(!px.area(&[(0.0, 20.0)], 32.0, RED));
            assert!(!px.line((0.0, 0.0), (8.0, 8.0), 0.0, RED));
        },
    );
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 10, 3), [220, 40, 30, 255]);
    assert_eq!(pixel(source, 10, 4), [220, 40, 30, 255]);
    assert_eq!(pixel(source, 10, 7), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 16, 28), [0, 200, 0, 255]);
    assert_eq!(pixel(source, 16, 10), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 2, 16), [10, 20, 30, 255]);
}

#[test]
fn a_color_with_no_rgb_value_draws_nothing() {
    let area = Rect::new(0, 0, 2, 1);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    images.background = None;
    paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert!(!px.fill_rect(
                PixelRect {
                    x: 0,
                    y: 0,
                    w: 16,
                    h: 16
                },
                0.0,
                Color::Default
            ));
        },
    );
    assert_eq!(pixel(&images.images[0].0, 4, 4), [10, 20, 30, 255]);
}

#[test]
fn text_is_measured_in_cells_by_default_and_cut_with_an_ellipsis() {
    let area = Rect::new(0, 0, 8, 1);
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    paint(
        RenderMode::Graphite,
        &mut images,
        &mut grid,
        &mut clusters,
        area,
        0,
        |px| {
            assert_eq!(px.measure(Face::Regular, 13.0, "hello"), 40.0);
            assert_eq!(px.fit(Face::Regular, 13.0, "hello", 40.0), "hello");
            assert_eq!(px.fit(Face::Regular, 13.0, "hello world", 40.0), "hell…");
            assert_eq!(
                px.fit(Face::Regular, 13.0, "e\u{301}xyz", 16.0),
                "e\u{301}…"
            );
            assert_eq!(px.fit(Face::Regular, 13.0, "hello", 4.0), "");
        },
    );
    assert_eq!(images.images.len(), 1);
}

struct Proportional;

impl TextMetrics for Proportional {
    fn width(&self, _face: Face, px: f32, text: &str) -> f32 {
        text.chars()
            .map(|c| if c == 'i' { px * 0.25 } else { px * 0.5 })
            .sum()
    }
}

#[test]
fn supplied_text_metrics_replace_the_cell_measure() {
    let area = Rect::new(0, 0, 8, 1);
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    backdrop(&mut grid, &mut clusters, area, BACK);
    let mut images = capture();
    let theme = Theme::builtin(Variant::Dark);
    let metrics = Proportional;
    let painted = RenderCx::new(&mut grid, &mut clusters, &theme)
        .image_sink(&mut images)
        .render_mode(RenderMode::Graphite)
        .text_metrics(&metrics)
        .paint_pixels(area, 0, |px| {
            assert_eq!(px.measure(Face::Regular, 16.0, "iii"), 12.0);
            assert_eq!(px.fit(Face::Regular, 16.0, "iiiiwide", 24.0), "iiii…");
        });
    assert!(painted);
}
