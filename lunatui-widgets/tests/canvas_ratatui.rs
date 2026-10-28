// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::canvas::{Circle, Context, Line, Points, Rectangle};
use lunatui_widgets::{Canvas, Marker, RenderCx, Widget};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::Color as RatColor;
use ratatui::symbols::Marker as RatMarker;
use ratatui::widgets::Widget as RatWidget;
use ratatui::widgets::canvas::{
    Canvas as RatCanvas, Circle as RatCircle, Context as RatContext, Line as RatLine,
    Points as RatPoints, Rectangle as RatRectangle,
};

type Bounds = ([f64; 2], [f64; 2]);

fn ours(
    marker: Marker,
    bounds: Bounds,
    size: (u16, u16),
    paint: impl Fn(&mut Context<'_>),
) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(size.0, size.1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Canvas::new(paint)
        .x_bounds(bounds.0)
        .y_bounds(bounds.1)
        .marker(marker)
        .render(Rect::new(0, 0, size.0, size.1), &mut cx);
    (0..size.1)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn reference(
    marker: RatMarker,
    bounds: Bounds,
    size: (u16, u16),
    paint: impl Fn(&mut RatContext<'_>),
) -> Vec<String> {
    let canvas = RatCanvas::default()
        .x_bounds(bounds.0)
        .y_bounds(bounds.1)
        .marker(marker)
        .paint(paint);
    let area = RatRect::new(0, 0, size.0, size.1);
    let mut buffer = Buffer::empty(area);
    RatWidget::render(canvas, area, &mut buffer);
    (0..size.1)
        .map(|y| {
            (0..size.0)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

fn markers() -> [(Marker, RatMarker); 6] {
    [
        (Marker::Braille, RatMarker::Braille),
        (Marker::HalfBlock, RatMarker::HalfBlock),
        (Marker::Dot, RatMarker::Dot),
        (Marker::Block, RatMarker::Block),
        (Marker::Quadrant, RatMarker::Quadrant),
        (Marker::Sextant, RatMarker::Sextant),
    ]
}

#[test]
fn non_braille_shapes_match_the_reference_canvas() {
    let plain = Style::new();
    for size in [(30, 10), (23, 9), (61, 17)] {
        for bounds in [([0.0, 20.0], [0.0, 10.0]), ([-10.0, 10.0], [-5.0, 5.0])] {
            for (marker, rat_marker) in markers()
                .into_iter()
                .filter(|(marker, _)| *marker != Marker::Braille)
            {
                let ours_rows = ours(marker, bounds, size, |ctx| {
                    ctx.draw(&Rectangle {
                        x: -4.0,
                        y: -2.0,
                        width: 12.0,
                        height: 5.0,
                        style: plain,
                    });
                    ctx.draw(&Line {
                        x1: -12.0,
                        y1: -9.0,
                        x2: 25.0,
                        y2: 12.0,
                        style: plain,
                    });
                    ctx.draw(&Line {
                        x1: 1.0,
                        y1: -4.0,
                        x2: 1.0,
                        y2: 4.0,
                        style: plain,
                    });
                    ctx.draw(&Line {
                        x1: -9.0,
                        y1: 3.0,
                        x2: 9.0,
                        y2: 3.0,
                        style: plain,
                    });
                    ctx.draw(&Points {
                        coords: &[(0.0, 0.0), (3.0, 3.5), (50.0, 50.0)],
                        style: plain,
                    });
                });
                let rat_rows = reference(rat_marker, bounds, size, |ctx| {
                    ctx.draw(&RatRectangle {
                        x: -4.0,
                        y: -2.0,
                        width: 12.0,
                        height: 5.0,
                        color: RatColor::Reset,
                    });
                    ctx.draw(&RatLine {
                        x1: -12.0,
                        y1: -9.0,
                        x2: 25.0,
                        y2: 12.0,
                        color: RatColor::Reset,
                    });
                    ctx.draw(&RatLine {
                        x1: 1.0,
                        y1: -4.0,
                        x2: 1.0,
                        y2: 4.0,
                        color: RatColor::Reset,
                    });
                    ctx.draw(&RatLine {
                        x1: -9.0,
                        y1: 3.0,
                        x2: 9.0,
                        y2: 3.0,
                        color: RatColor::Reset,
                    });
                    ctx.draw(&RatPoints {
                        coords: &[(0.0, 0.0), (3.0, 3.5), (50.0, 50.0)],
                        color: RatColor::Reset,
                    });
                });
                assert_eq!(ours_rows, rat_rows, "{marker:?} {bounds:?} {size:?}");
            }
        }
    }
}

#[test]
fn small_circles_match_the_reference_canvas() {
    let plain = Style::new();
    for (marker, rat_marker) in markers() {
        for radius in [1.0, 2.5, 3.0] {
            let bounds = ([-5.0, 5.0], [-5.0, 5.0]);
            let size = (21, 11);
            let ours_rows = ours(marker, bounds, size, |ctx| {
                ctx.draw(&Circle {
                    x: 0.5,
                    y: -0.5,
                    radius,
                    style: plain,
                });
            });
            let rat_rows = reference(rat_marker, bounds, size, |ctx| {
                ctx.draw(&RatCircle {
                    x: 0.5,
                    y: -0.5,
                    radius,
                    color: RatColor::Reset,
                });
            });
            assert_eq!(ours_rows, rat_rows, "{marker:?} radius {radius}");
        }
    }
}

#[test]
fn layers_and_labels_match_the_reference_canvas() {
    let plain = Style::new();
    let bounds = ([0.0, 20.0], [0.0, 10.0]);
    let size = (30, 10);
    let ours_rows = ours(Marker::Braille, bounds, size, |ctx| {
        ctx.draw(&Line {
            x1: 0.0,
            y1: 0.0,
            x2: 20.0,
            y2: 10.0,
            style: plain,
        });
        ctx.layer();
        ctx.draw(&Line {
            x1: 0.0,
            y1: 10.0,
            x2: 20.0,
            y2: 0.0,
            style: plain,
        });
        ctx.print(3.0, 8.0, "hello", plain);
        ctx.print(15.0, 2.0, "world", plain);
    });
    let rat_rows = reference(RatMarker::Braille, bounds, size, |ctx| {
        ctx.draw(&RatLine {
            x1: 0.0,
            y1: 0.0,
            x2: 20.0,
            y2: 10.0,
            color: RatColor::Reset,
        });
        ctx.layer();
        ctx.draw(&RatLine {
            x1: 0.0,
            y1: 10.0,
            x2: 20.0,
            y2: 0.0,
            color: RatColor::Reset,
        });
        ctx.print(3.0, 8.0, "hello");
        ctx.print(15.0, 2.0, "world");
    });
    assert_eq!(ours_rows, rat_rows);
}
