// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageId, ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Marker, PieChart, PieSlice, PixelPieChart, RenderCx, Widget};

struct Capture {
    size: Option<PixelSize>,
    underlay: bool,
    accept: bool,
    requests: Vec<(ImageSource, u64, Rect, i32)>,
}
impl ImageSink for Capture {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, _: Rect) -> Option<PixelTarget> {
        self.size
            .map(|size| PixelTarget::new(size, self.underlay, Some([0, 0, 0])))
    }
    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        self.requests.push((
            request.source().clone(),
            request.generation(),
            request.area(),
            request.depth(),
        ));
        self.accept
    }
}
fn sink() -> Capture {
    Capture {
        size: PixelSize::new(32, 32),
        underlay: true,
        accept: true,
        requests: vec![],
    }
}
fn chart() -> PieChart {
    PieChart::new([PieSlice::new("all", 1.0).color(Color::Rgb(255, 255, 255))]).legend(false)
}
fn widget(chart: PieChart) -> Result<PixelPieChart, Box<dyn std::error::Error>> {
    Ok(PixelPieChart::new(
        ImageId::new(7).ok_or("nonzero ID")?,
        chart,
    ))
}
fn draw(chart: &impl Widget, images: &mut Capture, bg: Color) -> Grid {
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 4, 2),
        Style::new().bg(bg),
        ' ',
        &mut clusters,
    );
    let theme = Theme::builtin(Variant::Dark);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(images);
    chart.render(Rect::new(0, 0, 4, 2), &mut cx);
    grid
}
fn pixel(source: &ImageSource, x: usize, y: usize) -> &[u8] {
    let start = (y * usize::try_from(source.size().width()).unwrap_or(0) + x) * 4;
    &source.rgba8_pixels()[start..start + 4]
}

#[test]
fn circular_edge_is_blended_and_hole_preserves_the_panel() -> Result<(), Box<dyn std::error::Error>>
{
    let mut images = sink();
    draw(
        &widget(chart().donut(0.5))?,
        &mut images,
        Color::Rgb(0, 0, 0),
    );
    let source = &images.requests[0].0;
    assert_eq!(pixel(source, 0, 0), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 16, 16), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 16, 4), [255, 255, 255, 255]);
    let edge = pixel(source, 0, 12);
    assert!(
        edge[0] > 0 && edge[0] < 255,
        "AA edge must blend white into black: {edge:?}"
    );
    assert_eq!(edge, [edge[0], edge[0], edge[0], 255]);
    let inner_edge = pixel(source, 8, 13);
    assert!(
        inner_edge[0] > 0 && inner_edge[0] < 255,
        "hole must also be antialiased: {inner_edge:?}"
    );
    Ok(())
}

#[test]
fn raster_cache_tracks_values_colors_and_backdrop_without_caller_revisions()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = sink();
    let mut pie = widget(chart())?;
    draw(&pie, &mut images, Color::Rgb(0, 0, 0));
    draw(&pie, &mut images, Color::Rgb(0, 0, 0));
    assert_eq!(images.requests[0].1, images.requests[1].1);
    assert_eq!(images.requests[0].0, images.requests[1].0);
    draw(&pie, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(images.requests[2].1, 1);
    assert_eq!(pixel(&images.requests[2].0, 0, 0), [30, 40, 50, 255]);
    pie.set_chart(
        PieChart::new([PieSlice::new("all", 1.0).color(Color::Rgb(255, 0, 0))]).legend(false),
    );
    draw(&pie, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(images.requests[3].1, 2);
    assert_eq!(pixel(&images.requests[3].0, 16, 16), [255, 0, 0, 255]);
    Ok(())
}

#[test]
fn rejected_missing_or_excessive_targets_render_the_same_braille()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in 0..4 {
        let mut images = sink();
        match mode {
            0 => images.size = None,
            1 => images.accept = false,
            2 => images.size = PixelSize::new(4096, 4096),
            _ => {
                images.underlay = false;
            }
        }
        let spec = chart().donut(0.5).center("1");
        let actual = draw(&widget(spec.clone())?, &mut images, Color::Rgb(0, 0, 0));
        let expected = draw(
            &spec.marker(Marker::Braille),
            &mut sink(),
            Color::Rgb(0, 0, 0),
        );
        assert_eq!(actual, expected, "fallback mode {mode}");
        if mode != 1 {
            assert_eq!(images.requests, Vec::new());
        }
    }
    Ok(())
}

#[test]
fn immediate_protocol_retains_existing_text_and_kitty_requests_underlay_for_center()
-> Result<(), Box<dyn std::error::Error>> {
    let pie = widget(chart().donut(0.8).center("1"))?;
    let mut images = sink();
    draw(&pie, &mut images, Color::Rgb(0, 0, 0));
    assert_eq!(images.requests[0].3, -1);
    let mut images = sink();
    images.underlay = false;
    let spec = chart();
    let mut grid = Grid::new(4, 2);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 4, 2),
        Style::new().bg(Color::Rgb(0, 0, 0)),
        ' ',
        &mut clusters,
    );
    grid.put_str(0, 0, Style::new(), "x", &mut clusters);
    let theme = Theme::builtin(Variant::Dark);
    widget(spec)?.render(
        Rect::new(0, 0, 4, 2),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut images),
    );
    assert_eq!(images.requests, Vec::new());
    Ok(())
}

#[test]
fn explicit_pixel_opt_out_renders_braille_without_submission()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = sink();
    let actual = draw(
        &widget(chart())?.pixels(false),
        &mut images,
        Color::Rgb(0, 0, 0),
    );
    let expected = draw(
        &chart().marker(Marker::Braille),
        &mut sink(),
        Color::Rgb(0, 0, 0),
    );
    assert_eq!(actual, expected);
    assert_eq!(images.requests, Vec::new());
    Ok(())
}
