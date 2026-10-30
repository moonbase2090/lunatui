// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::Cell;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::canvas::{Circle, Context, Line, Painter, Points, Shape};
use lunatui_widgets::{Canvas, Marker, RenderCx, Widget, WidgetState};

struct Capture {
    cell: (u32, u32),
    measured: bool,
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
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        if !self.measured {
            return None;
        }
        Some(PixelTarget::new(
            PixelSize::new(
                u32::from(area.width) * self.cell.0,
                u32::from(area.height) * self.cell.1,
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
        if self.accept {
            self.images.push((source.clone(), area, depth));
        }
        self.accept
    }
}

fn capture(cell: (u32, u32)) -> Capture {
    Capture {
        cell,
        measured: true,
        underlay: true,
        background: Some([0, 0, 0]),
        accept: true,
        attempts: 0,
        images: Vec::new(),
    }
}

fn theme() -> Theme {
    let mut theme = Theme::builtin(Variant::Dark);
    theme.set_token(Token::TextPrimary, Color::Rgb(200, 120, 80));
    theme
}

fn draw<F: Fn(&mut Context<'_>)>(
    canvas: &Canvas<F>,
    images: &mut Capture,
    area: Rect,
) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(area.x + area.width, area.y + area.height);
    let mut clusters = ClusterTable::new();
    canvas.render(
        area,
        &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(images),
    );
    (grid, clusters)
}

fn pixel(source: &ImageSource, column: usize, row: usize) -> &[u8] {
    let offset = (row * usize::try_from(source.size().width()).unwrap_or(0) + column) * 4;
    &source.rgba8_pixels()[offset..offset + 4]
}

fn white() -> Style {
    Style::new().fg(Color::Rgb(255, 255, 255))
}

fn line(ctx: &mut Context<'_>, from: (f64, f64), to: (f64, f64), style: Style) {
    ctx.draw(&Line {
        x1: from.0,
        y1: from.1,
        x2: to.0,
        y2: to.1,
        style,
    });
}

struct Filled(Style);
impl Shape for Filled {
    fn draw(&self, painter: &mut Painter<'_>) {
        let (width, height) = painter.resolution();
        for row in 0..height {
            for column in 0..width {
                painter.point(
                    f64::from(column) / f64::from(width.saturating_sub(1).max(1)),
                    f64::from(row) / f64::from(height.saturating_sub(1).max(1)),
                    self.0,
                );
            }
        }
    }
}

#[test]
fn default_canvas_draws_native_aa_and_runs_callback_and_custom_shape_once() {
    struct Observed<'a> {
        calls: &'a Cell<usize>,
        resolution: &'a Cell<(u32, u32)>,
    }
    impl Shape for Observed<'_> {
        fn draw(&self, painter: &mut Painter<'_>) {
            self.calls.set(self.calls.get() + 1);
            self.resolution.set(painter.resolution());
            painter.line((0.0, 0.0), (1.0, 1.0), white());
        }
    }
    let (callbacks, shapes, resolution) = (Cell::new(0), Cell::new(0), Cell::new((0, 0)));
    let canvas = Canvas::new(|ctx| {
        callbacks.set(callbacks.get() + 1);
        ctx.draw(&Observed {
            calls: &shapes,
            resolution: &resolution,
        });
    });
    let mut images = capture((4, 4));
    let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 8, 8));
    assert_eq!(
        (callbacks.get(), shapes.get(), resolution.get()),
        (1, 1, (16, 32))
    );
    assert_eq!(images.images.len(), 1);
    let source = &images.images[0].0;
    assert_eq!(images.images[0].2, 0);
    assert_eq!(grid.row_display(0, &clusters), "        ");
    assert_eq!(pixel(source, 16, 15), [255, 255, 255, 255]);
    let edge = pixel(source, 16, 16);
    assert!(edge[0] > 0 && edge[0] < 255, "AA edge {edge:?}");
    assert_eq!(edge, [edge[0], edge[0], edge[0], 255]);
    assert_eq!(pixel(source, 0, 0), [0, 0, 0, 255]);
    let mut rejected = capture((4, 4));
    rejected.accept = false;
    draw(&canvas, &mut rejected, Rect::new(0, 0, 8, 8));
    assert_eq!(
        (callbacks.get(), shapes.get(), resolution.get()),
        (2, 2, (16, 32))
    );
    assert_eq!(rejected.attempts, 1);
}

#[test]
fn rejected_submission_uses_the_complete_cell_result_without_callback_replay() {
    let calls = Cell::new(0);
    let paint = |ctx: &mut Context<'_>| {
        calls.set(calls.get() + 1);
        line(ctx, (0.0, 0.0), (1.0, 1.0), white());
        ctx.print(0.0, 1.0, "A ", white());
    };
    let mut rejected = capture((8, 8));
    rejected.accept = false;
    let (actual, actual_clusters) = draw(
        &Canvas::new(paint).marker(Marker::Block),
        &mut rejected,
        Rect::new(0, 0, 4, 2),
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(rejected.attempts, 1);
    assert_eq!(actual.row_display(0, &actual_clusters), "A ██");
    assert_eq!(actual.row_display(1, &actual_clusters), "██  ");
    let mut explicit = capture((8, 8));
    let (expected, expected_clusters) = draw(
        &Canvas::new(paint).marker(Marker::Block).pixels(false),
        &mut explicit,
        Rect::new(0, 0, 4, 2),
    );
    assert_eq!(calls.get(), 2);
    for row in 0..2 {
        assert_eq!(
            actual.row_display(row, &actual_clusters),
            expected.row_display(row, &expected_clusters)
        );
        for column in 0..4 {
            assert_eq!(actual.get(column, row), expected.get(column, row));
        }
    }
    assert_eq!(explicit.attempts, 0);
}

#[test]
fn block_and_halfblock_points_keep_exact_fill_footprints_with_odd_cell_heights() {
    for (marker, point, first_white, first_black) in [
        (Marker::Block, (0.0, 1.0), (3, 4), (4, 0)),
        (Marker::HalfBlock, (0.0, 1.0), (3, 1), (0, 2)),
        (Marker::HalfBlock, (0.0, 0.0), (3, 4), (0, 1)),
    ] {
        let canvas = Canvas::new(|ctx| {
            ctx.draw(&Points {
                coords: &[point],
                style: white(),
            });
        })
        .marker(marker);
        let mut images = capture((4, 5));
        draw(&canvas, &mut images, Rect::new(0, 0, 2, 1));
        assert_eq!(images.images.len(), 1);
        assert_eq!(
            pixel(&images.images[0].0, first_white.0, first_white.1),
            [255, 255, 255, 255]
        );
        assert_eq!(
            pixel(&images.images[0].0, first_black.0, first_black.1),
            [0, 0, 0, 255]
        );
    }
}

#[test]
fn quadrant_and_sextant_fills_use_gapless_integer_partitions() {
    for (marker, point, white_at, black_at) in [
        (Marker::Quadrant, (1.0, 1.0), (4, 1), (2, 2)),
        (Marker::Sextant, (1.0, 0.5), (4, 2), (2, 3)),
    ] {
        let canvas = Canvas::new(|ctx| {
            ctx.draw(&Points {
                coords: &[point],
                style: white(),
            });
        })
        .marker(marker);
        let mut images = capture((5, 5));
        draw(&canvas, &mut images, Rect::new(0, 0, 1, 1));
        assert_eq!(
            pixel(&images.images[0].0, white_at.0, white_at.1),
            [255, 255, 255, 255]
        );
        assert_eq!(
            pixel(&images.images[0].0, black_at.0, black_at.1),
            [0, 0, 0, 255]
        );
        assert_eq!(pixel(&images.images[0].0, 1, 1), [0, 0, 0, 255]);
    }
}

#[test]
fn native_layers_keep_uncovered_earlier_pixels_while_cells_replace_whole_cells() {
    let paint = |ctx: &mut Context<'_>| {
        line(
            ctx,
            (0.0, 0.75),
            (1.0, 0.75),
            Style::new().fg(Color::Rgb(255, 0, 0)),
        );
        ctx.layer();
        line(
            ctx,
            (0.75, 0.0),
            (0.75, 1.0),
            Style::new().fg(Color::Rgb(0, 0, 255)),
        );
    };
    let mut images = capture((16, 16));
    draw(&Canvas::new(paint), &mut images, Rect::new(0, 0, 1, 1));
    assert_eq!(pixel(&images.images[0].0, 1, 4), [255, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 11, 4), [0, 0, 255, 255]);
    let (grid, clusters) = draw(
        &Canvas::new(paint).pixels(false),
        &mut capture((16, 16)),
        Rect::new(0, 0, 1, 1),
    );
    assert_eq!(grid.row_display(0, &clusters), "⢸");
    assert_eq!(
        grid.get(0, 0).map(|cell| cell.style().fg),
        Some(Color::Rgb(0, 0, 255))
    );
}

#[test]
fn label_spaces_and_wide_cells_mask_geometry_behind_the_entire_visible_span() {
    let canvas = Canvas::new(|ctx| {
        ctx.draw(&Filled(white()));
        ctx.print(0.0, 1.0, " A  界 ", white());
    })
    .marker(Marker::Block);
    let mut images = capture((4, 5));
    let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 8, 2));
    assert_eq!(grid.row_display(0, &clusters), " A  界  ");
    assert_eq!(images.images[0].2, -1);
    let source = &images.images[0].0;
    for column in 0..28 {
        assert_eq!(pixel(source, column, 2), [0, 0, 0, 255]);
    }
    assert_eq!(pixel(source, 28, 2), [255, 255, 255, 255]);
    assert_eq!(pixel(source, 0, 6), [255, 255, 255, 255]);
}

#[test]
fn label_controls_remain_on_one_row_and_offsets_use_floor_mapping() {
    let canvas = Canvas::new(|ctx| {
        ctx.draw(&Filled(white()));
        ctx.print(0.34, 1.0, "a\nb", white());
    })
    .marker(Marker::Block);
    let mut images = capture((4, 4));
    let (grid, clusters) = draw(&canvas, &mut images, Rect::new(2, 1, 4, 2));
    assert_eq!(grid.row_display(1, &clusters), "   ab ");
    assert_eq!(images.images[0].1, Rect::new(2, 1, 4, 2));
    assert_eq!(pixel(&images.images[0].0, 3, 1), [255, 255, 255, 255]);
    assert_eq!(pixel(&images.images[0].0, 4, 1), [0, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 11, 1), [0, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 12, 1), [255, 255, 255, 255]);
}

#[test]
fn overlapping_labels_mask_using_final_backgrounds_including_erased_wide_halves() {
    let canvas = Canvas::new(|ctx| {
        ctx.draw(&Filled(white()));
        ctx.print(0.0, 1.0, "界 ", white().bg(Color::Rgb(10, 20, 30)));
        ctx.print(1.0 / 3.0, 1.0, "b", white().bg(Color::Rgb(40, 50, 60)));
    })
    .marker(Marker::Block);
    let mut images = capture((4, 4));
    let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 4, 2));
    assert_eq!(grid.row_display(0, &clusters), " b  ");
    assert_eq!(pixel(&images.images[0].0, 0, 1), [40, 50, 60, 255]);
    assert_eq!(pixel(&images.images[0].0, 4, 1), [40, 50, 60, 255]);
    assert_eq!(pixel(&images.images[0].0, 8, 1), [10, 20, 30, 255]);
    assert_eq!(pixel(&images.images[0].0, 12, 1), [255, 255, 255, 255]);
}

#[test]
fn immediate_protocols_fall_back_for_visible_labels_including_whitespace() {
    for text in [" ", "界", "a\nb"] {
        let calls = Cell::new(0);
        let canvas = Canvas::new(|ctx| {
            calls.set(calls.get() + 1);
            line(ctx, (0.0, 0.0), (1.0, 1.0), white());
            ctx.print(0.0, 1.0, text, white());
        })
        .marker(Marker::Block);
        let mut images = capture((4, 4));
        images.underlay = false;
        let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 4, 2));
        assert_eq!(calls.get(), 1);
        assert_eq!(images.attempts, 0);
        assert_eq!(grid.row_display(1, &clusters), "██  ");
    }
}

#[test]
fn invisible_clipped_or_zero_width_labels_do_not_block_immediate_images() {
    for (position, text) in [
        ((1.0, 1.0), "界"),
        ((2.0, 1.0), "label"),
        ((0.0, 1.0), "\n"),
    ] {
        let canvas = Canvas::new(|ctx| {
            line(ctx, (0.0, 0.0), (1.0, 1.0), white());
            ctx.print(position.0, position.1, text, white());
        });
        let mut images = capture((4, 4));
        images.underlay = false;
        draw(&canvas, &mut images, Rect::new(0, 0, 4, 2));
        assert_eq!(images.images.len(), 1, "label {text:?}");
        assert_eq!(images.images[0].2, 0);
    }
}

#[test]
fn raw_primitive_styles_precede_canvas_background_inheritance_and_dim_is_literal() {
    let canvas = Canvas::new(|ctx| {
        ctx.draw(&Points {
            coords: &[(0.0, 1.0)],
            style: Style::new(),
        });
    })
    .marker(Marker::Block)
    .background(Style::new().bg(Color::Rgb(11, 22, 33)))
    .state(WidgetState::Disabled);
    let mut images = capture((4, 4));
    draw(&canvas, &mut images, Rect::new(0, 0, 2, 1));
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [100, 60, 40, 255]);
    assert_eq!(pixel(&images.images[0].0, 4, 0), [11, 22, 33, 255]);
}

#[test]
fn invisible_unsupported_styles_do_not_reject_visible_native_primitives() {
    let canvas = Canvas::new(|ctx| {
        ctx.draw(&Points {
            coords: &[(2.0, 2.0)],
            style: white().with_attrs(Attrs::new().set_bold(true)),
        });
        ctx.draw(&Points {
            coords: &[(0.0, 1.0)],
            style: white(),
        });
    })
    .marker(Marker::Block);
    let mut images = capture((4, 4));
    draw(&canvas, &mut images, Rect::new(0, 0, 2, 1));
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [255, 255, 255, 255]);
}

#[test]
fn unsupported_primitives_reverse_backgrounds_and_missing_metrics_use_exact_cells_once() {
    for failure in 0..6 {
        let calls = Cell::new(0);
        let primitive = match failure {
            0 => white().bg(Color::Rgb(1, 2, 3)),
            1 => white().with_attrs(Attrs::new().set_bold(true)),
            _ => white(),
        };
        let background = if failure == 2 {
            Style::new().with_attrs(Attrs::new().set_reverse(true))
        } else {
            Style::new()
        };
        let paint = |ctx: &mut Context<'_>| {
            calls.set(calls.get() + 1);
            line(ctx, (0.0, 0.0), (1.0, 1.0), primitive);
            if failure == 3 {
                ctx.print(
                    0.0,
                    1.0,
                    "a",
                    white().with_attrs(Attrs::new().set_reverse(true)),
                );
            }
        };
        let mut images = capture((4, 4));
        if failure == 4 {
            images.measured = false;
        }
        if failure == 5 {
            images.background = None;
        }
        let (actual, actual_clusters) = draw(
            &Canvas::new(paint)
                .marker(Marker::Block)
                .background(background),
            &mut images,
            Rect::new(0, 0, 4, 2),
        );
        assert_eq!(calls.get(), 1, "failure {failure}");
        assert_eq!(images.attempts, 0, "failure {failure}");
        let (expected, expected_clusters) = draw(
            &Canvas::new(paint)
                .marker(Marker::Block)
                .background(background)
                .pixels(false),
            &mut capture((4, 4)),
            Rect::new(0, 0, 4, 2),
        );
        for row in 0..2 {
            assert_eq!(
                actual.row_display(row, &actual_clusters),
                expected.row_display(row, &expected_clusters)
            );
            for column in 0..4 {
                assert_eq!(actual.get(column, row), expected.get(column, row));
            }
        }
    }
}

#[test]
fn circles_are_continuous_native_curves_and_negative_radius_preserves_geometry() {
    let scene = |radius| {
        Canvas::new(move |ctx| {
            ctx.draw(&Circle {
                x: 0.5,
                y: 0.5,
                radius,
                style: white(),
            });
        })
    };
    let mut positive = capture((128, 128));
    draw(&scene(0.25), &mut positive, Rect::new(0, 0, 8, 8));
    let source = &positive.images[0].0;
    assert_eq!(pixel(source, 512, 512), [0, 0, 0, 255]);
    for row in 270..754 {
        assert!(
            (512..800).any(|column| pixel(source, column, row)[0] != 0),
            "curve gap at row {row}"
        );
    }
    let mut negative = capture((128, 128));
    draw(&scene(-0.25), &mut negative, Rect::new(0, 0, 8, 8));
    assert_eq!(source.rgba8_pixels(), negative.images[0].0.rgba8_pixels());
}

#[test]
fn zero_radius_circles_keep_the_marker_and_extreme_circles_fall_back_once() {
    let mut images = capture((4, 4));
    draw(
        &Canvas::new(|ctx| {
            ctx.draw(&Circle {
                x: 0.0,
                y: 1.0,
                radius: 0.0,
                style: white(),
            });
        })
        .marker(Marker::Block),
        &mut images,
        Rect::new(0, 0, 2, 1),
    );
    assert_eq!(pixel(&images.images[0].0, 0, 0), [255, 255, 255, 255]);
    for radius in [f64::NAN, f64::INFINITY, 1e300] {
        let calls = Cell::new(0);
        let canvas = Canvas::new(|ctx| {
            calls.set(calls.get() + 1);
            ctx.draw(&Circle {
                x: 0.5,
                y: 0.5,
                radius,
                style: white(),
            });
        });
        let mut rejected = capture((4, 4));
        let (grid, clusters) = draw(&canvas, &mut rejected, Rect::new(0, 0, 4, 2));
        assert_eq!(calls.get(), 1);
        assert_eq!(rejected.attempts, 0);
        assert_eq!(grid.row_display(0, &clusters), "    ");
        assert_eq!(grid.row_display(1, &clusters), "    ");
    }
}

#[test]
fn clipped_and_reversed_circles_accept_finite_geometry() {
    let scene = |reversed| {
        Canvas::new(|ctx| {
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.5,
                radius: 0.3,
                style: white(),
            });
        })
        .x_bounds(if reversed { [1.0, 0.0] } else { [0.0, 1.0] })
    };
    let mut normal = capture((8, 8));
    draw(&scene(false), &mut normal, Rect::new(0, 0, 8, 8));
    let mut reversed = capture((8, 8));
    draw(&scene(true), &mut reversed, Rect::new(0, 0, 8, 8));
    assert_eq!(normal.images.len(), 1);
    assert_eq!(reversed.images.len(), 1);
    let source = &normal.images[0].0;
    for row in 0..64 {
        for column in 0..64 {
            if pixel(source, column, row)[0] != 0 {
                let reflected = 63 - column;
                assert!(
                    (reflected.saturating_sub(1)..=(reflected + 1).min(63)).any(|neighbor| pixel(
                        &reversed.images[0].0,
                        neighbor,
                        row
                    )[0]
                        != 0),
                    "missing reflected curve at {column},{row}"
                );
            }
        }
    }
    assert!(pixel(source, 19, 32)[0] > 0);
    assert!(pixel(&reversed.images[0].0, 44, 32)[0] > 0);
    assert_eq!(pixel(source, 40, 32), [0, 0, 0, 255]);
    assert_eq!(pixel(&reversed.images[0].0, 20, 32), [0, 0, 0, 255]);
}

#[test]
fn many_short_coastline_like_segments_use_local_work_and_accept_native() {
    let calls = Cell::new(0);
    let canvas = Canvas::new(|ctx| {
        calls.set(calls.get() + 1);
        for _ in 0..5000 {
            line(ctx, (0.2, 0.4), (0.2001, 0.4), white());
        }
    });
    let mut images = capture((64, 32));
    draw(&canvas, &mut images, Rect::new(0, 0, 16, 16));
    assert_eq!(calls.get(), 1);
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 205, 306), [255, 255, 255, 255]);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [0, 0, 0, 255]);
}

#[test]
fn operation_limit_accepts_the_boundary_then_falls_back_without_replaying() {
    for count in [65_536, 65_537] {
        let calls = Cell::new(0);
        let canvas = Canvas::new(|ctx| {
            calls.set(calls.get() + 1);
            for _ in 0..count {
                ctx.draw(&Points {
                    coords: &[(0.0, 1.0)],
                    style: white(),
                });
            }
        })
        .marker(Marker::Block);
        let mut images = capture((1, 1));
        let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 1, 1));
        assert_eq!(calls.get(), 1);
        if count == 65_536 {
            assert_eq!(images.images.len(), 1);
            assert_eq!(pixel(&images.images[0].0, 0, 0), [255, 255, 255, 255]);
        } else {
            assert_eq!(images.attempts, 0);
            assert_eq!(grid.row_display(0, &clusters), "█");
        }
    }
}

#[test]
fn expensive_native_work_and_invalid_target_sizes_fall_back_once() {
    for cell in [(64, 64), (65, 65), (4097, 1)] {
        let calls = Cell::new(0);
        let canvas = Canvas::new(|ctx| {
            calls.set(calls.get() + 1);
            for _ in 0..1000 {
                line(ctx, (0.0, 0.0), (1.0, 1.0), white());
            }
        })
        .marker(Marker::Block);
        let mut images = capture(cell);
        let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 16, 16));
        assert_eq!(calls.get(), 1);
        assert_eq!(images.attempts, 0, "cell {cell:?}");
        assert_eq!(grid.row_display(0, &clusters), "               █");
    }
}

#[test]
fn coordinate_edges_and_empty_areas_do_not_overflow_or_invoke_the_callback() {
    let calls = Cell::new(0);
    let canvas = Canvas::new(|_| calls.set(calls.get() + 1));
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let mut images = capture((1, 1));
    for area in [
        Rect::new(u16::MAX, u16::MAX, 1, 1),
        Rect::new(0, 0, 0, 1),
        Rect::new(0, 0, 1, 0),
    ] {
        canvas.render(
            area,
            &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(&mut images),
        );
    }
    assert_eq!(calls.get(), 0);
    assert_eq!(images.attempts, 0);
}

#[test]
fn exact_right_edge_wide_labels_and_whitespace_only_labels_mask_all_cells() {
    for (text, expected, first_mask) in [("界", "  界", 8), ("  ", "    ", 8)] {
        let canvas = Canvas::new(|ctx| {
            ctx.draw(&Filled(white()));
            ctx.print(2.0 / 3.0, 1.0, text, white());
        })
        .marker(Marker::Block);
        let mut images = capture((4, 4));
        let (grid, clusters) = draw(&canvas, &mut images, Rect::new(0, 0, 4, 2));
        assert_eq!(grid.row_display(0, &clusters), expected);
        assert_eq!(
            pixel(&images.images[0].0, first_mask - 1, 1),
            [255, 255, 255, 255]
        );
        for column in first_mask..16 {
            assert_eq!(pixel(&images.images[0].0, column, 1), [0, 0, 0, 255]);
        }
    }
}

#[test]
fn degenerate_native_lines_keep_visible_points_and_subpixel_fill_limits_fall_back() {
    let canvas = Canvas::new(|ctx| line(ctx, (0.5, 0.5), (0.5, 0.5), white()));
    let mut images = capture((1, 1));
    draw(&canvas, &mut images, Rect::new(0, 0, 1, 1));
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 0, 0), [255, 255, 255, 255]);
    let mut too_small = capture((1, 1));
    let (grid, clusters) = draw(
        &Canvas::new(|ctx| {
            ctx.draw(&Points {
                coords: &[(0.0, 1.0)],
                style: white(),
            });
        })
        .marker(Marker::HalfBlock),
        &mut too_small,
        Rect::new(0, 0, 1, 1),
    );
    assert_eq!(too_small.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), "▀");
}
