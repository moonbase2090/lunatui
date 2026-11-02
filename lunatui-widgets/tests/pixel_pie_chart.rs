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
fn native_circle_uses_the_measured_terminal_cell_aspect() -> Result<(), Box<dyn std::error::Error>>
{
    let mut images = sink();
    images.size = PixelSize::new(32, 16);
    draw(&widget(chart())?, &mut images, Color::Rgb(0, 0, 0));
    let source = &images.requests[0].0;
    let mut bounds = [usize::MAX, usize::MAX, 0, 0];
    for y in 0..16 {
        for x in 0..32 {
            if pixel(source, x, y)[0] > 0 {
                bounds[0] = bounds[0].min(x);
                bounds[1] = bounds[1].min(y);
                bounds[2] = bounds[2].max(x);
                bounds[3] = bounds[3].max(y);
            }
        }
    }
    let width = bounds[2] - bounds[0] + 1;
    let height = bounds[3] - bounds[1] + 1;
    assert!(
        width.abs_diff(height) <= 1,
        "native disc bounds: {bounds:?}"
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

struct LegendCapture {
    accept_pie: bool,
    accept_strip: bool,
    underlay: bool,
    pies: Vec<Rect>,
    strips: Vec<(Rect, ImageSource)>,
}

impl Default for LegendCapture {
    fn default() -> Self {
        Self {
            accept_pie: true,
            accept_strip: true,
            underlay: true,
            pies: Vec::new(),
            strips: Vec::new(),
        }
    }
}

impl ImageSink for LegendCapture {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        PixelSize::new(u32::from(area.width) * 8, u32::from(area.height) * 16)
            .map(|size| PixelTarget::new(size, self.underlay, Some([0, 0, 0])))
    }
    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        self.pies.push(request.area());
        self.accept_pie
    }
    fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
        assert_eq!(depth, 0);
        self.strips.push((area, source.clone()));
        self.accept_strip
    }
}

fn legend_spec() -> PieChart {
    PieChart::new([
        PieSlice::new("red", 1.0).color(Color::Rgb(255, 0, 0)),
        PieSlice::new("green", 1.0).color(Color::Rgb(0, 255, 0)),
    ])
    .cell_aspect(2.0)
}

fn legend_draw(pie: &impl Widget, images: &mut LegendCapture, second_bg: Color) -> Grid {
    let mut grid = Grid::new(40, 10);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 40, 10),
        Style::new().bg(Color::Rgb(10, 20, 30)),
        ' ',
        &mut clusters,
    );
    grid.fill(
        Rect::new(22, 5, 1, 1),
        Style::new().bg(second_bg),
        ' ',
        &mut clusters,
    );
    pie.render(
        Rect::new(0, 0, 40, 10),
        &mut RenderCx::new(&mut grid, &mut clusters, &Theme::builtin(Variant::Dark))
            .image_sink(images),
    );
    grid
}

fn glyph(grid: &Grid, x: u16, y: u16) -> Option<lunatui_core::DecodedGlyph> {
    grid.get(x, y).map(|cell| cell.glyph().decode())
}

#[test]
fn native_legend_has_real_circles_and_preserves_each_row_background()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = LegendCapture::default();
    let grid = legend_draw(&widget(legend_spec())?, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(images.pies, [Rect::new(0, 0, 20, 10)]);
    assert_eq!(
        images.strips.len(),
        1,
        "native legend must submit its own raster"
    );
    assert_eq!(images.strips[0].0, Rect::new(22, 4, 1, 2));
    let source = &images.strips[0].1;
    assert_eq!(source.size(), PixelSize::new(8, 32).ok_or("fixture size")?);
    assert_eq!(pixel(source, 0, 0), [10, 20, 30, 255]);
    assert_eq!(pixel(source, 4, 8), [255, 0, 0, 255]);
    assert_eq!(pixel(source, 0, 16), [30, 40, 50, 255]);
    assert_eq!(pixel(source, 4, 24), [0, 255, 0, 255]);
    let edge = pixel(source, 2, 6);
    assert!(
        edge[0] > 10 && edge[0] < 255,
        "circle edge must blend: {edge:?}"
    );
    assert_eq!(
        glyph(&grid, 22, 4),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    assert_eq!(
        glyph(&grid, 22, 5),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    assert_eq!(
        glyph(&grid, 24, 4),
        Some(lunatui_core::DecodedGlyph::Inline('r'))
    );
    assert_eq!(
        glyph(&grid, 31, 4),
        Some(lunatui_core::DecodedGlyph::Inline('5'))
    );
    assert_eq!(
        glyph(&grid, 33, 4),
        Some(lunatui_core::DecodedGlyph::Inline('%'))
    );
    Ok(())
}

#[test]
fn late_legend_rejection_keeps_native_pie_and_the_original_cell_legend()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = LegendCapture {
        accept_strip: false,
        ..LegendCapture::default()
    };
    let grid = legend_draw(&widget(legend_spec())?, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(images.pies, [Rect::new(0, 0, 20, 10)]);
    assert_eq!(images.strips[0].0, Rect::new(22, 4, 1, 2));
    assert_eq!(
        glyph(&grid, 22, 4),
        Some(lunatui_core::DecodedGlyph::Inline('●'))
    );
    assert_eq!(
        glyph(&grid, 24, 4),
        Some(lunatui_core::DecodedGlyph::Inline('r'))
    );
    assert_eq!(
        glyph(&grid, 8, 4),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    Ok(())
}

#[test]
fn pie_rejection_never_attempts_a_native_legend_and_keeps_complete_braille()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = LegendCapture {
        accept_pie: false,
        ..LegendCapture::default()
    };
    let grid = legend_draw(&widget(legend_spec())?, &mut images, Color::Rgb(30, 40, 50));
    let expected = legend_draw(
        &legend_spec().marker(Marker::Braille),
        &mut LegendCapture::default(),
        Color::Rgb(30, 40, 50),
    );
    assert_eq!(grid, expected);
    assert_eq!(
        glyph(&grid, 22, 4),
        Some(lunatui_core::DecodedGlyph::Inline('●'))
    );
    assert_eq!(images.pies, [Rect::new(0, 0, 20, 10)]);
    assert_eq!(images.strips, Vec::new());
    Ok(())
}

#[test]
fn native_legend_cache_invalidates_when_only_the_legend_background_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let pie = widget(legend_spec())?;
    let mut images = LegendCapture::default();
    legend_draw(&pie, &mut images, Color::Rgb(30, 40, 50));
    legend_draw(&pie, &mut images, Color::Rgb(30, 40, 50));
    legend_draw(&pie, &mut images, Color::Rgb(80, 90, 100));
    assert_eq!(images.strips[0].1, images.strips[1].1);
    assert_eq!(pixel(&images.strips[2].1, 0, 16), [80, 90, 100, 255]);
    assert_eq!(pixel(&images.strips[2].1, 4, 24), [0, 255, 0, 255]);
    Ok(())
}

#[test]
fn unknown_foreground_is_not_replaced_with_the_terminal_background()
-> Result<(), Box<dyn std::error::Error>> {
    let spec = PieChart::new([
        PieSlice::new("red", 1.0).color(Color::Default),
        PieSlice::new("green", 1.0).color(Color::Rgb(0, 255, 0)),
    ])
    .cell_aspect(2.0);
    let mut images = LegendCapture::default();
    let grid = legend_draw(&widget(spec.clone())?, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(
        grid,
        legend_draw(
            &spec.marker(Marker::Braille),
            &mut LegendCapture::default(),
            Color::Rgb(30, 40, 50)
        )
    );
    assert_eq!(
        glyph(&grid, 22, 4),
        Some(lunatui_core::DecodedGlyph::Inline('●'))
    );
    assert_eq!(
        glyph(&grid, 24, 4),
        Some(lunatui_core::DecodedGlyph::Inline('r'))
    );
    assert_eq!(images.pies, Vec::new());
    assert_eq!(images.strips, Vec::new());
    Ok(())
}

#[test]
fn text_free_immediate_protocol_keeps_both_native_pie_and_native_swatches()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = LegendCapture {
        underlay: false,
        ..LegendCapture::default()
    };
    let grid = legend_draw(&widget(legend_spec())?, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(images.pies, [Rect::new(0, 0, 20, 10)]);
    assert_eq!(images.strips[0].0, Rect::new(22, 4, 1, 2));
    assert_eq!(pixel(&images.strips[0].1, 4, 8), [255, 0, 0, 255]);
    assert_eq!(
        glyph(&grid, 22, 4),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    Ok(())
}

struct OrdinaryOnly(LegendCapture);
impl ImageSink for OrdinaryOnly {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        self.0.pixel_target(area)
    }
    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        self.0.submit(request)
    }
}

#[test]
fn an_ordinary_submit_only_sink_keeps_its_native_pie_and_cell_legend()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = OrdinaryOnly(LegendCapture::default());
    let mut grid = Grid::new(40, 10);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    grid.fill(
        Rect::new(0, 0, 40, 10),
        Style::new().bg(Color::Rgb(10, 20, 30)),
        ' ',
        &mut clusters,
    );
    widget(legend_spec())?.render(
        Rect::new(0, 0, 40, 10),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut images),
    );
    assert_eq!(images.0.pies, [Rect::new(0, 0, 20, 10)]);
    assert_eq!(
        glyph(&grid, 22, 4),
        Some(lunatui_core::DecodedGlyph::Inline('●'))
    );
    assert_eq!(
        glyph(&grid, 24, 4),
        Some(lunatui_core::DecodedGlyph::Inline('r'))
    );
    assert_eq!(
        glyph(&grid, 8, 4),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    Ok(())
}

#[test]
fn a_three_cell_legend_below_the_disc_still_uses_native_swatches()
-> Result<(), Box<dyn std::error::Error>> {
    let mut images = LegendCapture::default();
    let mut grid = Grid::new(3, 8);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    grid.fill(
        Rect::new(0, 0, 3, 8),
        Style::new().bg(Color::Rgb(10, 20, 30)),
        ' ',
        &mut clusters,
    );
    widget(legend_spec())?.render(
        Rect::new(0, 0, 3, 8),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut images),
    );
    assert_eq!(images.pies, [Rect::new(0, 0, 3, 1)]);
    assert_eq!(images.strips[0].0, Rect::new(0, 2, 1, 2));
    assert_eq!(pixel(&images.strips[0].1, 4, 8), [255, 0, 0, 255]);
    assert_eq!(
        glyph(&grid, 0, 2),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    assert_eq!(
        glyph(&grid, 2, 2),
        Some(lunatui_core::DecodedGlyph::Inline(' '))
    );
    Ok(())
}

#[test]
fn zero_shares_and_focus_styles_remain_cell_text_beside_native_swatches()
-> Result<(), Box<dyn std::error::Error>> {
    let spec = PieChart::new([
        PieSlice::new("red", 0.0).color(Color::Rgb(255, 0, 0)),
        PieSlice::new("green", 0.0).color(Color::Rgb(0, 255, 0)),
    ])
    .cell_aspect(2.0)
    .highlight(Some(0))
    .state(lunatui_widgets::WidgetState::Focused);
    let mut images = LegendCapture::default();
    let grid = legend_draw(&widget(spec)?, &mut images, Color::Rgb(30, 40, 50));
    assert_eq!(images.strips[0].0, Rect::new(22, 4, 1, 2));
    assert_eq!(
        glyph(&grid, 32, 4),
        Some(lunatui_core::DecodedGlyph::Inline('0'))
    );
    assert_eq!(
        glyph(&grid, 33, 4),
        Some(lunatui_core::DecodedGlyph::Inline('%'))
    );
    let focused = grid.get(24, 4).ok_or("focused label")?.style();
    assert!(focused.attrs.bold());
    assert!(!focused.attrs.dim());
    assert_eq!(
        focused.fg,
        Theme::builtin(Variant::Dark).resolve(lunatui_theme::Token::FocusRing)
    );
    let other = grid.get(31, 5).ok_or("unfocused percentage")?.style();
    assert!(other.attrs.dim());
    assert!(!other.attrs.bold());
    assert_eq!(pixel(&images.strips[0].1, 4, 8), [255, 0, 0, 255]);
    Ok(())
}
