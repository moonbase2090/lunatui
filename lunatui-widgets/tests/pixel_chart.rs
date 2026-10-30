// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Axis, Chart, Dataset, GraphType, LegendPosition, RenderCx, Widget};

struct Capture {
    size: Option<PixelSize>,
    underlay: bool,
    background: Option<[u8; 3]>,
    accept: bool,
    attempts: usize,
    images: Vec<(ImageSource, Rect, i32)>,
}

impl ImageSink for Capture {
    fn supports_pixels(&self) -> bool {
        true
    }

    fn pixel_target(&self, _: Rect) -> Option<PixelTarget> {
        self.size
            .map(|size| PixelTarget::new(size, self.underlay, self.background))
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
        underlay: true,
        background: Some([0, 0, 0]),
        accept: true,
        attempts: 0,
        images: Vec::new(),
    }
}

fn draw(chart: &Chart<'_>, images: &mut Capture, width: u16, height: u16) -> (Grid, ClusterTable) {
    let mut theme = Theme::builtin(Variant::Dark);
    theme.set_token(Token::Surface0, Color::Rgb(11, 22, 33));
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    chart.render(
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
fn existing_chart_calls_use_native_antialiased_lines_and_keep_axes_in_cells() {
    let points = [(0.0, 0.0), (1.0, 1.0)];
    let sets = [Dataset::new(&points)
        .graph_type(GraphType::Line)
        .style(Style::new().fg(Color::Rgb(255, 255, 255)))];
    let chart = Chart::new(&sets);
    let mut images = capture(32, 32);
    let (grid, clusters) = draw(&chart, &mut images, 6, 5);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(1, 0, 5, 4));
    assert_eq!(images.images[0].2, 0);
    assert_eq!(grid.row_display(4, &clusters), "└─────");
    assert_eq!(pixel(&images.images[0].0, 0, 0), [0, 0, 0, 255]);
    let center = pixel(&images.images[0].0, 16, 15);
    assert_eq!(center, [255, 255, 255, 255]);
    let edge = pixel(&images.images[0].0, 16, 16);
    assert!(edge[0] > 0 && edge[0] < 255, "AA edge: {edge:?}");
    assert_eq!(edge, [edge[0], edge[0], edge[0], 255]);
}

#[test]
fn bar_heights_use_the_first_axis_bound_and_keep_a_cell_wide_footprint() {
    let points = [(0.0, 1.0), (0.5, 0.5), (1.0, 0.25)];
    let sets = [Dataset::new(&points)
        .graph_type(GraphType::Bar)
        .style(Style::new().fg(Color::Rgb(240, 120, 60)))];
    let chart = Chart::new(&sets);
    let mut images = capture(30, 20);
    draw(&chart, &mut images, 4, 3);
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 5, 2), [240, 120, 60, 255]);
    assert_eq!(pixel(source, 15, 2), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 15, 12), [240, 120, 60, 255]);
    assert_eq!(pixel(source, 25, 12), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 25, 17), [240, 120, 60, 255]);
    let mut reversed = capture(30, 20);
    draw(
        &chart.y_axis(Axis::new().bounds([1.0, 0.0])),
        &mut reversed,
        4,
        3,
    );
    assert_eq!(pixel(&reversed.images[0].0, 5, 19), [0, 0, 0, 255]);
    assert_eq!(pixel(&reversed.images[0].0, 25, 7), [240, 120, 60, 255]);
}

#[test]
fn later_braille_series_replaces_the_occupied_cell_instead_of_oring_masks() {
    let first = [(0.0, 1.0)];
    let second = [(1.0, 0.0)];
    let sets = [
        Dataset::new(&first).style(Style::new().fg(Color::Rgb(255, 0, 0))),
        Dataset::new(&second).style(Style::new().fg(Color::Rgb(0, 0, 255))),
    ];
    let mut images = capture(20, 20);
    let (grid, clusters) = draw(&Chart::new(&sets).pixels(false), &mut images, 2, 2);
    assert_eq!(grid.row_display(0, &clusters), "│⢀");
    assert_eq!(
        grid.get(1, 0).map(|cell| cell.style().fg),
        Some(Color::Rgb(0, 0, 255))
    );
    assert_eq!(images.attempts, 0);
}

#[test]
fn clipping_joins_finite_neighbors_and_scatter_overlays_only_its_pixels() {
    let line = [(-1.0, 0.5), (f64::NAN, f64::INFINITY), (2.0, 0.5)];
    let scatter = [(0.5, 0.5), (-1.0, 1.0), (2.0, 0.0)];
    let sets = [
        Dataset::new(&line)
            .graph_type(GraphType::Line)
            .style(Style::new().fg(Color::Rgb(255, 0, 0))),
        Dataset::new(&scatter).style(Style::new().fg(Color::Rgb(0, 255, 0))),
    ];
    let mut images = capture(32, 32);
    draw(&Chart::new(&sets), &mut images, 6, 5);
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 0, 0), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 16, 16), [0, 255, 0, 255]);
    assert!(pixel(source, 3, 15)[0] > 0);
    assert_eq!(pixel(source, 3, 15)[1..], [0, 0, 255]);
    assert!(
        source
            .rgba8_pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|rgba| rgba[3] == 255)
    );
}

#[test]
fn legend_labels_stay_in_cells_and_swatches_are_pixels_on_the_real_panel_background() {
    let points = [(0.0, 0.5), (1.0, 0.5)];
    let sets = [Dataset::new(&points)
        .name("cpu %")
        .graph_type(GraphType::Line)
        .style(Style::new().fg(Color::Rgb(255, 0, 0)))];
    let chart = Chart::new(&sets).legend_position(Some(LegendPosition::TopLeft));
    let mut images = capture(290, 90);
    let (grid, clusters) = draw(&chart, &mut images, 30, 10);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].2, -1);
    let row = grid.row_display(1, &clusters);
    assert!(row.contains("cpu %"), "{row}");
    assert!(!row.contains('■'), "pixel path must replace cell swatches");
    assert_eq!(pixel(&images.images[0].0, 25, 15), [255, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 50, 15), [11, 22, 33, 255]);
    assert_eq!(pixel(&images.images[0].0, 150, 15), [0, 0, 0, 255]);
}

#[test]
fn every_declined_pixel_path_reproduces_the_complete_cell_chart_and_legend() {
    let points = [(0.0, 0.0), (1.0, 1.0)];
    for mode in 0..10 {
        let mut images = capture(290, 90);
        let mut style = Style::new().fg(Color::Rgb(200, 100, 50));
        match mode {
            0 => images.size = None,
            1 => images.accept = false,
            2 => images.size = PixelSize::new(4096, 4096),
            3 => images.underlay = false,
            4 => images.background = None,
            5 => style = style.with_attrs(Attrs::new().set_reverse(true)),
            6 => style = style.with_attrs(Attrs::new().set_bold(true)),
            7 => style = style.bg(Color::Rgb(10, 20, 30)),
            9 => style = Style::new(),
            _ => {}
        }
        let sets = [Dataset::new(&points)
            .name("cpu %")
            .graph_type(GraphType::Line)
            .style(style)];
        let mut chart = Chart::new(&sets).legend_position(Some(LegendPosition::TopLeft));
        if mode == 8 {
            chart = chart.x_axis(Axis::new().bounds([0.0, 0.0]));
        }
        let (actual, actual_clusters) = draw(&chart, &mut images, 30, 10);
        let mut cell_sink = capture(290, 90);
        let (expected, expected_clusters) = draw(&chart.pixels(false), &mut cell_sink, 30, 10);
        assert_eq!(actual, expected, "fallback mode {mode}");
        for row in 0..10 {
            assert_eq!(
                actual.row_display(row, &actual_clusters),
                expected.row_display(row, &expected_clusters)
            );
        }
        assert!(images.images.is_empty(), "mode {mode}");
        assert!(actual.row_display(1, &actual_clusters).contains("■ cpu %"));
        assert_eq!(images.attempts, usize::from(mode == 1));
    }
}

#[test]
fn dim_series_and_explicit_chart_background_are_encoded_in_opaque_pixels() {
    let points = [(0.5, 0.5)];
    let sets = [Dataset::new(&points).style(
        Style::new()
            .fg(Color::Rgb(200, 100, 50))
            .with_attrs(Attrs::new().set_dim(true)),
    )];
    let mut images = capture(32, 32);
    draw(
        &Chart::new(&sets).background(Style::new().bg(Color::Rgb(10, 20, 30))),
        &mut images,
        6,
        5,
    );
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 0, 0), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 16, 16), [100, 50, 25, 255]);
}

#[test]
fn excessive_data_work_falls_back_before_submitting_an_image() {
    let points = vec![(0.5, 0.5); 65_537];
    let sets = [Dataset::new(&points)];
    let chart = Chart::new(&sets);
    let mut images = capture(32, 32);
    let (actual, _) = draw(&chart, &mut images, 6, 5);
    let (expected, _) = draw(&chart.pixels(false), &mut capture(32, 32), 6, 5);
    assert_eq!(actual, expected);
    assert_eq!(images.attempts, 0);
    assert_eq!(images.images, []);
}

#[test]
fn repeated_tall_bars_and_crossing_lines_exceed_the_primitive_work_budget() {
    for graph in [GraphType::Line, GraphType::Bar] {
        let points: Vec<_> = (0..2048)
            .map(|index| {
                if index % 2 == 0 {
                    (0.0, 0.0)
                } else {
                    (1.0, 1.0)
                }
            })
            .collect();
        let sets = [Dataset::new(&points)
            .graph_type(graph)
            .style(Style::new().fg(Color::Rgb(255, 0, 0)))];
        let chart = Chart::new(&sets);
        let mut images = capture(1301, 801);
        let (actual, _) = draw(&chart, &mut images, 6, 5);
        let (expected, _) = draw(&chart.pixels(false), &mut capture(1301, 801), 6, 5);
        assert_eq!(actual, expected);
        assert_eq!(images.attempts, 0);
        assert_eq!(images.images, []);
    }
}

#[test]
fn an_ordinary_sixty_four_point_plot_fits_the_work_budget_at_native_resolution() {
    let points: Vec<_> = (0..64)
        .map(|index| (f64::from(index) / 63.0, 0.5))
        .collect();
    let sets = [Dataset::new(&points)
        .graph_type(GraphType::Line)
        .style(Style::new().fg(Color::Rgb(255, 0, 0)))];
    let mut images = capture(1301, 801);
    draw(&Chart::new(&sets), &mut images, 101, 33);
    assert_eq!(images.attempts, 1);
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 650, 400), [255, 0, 0, 255]);
}
