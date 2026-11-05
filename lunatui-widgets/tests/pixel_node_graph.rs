// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(feature = "pixel-node-graph")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{GraphNode, NodeGraph, RenderCx, Widget};

struct Capture {
    accept: bool,
    underlay: bool,
    measured: bool,
    scale: u32,
    attempts: usize,
    images: Vec<(ImageSource, Rect, i32)>,
}

impl ImageSink for Capture {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        if !self.measured {
            return None;
        }
        Some(PixelTarget::new(
            PixelSize::new(
                u32::from(area.width) * self.scale,
                u32::from(area.height) * self.scale * 2,
            )?,
            self.underlay,
            Some([10, 20, 30]),
        ))
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

fn capture() -> Capture {
    Capture {
        accept: true,
        underlay: true,
        measured: true,
        scale: 10,
        attempts: 0,
        images: Vec::new(),
    }
}
fn theme() -> Theme {
    let mut theme = Theme::builtin(Variant::Dark);
    for (token, rgb) in [
        (Token::Surface0, (10, 20, 30)),
        (Token::Surface1, (30, 40, 50)),
        (Token::TextMuted, (210, 140, 80)),
    ] {
        theme.set_token(token, Color::Rgb(rgb.0, rgb.1, rgb.2));
    }
    theme
}
fn draw(graph: &NodeGraph, sink: Option<&mut Capture>) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(7, 5);
    let mut clusters = ClusterTable::new();
    let theme = theme();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    if let Some(sink) = sink {
        cx = cx.image_sink(sink);
    }
    graph.view().render(Rect::new(0, 0, 7, 5), &mut cx);
    (grid, clusters)
}

#[test]
fn submits_antialiased_native_geometry_under_real_cell_labels()
-> Result<(), Box<dyn core::error::Error>> {
    let graph = NodeGraph::new(&[GraphNode::new("A", Size::new(5, 3)).body("one")], &[])?;
    let mut capture = capture();
    let (grid, clusters) = draw(&graph, Some(&mut capture));
    assert_eq!(capture.attempts, 1);
    let (source, area, depth) = capture.images.first().ok_or("missing raster")?;
    assert_eq!((*area, *depth), (Rect::new(0, 0, 7, 5), -1));
    assert_eq!((source.size().width(), source.size().height()), (70, 100));
    assert_eq!(&source.rgba8_pixels()[..4], &[10, 20, 30, 255]);
    assert!(
        source
            .rgba8_pixels()
            .as_chunks::<4>()
            .0
            .contains(&[30, 40, 50, 255])
    );
    assert!(
        source
            .rgba8_pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| *p != [10, 20, 30, 255]
                && *p != [30, 40, 50, 255]
                && *p != [210, 140, 80, 255]
                && p[3] == 255)
    );
    let rgba = source.rgba8_pixels();
    let pixel = |x: usize, y: usize| &rgba[(y * 70 + x) * 4..(y * 70 + x) * 4 + 4];
    assert_eq!(pixel(40, 50), &[30, 40, 50, 255]);
    assert_eq!(pixel(5, 50), &[10, 20, 30, 255]);
    assert_ne!(pixel(15, 50), &[30, 40, 50, 255]);
    assert_ne!(pixel(15, 50), &[10, 20, 30, 255]);
    assert_eq!(grid.row_display(1, &clusters), "  A    ");
    assert_eq!(grid.row_display(2, &clusters), "  one  ");
    Ok(())
}

#[test]
fn unsupported_declined_and_oversized_targets_keep_exact_cell_output()
-> Result<(), Box<dyn core::error::Error>> {
    let graph = NodeGraph::new(&[GraphNode::new("A", Size::new(5, 3)).body("one")], &[])?;
    let (expected, expected_clusters) = draw(&graph, None);
    for mode in 0..4 {
        let mut capture = capture();
        match mode {
            0 => capture.accept = false,
            1 => capture.underlay = false,
            2 => capture.measured = false,
            _ => capture.scale = 1000,
        }
        let (grid, clusters) = draw(&graph, Some(&mut capture));
        assert_eq!(capture.attempts, usize::from(mode == 0));
        assert_eq!(capture.images.len(), 0);
        for y in 0..5 {
            assert_eq!(
                grid.row_display(y, &clusters),
                expected.row_display(y, &expected_clusters)
            );
            for x in 0..7 {
                assert_eq!(
                    grid.get(x, y).map(lunatui_core::Cell::style),
                    expected.get(x, y).map(lunatui_core::Cell::style)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn custom_border_backgrounds_use_cells() -> Result<(), Box<dyn core::error::Error>> {
    let graph = NodeGraph::new(&[GraphNode::new("A", Size::new(5, 3))], &[])?;
    let mut capture = capture();
    let mut grid = Grid::new(7, 5);
    let mut clusters = ClusterTable::new();
    let theme = theme();
    graph
        .view()
        .selected(Some(0))
        .highlight_style(Style::new().fg(Color::Rgb(10, 20, 30)))
        .render(
            Rect::new(0, 0, 7, 5),
            &mut RenderCx::new(&mut grid, &mut clusters, &theme).image_sink(&mut capture),
        );
    assert_eq!(capture.attempts, 0);
    assert_eq!(grid.row_display(1, &clusters), " ╭A──╮ ");
    Ok(())
}
