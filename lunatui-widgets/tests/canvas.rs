// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::canvas::{Circle, Context, Line, Painter, Points, Rectangle, Shape};
use lunatui_widgets::{Canvas, Marker, RenderCx, Widget, WidgetState};

fn show(
    marker: Marker,
    bounds: ([f64; 2], [f64; 2]),
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

const UNIT: ([f64; 2], [f64; 2]) = ([0.0, 1.0], [0.0, 1.0]);

fn dot(marker: Marker, size: (u16, u16), at: (f64, f64)) -> Vec<String> {
    show(marker, UNIT, size, |ctx| {
        ctx.draw(&Points {
            coords: &[at],
            style: Style::new(),
        });
    })
}

#[test]
fn block_lines_run_cell_by_cell() {
    let line = |x1, y1, x2, y2| {
        show(Marker::Block, UNIT, (5, 3), move |ctx| {
            ctx.draw(&Line {
                x1,
                y1,
                x2,
                y2,
                style: Style::new(),
            });
        })
    };
    assert_eq!(line(0.0, 0.0, 1.0, 1.0), ["    █", "  ██ ", "██   "]);
    assert_eq!(line(0.0, 0.5, 1.0, 0.5), ["     ", "█████", "     "]);
    assert_eq!(line(0.5, 0.0, 0.5, 1.0), ["  █  ", "  █  ", "  █  "]);
}

#[test]
fn braille_line_ties_stay_centered_on_a_shallow_slope() {
    let rows = show(Marker::Braille, ([0.0, 5.0], [0.0, 14.0]), (3, 2), |ctx| {
        ctx.draw(&Line {
            x1: 0.0,
            y1: 14.0,
            x2: 4.0,
            y2: 10.0,
            style: Style::new(),
        });
    });

    assert_eq!(rows, ["⠉⠢⠄", "   "]);
}

#[test]
fn braille_lines_keep_one_dot_per_major_tick_when_reversed() {
    let render = |reverse: bool| {
        show(Marker::Braille, ([0.0, 4.0], [0.0, 2.0]), (5, 2), |ctx| {
            let (from, to) = if reverse {
                ((4.0, 2.0), (0.0, 0.0))
            } else {
                ((0.0, 0.0), (4.0, 2.0))
            };
            ctx.draw(&Line {
                x1: from.0,
                y1: from.1,
                x2: to.0,
                y2: to.1,
                style: Style::new(),
            });
        })
    };
    let dots = |rows: &[String]| {
        rows.iter()
            .flat_map(|row| row.chars())
            .filter(|glyph| *glyph >= '\u{2800}' && *glyph <= '\u{28ff}')
            .map(|glyph| (u32::from(glyph) - 0x2800).count_ones())
            .sum::<u32>()
    };

    let forward = render(false);
    let backward = render(true);
    assert_eq!(forward, backward);
    assert_eq!(dots(&forward), 10);
}

#[test]
fn each_marker_has_its_own_grain() {
    let canvas = |marker| Canvas::new(|_: &mut Context<'_>| {}).marker(marker);
    let _ = canvas(Marker::Braille);
    let grains = [
        (Marker::Braille, (8, 8)),
        (Marker::Sextant, (8, 6)),
        (Marker::Quadrant, (8, 4)),
        (Marker::HalfBlock, (4, 4)),
        (Marker::Block, (4, 2)),
        (Marker::Dot, (4, 2)),
    ];
    for (marker, want) in grains {
        let seen = std::cell::Cell::new((0, 0));
        let _ = show(marker, UNIT, (4, 2), |ctx| {
            struct Probe<'a>(&'a std::cell::Cell<(u32, u32)>);
            impl lunatui_widgets::canvas::Shape for Probe<'_> {
                fn draw(&self, painter: &mut lunatui_widgets::canvas::Painter<'_>) {
                    self.0.set(painter.resolution());
                }
            }
            ctx.draw(&Probe(&seen));
        });
        assert_eq!(seen.get(), want, "{marker:?}");
    }
}

#[test]
fn braille_points_set_one_dot_each() {
    let dots = ["⠁", "⠈", "⠂", "⠐", "⠄", "⠠", "⡀", "⢀"];
    let positions = [
        (0.0, 1.0),
        (1.0, 1.0),
        (0.0, 2.0 / 3.0),
        (1.0, 2.0 / 3.0),
        (0.0, 1.0 / 3.0),
        (1.0, 1.0 / 3.0),
        (0.0, 0.0),
        (1.0, 0.0),
    ];
    for (glyph, at) in dots.iter().zip(positions) {
        assert_eq!(dot(Marker::Braille, (1, 1), at), [*glyph], "{at:?}");
    }
}

#[test]
fn quadrant_and_half_block_points() {
    assert_eq!(dot(Marker::Quadrant, (1, 1), (0.0, 1.0)), ["▘"]);
    assert_eq!(dot(Marker::Quadrant, (1, 1), (1.0, 1.0)), ["▝"]);
    assert_eq!(dot(Marker::Quadrant, (1, 1), (0.0, 0.0)), ["▖"]);
    assert_eq!(dot(Marker::Quadrant, (1, 1), (1.0, 0.0)), ["▗"]);
    assert_eq!(dot(Marker::HalfBlock, (1, 1), (0.5, 1.0)), ["▀"]);
    assert_eq!(dot(Marker::HalfBlock, (1, 1), (0.5, 0.0)), ["▄"]);
}

#[test]
fn sextant_points_and_combinations() {
    let at = |x: f64, y: f64| dot(Marker::Sextant, (1, 1), (x, y));
    assert_eq!(at(0.0, 1.0), ["\u{1FB00}"]);
    assert_eq!(at(1.0, 1.0), ["\u{1FB01}"]);
    assert_eq!(at(0.0, 0.5), ["\u{1FB03}"]);
    assert_eq!(at(1.0, 0.5), ["\u{1FB07}"]);
    assert_eq!(at(0.0, 0.0), ["\u{1FB0F}"]);
    assert_eq!(at(1.0, 0.0), ["\u{1FB1E}"]);
    let column = |x: f64| {
        show(Marker::Sextant, UNIT, (1, 1), move |ctx| {
            ctx.draw(&Line {
                x1: x,
                y1: 0.0,
                x2: x,
                y2: 1.0,
                style: Style::new(),
            });
        })
    };
    assert_eq!(column(0.0), ["▌"]);
    assert_eq!(column(1.0), ["▐"]);
    let all = show(Marker::Sextant, UNIT, (1, 1), |ctx| {
        ctx.draw(&Rectangle {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            style: Style::new(),
        });
    });
    assert_eq!(all, ["█"]);
    let top_two = show(Marker::Sextant, UNIT, (1, 1), |ctx| {
        ctx.draw(&Line {
            x1: 0.0,
            y1: 1.0,
            x2: 1.0,
            y2: 1.0,
            style: Style::new(),
        });
    });
    assert_eq!(top_two, ["\u{1FB02}"]);
}

#[test]
fn rectangles_circles_and_points() {
    let rows = show(Marker::Block, ([0.0, 4.0], [0.0, 2.0]), (5, 3), |ctx| {
        ctx.draw(&Rectangle {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 2.0,
            style: Style::new(),
        });
    });
    assert_eq!(rows, ["█████", "█   █", "█████"]);
    let rows = show(Marker::Block, ([-2.0, 2.0], [-1.0, 1.0]), (9, 3), |ctx| {
        ctx.draw(&Circle {
            x: 0.0,
            y: 0.0,
            radius: 1.0,
            style: Style::new(),
        });
    });
    assert_eq!(rows, ["  █████  ", "  █   █  ", "  █████  "]);
}

#[test]
fn a_later_layer_replaces_the_cell_it_shares() {
    let two_dots = |layered: bool| {
        show(Marker::Braille, UNIT, (1, 1), move |ctx| {
            ctx.draw(&Points {
                coords: &[(0.0, 1.0)],
                style: Style::new(),
            });
            if layered {
                ctx.layer();
            }
            ctx.draw(&Points {
                coords: &[(1.0, 0.0)],
                style: Style::new(),
            });
        })
    };
    assert_eq!(two_dots(false), ["⢁"]);
    assert_eq!(two_dots(true), ["⢀"]);
}

#[test]
fn context_can_use_blocks_for_fills_and_braille_for_lines() {
    let rows = show(Marker::Braille, UNIT, (3, 1), |ctx| {
        ctx.marker(Marker::Block);
        ctx.draw(&Points {
            coords: &[(0.0, 0.0)],
            style: Style::new(),
        });
        ctx.marker(Marker::Braille);
        ctx.draw(&Line {
            x1: 0.5,
            y1: 0.5,
            x2: 1.0,
            y2: 1.0,
            style: Style::new(),
        });
    });

    assert_eq!(rows, ["█⠠⠊"]);
}

#[test]
fn labels_sit_at_their_cell_and_cover_the_drawing() {
    let rows = show(Marker::Block, ([0.0, 20.0], [0.0, 10.0]), (30, 10), |ctx| {
        ctx.draw(&Line {
            x1: 0.0,
            y1: 4.0,
            x2: 20.0,
            y2: 4.0,
            style: Style::new(),
        });
        ctx.print(3.0, 8.0, "hello", Style::new());
        ctx.print(15.0, 4.0, "world", Style::new());
    });
    assert_eq!(rows[1], format!("    hello{}", " ".repeat(21)));
    assert_eq!(rows[5], format!("{}world{}", "█".repeat(21), "█".repeat(4)));
}

#[test]
fn labels_are_clipped_at_the_edge_and_dropped_outside_the_bounds() {
    let rows = show(Marker::Block, UNIT, (8, 2), |ctx| {
        ctx.print(1.0, 1.0, "toolong", Style::new());
        ctx.print(0.5, 0.0, "edge", Style::new());
        ctx.print(2.0, 0.5, "out", Style::new());
        ctx.print(0.5, -1.0, "out", Style::new());
        ctx.print(f64::NAN, 0.5, "nan", Style::new());
    });
    assert_eq!(rows, ["       t", "   edge "]);
}

#[test]
fn lines_are_clipped_to_the_bounds() {
    let rows = show(Marker::Block, UNIT, (5, 3), |ctx| {
        ctx.draw(&Line {
            x1: -1.0,
            y1: 0.5,
            x2: 2.0,
            y2: 0.5,
            style: Style::new(),
        });
        ctx.draw(&Line {
            x1: 3.0,
            y1: 3.0,
            x2: 4.0,
            y2: 4.0,
            style: Style::new(),
        });
    });
    assert_eq!(rows, ["     ", "█████", "     "]);
}

#[test]
fn reversed_bounds_flip_and_empty_bounds_draw_nothing() {
    let diagonal = |bounds: Bounds| {
        show(Marker::Block, bounds, (5, 3), |ctx| {
            ctx.draw(&Line {
                x1: 0.0,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0,
                style: Style::new(),
            });
        })
    };
    assert_eq!(
        diagonal(([1.0, 0.0], [0.0, 1.0])),
        ["██   ", "  ██ ", "    █"].map(str::to_owned).to_vec()
    );
    assert_eq!(
        diagonal(([0.0, 1.0], [1.0, 0.0])),
        ["██   ", "  ██ ", "    █"].map(str::to_owned).to_vec()
    );
    for bounds in [
        ([1.0, 1.0], [0.0, 1.0]),
        ([0.0, 1.0], [2.0, 2.0]),
        ([f64::NAN, 1.0], [0.0, 1.0]),
    ] {
        assert_eq!(diagonal(bounds), ["     "; 3].map(str::to_owned).to_vec());
    }
}

type Bounds = ([f64; 2], [f64; 2]);

#[test]
fn non_finite_coordinates_are_skipped() {
    let rows = show(Marker::Block, UNIT, (3, 1), |ctx| {
        ctx.draw(&Points {
            coords: &[(f64::NAN, 0.5), (0.5, f64::INFINITY), (0.0, 0.5)],
            style: Style::new(),
        });
        ctx.draw(&Line {
            x1: f64::NAN,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
            style: Style::new(),
        });
    });
    assert_eq!(rows, ["█  "]);
}

#[test]
fn big_circles_stay_unbroken() {
    let rows = show(
        Marker::Block,
        ([-20.0, 20.0], [-20.0, 20.0]),
        (60, 40),
        |ctx| {
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 19.9,
                style: Style::new(),
            });
        },
    );
    let filled: usize = rows
        .iter()
        .map(|row| row.chars().filter(|&c| c == '█').count())
        .sum();
    assert!(filled >= 160, "{filled} cells");
    for (index, row) in rows.iter().enumerate() {
        assert!(row.contains('█'), "row {index} is empty");
    }
    for column in 0..60 {
        assert!(
            rows.iter().any(|row| row.chars().nth(column) == Some('█')),
            "column {column} is empty"
        );
    }
}

fn grid_with(canvas: &Canvas<impl Fn(&mut Context<'_>)>, size: (u16, u16)) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(size.0, size.1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    canvas.render(Rect::new(0, 0, size.0, size.1), &mut cx);
    (grid, clusters)
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .unwrap_or_default()
}

fn token(token: Token) -> Color {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    RenderCx::new(&mut grid, &mut clusters, &theme).token(token)
}

#[test]
fn unstyled_drawing_uses_the_text_color_and_explicit_styles_win() {
    let own = Style::new().fg(Color::Rgb(1, 2, 3));
    let canvas = Canvas::new(|ctx: &mut Context<'_>| {
        ctx.draw(&Points {
            coords: &[(0.0, 0.5)],
            style: Style::new(),
        });
        ctx.draw(&Points {
            coords: &[(1.0, 0.5)],
            style: own,
        });
        ctx.print(0.5, 1.0, "t", Style::new());
        ctx.print(0.5, 0.0, "u", own);
    })
    .marker(Marker::Block);
    let (grid, _) = grid_with(&canvas, (3, 3));
    assert_eq!(style_at(&grid, 0, 1).fg, token(Token::TextPrimary));
    assert_eq!(style_at(&grid, 2, 1), own);
    assert_eq!(style_at(&grid, 1, 0).fg, token(Token::TextPrimary));
    assert_eq!(style_at(&grid, 1, 2), own);
}

#[test]
fn styles_are_per_point_and_the_last_one_in_a_cell_wins() {
    let (red, blue) = (
        Style::new().fg(Color::Rgb(255, 0, 0)),
        Style::new().fg(Color::Rgb(0, 0, 255)),
    );
    let canvas = Canvas::new(move |ctx: &mut Context<'_>| {
        ctx.draw(&Points {
            coords: &[(0.0, 1.0)],
            style: red,
        });
        ctx.draw(&Points {
            coords: &[(1.0, 0.0)],
            style: blue,
        });
        ctx.draw(&Points {
            coords: &[(0.0, 0.0)],
            style: red,
        });
    });
    let (grid, clusters) = grid_with(&canvas, (2, 1));
    assert_eq!(grid.row_display(0, &clusters), "⡁⢀");
    assert_eq!(style_at(&grid, 0, 0).fg, red.fg);
    assert_eq!(style_at(&grid, 1, 0).fg, blue.fg);
    let mixed = Canvas::new(move |ctx: &mut Context<'_>| {
        ctx.draw(&Points {
            coords: &[(0.0, 1.0)],
            style: red,
        });
        ctx.draw(&Points {
            coords: &[(0.0, 0.0)],
            style: blue,
        });
    });
    let (grid, clusters) = grid_with(&mixed, (1, 1));
    assert_eq!(grid.row_display(0, &clusters), "⡁");
    assert_eq!(style_at(&grid, 0, 0).fg, blue.fg);
}

#[test]
fn disabled_dims_and_the_background_fills_every_cell() {
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let canvas = Canvas::new(|ctx: &mut Context<'_>| {
        ctx.draw(&Points {
            coords: &[(0.5, 0.5)],
            style: Style::new(),
        });
        ctx.print(0.0, 1.0, "x", Style::new());
    })
    .marker(Marker::Block)
    .background(panel)
    .state(WidgetState::Disabled);
    let (grid, _) = grid_with(&canvas, (3, 3));
    for y in 0..3 {
        for x in 0..3 {
            assert_eq!(style_at(&grid, x, y).bg, panel.bg, "bg at {x},{y}");
        }
    }
    assert!(style_at(&grid, 1, 1).attrs.dim());
    assert!(style_at(&grid, 0, 0).attrs.dim());
    let normal = Canvas::new(|ctx: &mut Context<'_>| {
        ctx.draw(&Points {
            coords: &[(0.5, 0.5)],
            style: Style::new(),
        });
    })
    .marker(Marker::Block);
    let (grid, _) = grid_with(&normal, (3, 3));
    assert!(!style_at(&grid, 1, 1).attrs.dim());
}

#[test]
fn a_second_render_leaves_nothing_of_the_first() {
    let theme = Theme::builtin(Variant::Dark);
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let big = Canvas::new(|ctx: &mut Context<'_>| {
        ctx.draw(&Rectangle {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            style: Style::new(),
        });
        ctx.print(0.0, 0.5, "long label", Style::new().bg(Color::Rgb(1, 1, 1)));
    })
    .background(panel);
    let small = Canvas::new(|_: &mut Context<'_>| {}).background(panel);
    let mut grid = Grid::new(12, 4);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    big.render(Rect::new(0, 0, 12, 4), &mut cx);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    small.render(Rect::new(0, 0, 12, 4), &mut cx);
    let mut fresh = Grid::new(12, 4);
    let mut fresh_clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut fresh, &mut fresh_clusters, &theme);
    small.render(Rect::new(0, 0, 12, 4), &mut cx);
    for y in 0..4 {
        for x in 0..12 {
            assert_eq!(grid.cell_bits(x, y), fresh.cell_bits(x, y), "cell {x},{y}");
        }
    }
}

#[test]
fn degenerate_areas_do_not_panic() {
    for marker in [
        Marker::Braille,
        Marker::Sextant,
        Marker::Quadrant,
        Marker::HalfBlock,
        Marker::Block,
        Marker::Dot,
    ] {
        for width in 0..5 {
            for height in 0..5 {
                let _ = show(marker, UNIT, (width.max(1), height.max(1)), |ctx| {
                    ctx.draw(&Circle {
                        x: 0.5,
                        y: 0.5,
                        radius: 0.5,
                        style: Style::new(),
                    });
                    ctx.draw(&Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                        style: Style::new(),
                    });
                    ctx.print(0.5, 0.5, "text", Style::new());
                });
                let theme = Theme::builtin(Variant::Dark);
                let mut grid = Grid::new(width.max(1), height.max(1));
                let mut clusters = ClusterTable::new();
                let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
                Canvas::new(|ctx: &mut Context<'_>| ctx.print(0.5, 0.5, "x", Style::new()))
                    .marker(marker)
                    .render(Rect::new(0, 0, width, height), &mut cx);
            }
        }
    }
}

#[test]
fn bounds_are_reported_to_the_paint_closure() {
    let seen = std::cell::Cell::new(([0.0; 2], [0.0; 2]));
    let _ = show(Marker::Block, ([1.0, 3.0], [-2.0, 2.0]), (2, 2), |ctx| {
        seen.set((ctx.x_bounds(), ctx.y_bounds()));
    });
    assert_eq!(seen.get(), ([1.0, 3.0], [-2.0, 2.0]));
}

#[test]
fn custom_shapes_paint_through_the_painter() {
    struct Cross;
    impl Shape for Cross {
        fn draw(&self, painter: &mut Painter<'_>) {
            painter.line((0.0, 0.5), (1.0, 0.5), Style::new());
            painter.point(0.5, 1.0, Style::new());
        }
    }
    let rows = show(Marker::Block, UNIT, (3, 3), |ctx| ctx.draw(&Cross));
    assert_eq!(rows, [" █ ", "███", "   "]);
}

#[test]
fn measure_takes_all_the_room() {
    let canvas = Canvas::new(|_: &mut Context<'_>| {});
    assert_eq!(canvas.measure(Size::new(7, 3)), Size::new(7, 3));
}

#[test]
fn sextant_masks_across_the_glyph_ranges() {
    let dots = |cells: &[(u8, u8)]| {
        let coords: Vec<(f64, f64)> = cells
            .iter()
            .map(|&(x, y)| (f64::from(x), 1.0 - f64::from(y) / 2.0))
            .collect();
        show(Marker::Sextant, UNIT, (1, 1), move |ctx| {
            ctx.draw(&Points {
                coords: &coords,
                style: Style::new(),
            });
        })
    };
    assert_eq!(dots(&[(1, 0), (0, 1), (0, 2)]), ["\u{1FB14}"]);
    assert_eq!(dots(&[(0, 0), (1, 0), (1, 1), (1, 2)]), ["\u{1FB28}"]);
    assert_eq!(
        dots(&[(1, 0), (0, 1), (1, 1), (0, 2), (1, 2)]),
        ["\u{1FB3B}"]
    );
    assert_eq!(
        dots(&[(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)]),
        ["\u{1FB1D}"]
    );
    assert_eq!(dots(&[(1, 0), (0, 1), (1, 1)]), ["\u{1FB0D}"]);
}

#[test]
fn a_point_with_one_non_finite_coordinate_is_skipped() {
    let rows = show(Marker::Block, UNIT, (3, 1), |ctx| {
        ctx.draw(&Points {
            coords: &[
                (f64::NAN, 0.5),
                (0.5, f64::NAN),
                (f64::INFINITY, 0.5),
                (1.0, 0.5),
            ],
            style: Style::new(),
        });
    });
    assert_eq!(rows, ["  █"]);
}

#[test]
fn labels_with_a_non_finite_coordinate_are_skipped() {
    let rows = show(Marker::Block, UNIT, (4, 2), |ctx| {
        ctx.print(0.5, f64::NAN, "a", Style::new());
        ctx.print(f64::NAN, 0.5, "b", Style::new());
        ctx.print(f64::INFINITY, 0.5, "c", Style::new());
    });
    assert_eq!(rows, ["    ", "    "]);
}

#[test]
fn labels_follow_offset_bounds() {
    let rows = show(Marker::Block, ([10.0, 30.0], [0.0, 1.0]), (21, 2), |ctx| {
        ctx.print(20.0, 1.0, "mid", Style::new());
        ctx.print(30.0, 0.0, "e", Style::new());
        ctx.print(10.0, 0.0, "s", Style::new());
    });
    assert_eq!(rows[0], format!("{}mid{}", " ".repeat(10), " ".repeat(8)));
    assert_eq!(rows[1], format!("s{}e", " ".repeat(19)));
}

#[test]
fn labels_stop_at_the_area_edge_even_in_a_wider_grid() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Canvas::new(|ctx: &mut Context<'_>| ctx.print(1.0, 1.0, "overflowing", Style::new()))
        .render(Rect::new(2, 0, 6, 1), &mut cx);
    assert_eq!(
        grid.row_display(0, &clusters),
        format!("{}o{}", " ".repeat(7), " ".repeat(12))
    );
}

#[test]
fn explicit_backgrounds_survive_a_canvas_background() {
    let panel = Style::new().bg(Color::Rgb(10, 20, 30));
    let own = Style::new().bg(Color::Rgb(1, 1, 1));
    let canvas = Canvas::new(move |ctx: &mut Context<'_>| {
        ctx.draw(&Points {
            coords: &[(0.0, 0.5)],
            style: own,
        });
        ctx.print(1.0, 0.5, "x", own);
    })
    .marker(Marker::Block)
    .background(panel);
    let (grid, _) = grid_with(&canvas, (3, 1));
    assert_eq!(style_at(&grid, 0, 0).bg, own.bg);
    assert_eq!(style_at(&grid, 2, 0).bg, own.bg);
    assert_eq!(style_at(&grid, 1, 0).bg, panel.bg);
}

#[test]
fn huge_circles_add_steps_beyond_the_default() {
    let rows = show(
        Marker::Block,
        ([-100.0, 100.0], [-100.0, 100.0]),
        (200, 200),
        |ctx| {
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 99.0,
                style: Style::new(),
            });
        },
    );
    let filled: usize = rows
        .iter()
        .map(|row| row.chars().filter(|&c| c == '█').count())
        .sum();
    assert!(filled >= 500, "{filled} cells");
}
