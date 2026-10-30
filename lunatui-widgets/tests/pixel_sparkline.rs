// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    BarSet, Baseline, Block, NINE_LEVELS, RenderCx, RenderDirection, Sparkline, SparklineBar,
    SparklineValue, THREE_LEVELS, Widget, WidgetState,
};

struct Capture {
    size: Option<PixelSize>,
    underlay: bool,
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
        supported: true,
        accept: true,
        queries: Cell::new(0),
        attempts: 0,
        images: Vec::new(),
    }
}

fn theme() -> Theme {
    let mut theme = Theme::builtin(Variant::Dark);
    theme.set_token(Token::Accent, Color::Rgb(240, 120, 60));
    theme.set_token(Token::FocusRing, Color::Rgb(100, 200, 80));
    theme.set_token(Token::TextPrimary, Color::Rgb(200, 180, 160));
    theme.set_token(Token::Surface0, Color::Rgb(11, 22, 33));
    theme
}

fn draw(
    chart: &Sparkline<'_, impl SparklineValue>,
    images: &mut Capture,
    width: u16,
    height: u16,
) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    chart.render(
        Rect::new(0, 0, width, height),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(images),
    );
    (grid, clusters)
}

fn pixel(source: &ImageSource, x: usize, y: usize) -> &[u8] {
    let offset = (y * usize::try_from(source.size().width()).unwrap_or(0) + x) * 4;
    &source.rgba8_pixels()[offset..offset + 4]
}

#[test]
fn default_calls_render_raw_antialiased_heights_beyond_eighth_cells() {
    let data = [1_u64];
    let chart = Sparkline::new(&data).max(3);
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(&chart, &mut images, 1, 1);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(0, 0, 1, 1));
    assert_eq!(images.images[0].2, 0);
    assert_eq!(grid.row_display(0, &clusters), " ");
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 6, 15), [0, 0, 0, 255]);
    let edge = pixel(source, 6, 16);
    assert!(edge[0] > 0 && edge[0] < 240, "AA edge: {edge:?}");
    assert_eq!(pixel(source, 6, 17), [240, 120, 60, 255]);
    let (grid, clusters) = draw(&chart.pixels(false), &mut capture(13, 25), 1, 1);
    assert_eq!(grid.row_display(0, &clusters), "▂");
}

#[test]
fn equal_cell_glyphs_do_not_collapse_distinct_raw_native_heights() {
    let mut images = capture(20, 100);
    let data = [25_u64, 30];
    let chart = Sparkline::new(&data)
        .max(100)
        .style(Style::new().fg(Color::Rgb(255, 0, 0)));
    let (grid, clusters) = draw(&chart.clone().pixels(false), &mut images, 2, 1);
    assert_eq!(grid.row_display(0, &clusters), "▂▂");
    draw(&chart, &mut images, 2, 1);
    assert_eq!(pixel(&images.images[0].0, 5, 72), [0, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 15, 72), [255, 0, 0, 255]);
}

#[test]
fn minimum_baseline_direction_and_invisible_tail_keep_the_existing_scale() {
    let data = [2_u64, 4, 6];
    let chart = Sparkline::new(&data)
        .baseline(Baseline::Minimum)
        .direction(RenderDirection::RightToLeft)
        .style(Style::new().fg(Color::Rgb(255, 0, 0)));
    let mut images = capture(20, 40);
    let (grid, clusters) = draw(&chart, &mut images, 2, 2);
    assert_eq!(images.images.len(), 1);
    assert_eq!(grid.row_display(1, &clusters), "  ");
    assert_eq!(pixel(&images.images[0].0, 5, 15), [0, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 5, 25), [255, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 15, 35), [0, 0, 0, 255]);
}

#[test]
fn row_colors_override_bar_colors_repeat_and_keep_final_backgrounds() {
    let data = [
        SparklineBar::value(8).style(Style::new().fg(Color::Rgb(255, 0, 0))),
        SparklineBar::value(4),
    ];
    let top = Style::new().fg(Color::Rgb(255, 200, 0));
    let lower = Style::new().fg(Color::Rgb(0, 0, 255));
    let chart = Sparkline::new(&data)
        .max(8)
        .style(
            Style::new()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(1, 2, 3)),
        )
        .row_styles(&[top, lower]);
    let mut images = capture(20, 30);
    let (grid, _) = draw(&chart, &mut images, 2, 3);
    assert_eq!(images.images.len(), 1);
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 5, 5), [255, 200, 0, 255]);
    assert_eq!(pixel(source, 5, 15), [0, 0, 255, 255]);
    assert_eq!(pixel(source, 5, 25), [0, 0, 255, 255]);
    assert_eq!(pixel(source, 15, 12), [1, 2, 3, 255]);
    assert_eq!(pixel(source, 15, 17), [0, 0, 255, 255]);
    assert_eq!(
        grid.get(0, 2).map(|c| c.style().bg),
        Some(Color::Rgb(1, 2, 3))
    );
}

#[test]
fn focused_rgb_bold_and_disabled_dim_remain_native() {
    let data = [1_u64, 1];
    for (state, first, last, bold) in [
        (
            WidgetState::Normal,
            [200, 180, 160, 255],
            [240, 120, 60, 255],
            false,
        ),
        (
            WidgetState::Focused,
            [240, 120, 60, 255],
            [100, 200, 80, 255],
            true,
        ),
        (
            WidgetState::Disabled,
            [100, 90, 80, 255],
            [100, 90, 80, 255],
            false,
        ),
    ] {
        let mut images = capture(20, 20);
        let (grid, clusters) = draw(&Sparkline::new(&data).state(state), &mut images, 2, 1);
        assert_eq!(images.images.len(), 1);
        assert_eq!(grid.row_display(0, &clusters), "  ");
        assert_eq!(pixel(&images.images[0].0, 5, 5), first);
        assert_eq!(pixel(&images.images[0].0, 15, 5), last);
        assert_eq!(grid.get(1, 0).map(|c| c.style().attrs.bold()), Some(bold));
    }
}

#[test]
fn missing_block_marks_keep_absent_style_and_native_fraction_in_every_row() {
    let data = [
        SparklineBar::value(8),
        SparklineBar::absent().style(Style::new().fg(Color::Rgb(255, 0, 0))),
    ];
    let chart = Sparkline::new(&data)
        .style(Style::new().fg(Color::Rgb(255, 255, 255)))
        .absent_value_symbol("▃ ignored")
        .absent_value_style(
            Style::new()
                .fg(Color::Rgb(0, 0, 255))
                .bg(Color::Rgb(1, 2, 3)),
        );
    let mut images = capture(20, 50);
    images.underlay = false;
    let (grid, clusters) = draw(&chart, &mut images, 2, 2);
    assert_eq!(images.images.len(), 1);
    assert_eq!(grid.row_display(0, &clusters), "  ");
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 5, 5), [255, 255, 255, 255]);
    assert_eq!(pixel(source, 15, 5), [1, 2, 3, 255]);
    assert_eq!(pixel(source, 15, 20), [0, 0, 255, 255]);
    assert_eq!(pixel(source, 15, 30), [1, 2, 3, 255]);
    assert_eq!(pixel(source, 15, 45), [0, 0, 255, 255]);
}

#[test]
fn missing_spaces_and_unused_custom_absence_keep_native_support() {
    for missing in [false, true] {
        let data = [Some(8_u64), if missing { None } else { Some(4) }];
        let mut images = capture(20, 20);
        draw(
            &Sparkline::new(&data)
                .absent_value_symbol(if missing { " " } else { "?" })
                .absent_value_style(Style::new().bg(Color::Rgb(1, 2, 3))),
            &mut images,
            2,
            1,
        );
        assert_eq!(images.images.len(), 1);
        if missing {
            assert_eq!(pixel(&images.images[0].0, 15, 15), [1, 2, 3, 255]);
        }
    }
}

#[test]
fn coarse_and_custom_block_sets_preserve_their_quantization() {
    let data = [1_u64, 2, 6, 7, 8];
    let chart = Sparkline::new(&data)
        .style(Style::new().fg(Color::Rgb(255, 0, 0)))
        .max(8)
        .bar_set(THREE_LEVELS);
    let mut images = capture(50, 20);
    draw(&chart, &mut images, 5, 1);
    assert_eq!(images.images.len(), 1);
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 5, 15), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 15, 5), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 15, 15), [255, 0, 0, 255]);
    assert_eq!(pixel(source, 25, 5), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 35, 5), [255, 0, 0, 255]);
    let data = [1_u64];
    let custom = BarSet {
        full: "█ ignored",
        ..NINE_LEVELS
    };
    let mut images = capture(10, 24);
    draw(
        &Sparkline::new(&data).max(3).bar_set(custom),
        &mut images,
        1,
        1,
    );
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 5, 16), [0, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 5, 19), [240, 120, 60, 255]);
}

#[test]
fn maximum_zero_clamping_and_extreme_values_keep_u64_precision() {
    let data = [u64::MAX];
    for max in [1, u64::MAX] {
        let mut images = capture(13, 25);
        draw(&Sparkline::new(&data).max(max), &mut images, 1, 1);
        assert_eq!(images.images.len(), 1);
        assert_eq!(pixel(&images.images[0].0, 6, 0), [240, 120, 60, 255]);
    }
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(&Sparkline::new(&data).max(0), &mut images, 1, 1);
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), " ");
}

fn seeded_draw(
    chart: &Sparkline<'_, impl SparklineValue>,
    images: &mut Capture,
    area: Rect,
) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(14, 7);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 14, 7),
        Style::new().fg(Color::Rgb(7, 8, 9)),
        '.',
        &mut clusters,
    );
    grid.put_str(1, 1, Style::new(), "界", &mut clusters);
    grid.put_str(4, 2, Style::new(), "界", &mut clusters);
    grid.put_str(0, 6, Style::new(), "e\u{301}", &mut clusters);
    grid.clear_dirty();
    grid.set_style(Rect::new(12, 0, 1, 1), Style::new().fg(Color::Rgb(1, 2, 3)));
    chart.render(
        area,
        &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(images),
    );
    (grid, clusters)
}

#[test]
fn every_native_decline_keeps_the_exact_original_cells_and_dirty_spans() {
    for mode in 0..12 {
        let data = [Some(3_u64), Some(1), None];
        let mut images = capture(30, 50);
        let mut style = Style::new().fg(Color::Rgb(200, 100, 50));
        let mut absent = " ";
        let mut area = Rect::new(2, 1, 9, 5);
        let mut bars = NINE_LEVELS;
        match mode {
            0 => images.accept = false,
            1 => images.size = None,
            2 => images.background = None,
            3 => images.size = PixelSize::new(4097, 50),
            4 => images.size = PixelSize::new(2048, 1024),
            5 => images.size = PixelSize::new(2, 50),
            6 => style = style.with_attrs(Attrs::new().set_reverse(true)),
            7 => style = style.with_attrs(Attrs::new().set_italic(true)),
            8 => {
                style = Style::new()
                    .fg(Color::Indexed(1))
                    .with_attrs(Attrs::new().set_bold(true));
            }
            9 => absent = "界",
            10 => {
                bars = BarSet {
                    full: "*",
                    ..NINE_LEVELS
                }
            }
            _ => area = Rect::new(2, 5, 9, 5),
        }
        let chart = Sparkline::new(&data)
            .max(3)
            .style(style)
            .absent_value_symbol(absent)
            .bar_set(bars);
        let (actual, clusters) = seeded_draw(&chart, &mut images, area);
        let (expected, expected_clusters) =
            seeded_draw(&chart.pixels(false), &mut capture(30, 50), area);
        assert_eq!(images.images.len(), 0, "mode {mode}");
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
                "dirty mode {mode}, row {y}"
            );
            assert_eq!(
                actual.row_display(y, &clusters),
                expected.row_display(y, &expected_clusters)
            );
        }
    }
}

#[test]
fn accepted_pixels_keep_wide_repair_damage_and_unused_columns_exact() {
    let data = [3_u64, 1, 2];
    for direction in [RenderDirection::LeftToRight, RenderDirection::RightToLeft] {
        let chart = Sparkline::new(&data).direction(direction);
        let mut images = capture(30, 50);
        let area = Rect::new(2, 1, 9, 5);
        let (actual, clusters) = seeded_draw(&chart, &mut images, area);
        let (expected, expected_clusters) =
            seeded_draw(&chart.pixels(false), &mut capture(30, 50), area);
        assert_eq!(images.images.len(), 1);
        let x = if direction == RenderDirection::LeftToRight {
            2
        } else {
            8
        };
        assert_eq!(images.images[0].1, Rect::new(x, 1, 3, 5));
        for y in 0..7 {
            assert_eq!(actual.row_dirty(y), expected.row_dirty(y));
            for column in 0..14 {
                let actual_cell = actual.get(column, y);
                let expected_cell = expected.get(column, y);
                if (1..6).contains(&y) && (x..x + 3).contains(&column) {
                    assert_eq!(
                        actual_cell.map(|cell| cell.glyph().decode()),
                        Some(lunatui_core::DecodedGlyph::Inline(' '))
                    );
                    assert_eq!(
                        actual_cell.map(lunatui_core::Cell::style),
                        expected_cell.map(lunatui_core::Cell::style)
                    );
                    assert_eq!(
                        actual_cell.map(lunatui_core::Cell::meta),
                        expected_cell.map(lunatui_core::Cell::meta)
                    );
                } else {
                    assert_eq!(actual_cell, expected_cell, "{direction:?} at {column},{y}");
                }
            }
        }
        assert_eq!(
            actual.row_display(6, &clusters),
            expected.row_display(6, &expected_clusters)
        );
    }
}

#[test]
fn block_borders_stay_in_cells_and_native_bounds_exclude_unused_inner_columns() {
    let data = [8_u64, 4];
    let chart = Sparkline::new(&data).block(Block::bordered().title("load"));
    let mut images = capture(20, 40);
    let (grid, clusters) = draw(&chart, &mut images, 8, 4);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(1, 1, 2, 2));
    assert_eq!(grid.row_display(0, &clusters), "┌load──┐");
    assert_eq!(grid.row_display(3, &clusters), "└──────┘");
}

struct Probe {
    index: usize,
    calls: Cell<usize>,
    events: Rc<RefCell<Vec<usize>>>,
    shared: Rc<Cell<u64>>,
    panic_on_second: bool,
}

impl SparklineValue for Probe {
    fn sparkline_bar(&self) -> SparklineBar {
        self.events.borrow_mut().push(self.index);
        let calls = self.calls.replace(self.calls.get() + 1);
        assert!(!(self.panic_on_second && calls == 1), "sample panic");
        SparklineBar::value(if calls == 0 { 3 } else { 1 + self.shared.get() })
    }
}

struct SharedSink {
    shared: Rc<Cell<u64>>,
    capture: Capture,
}

impl ImageSink for SharedSink {
    fn supports_pixels(&self) -> bool {
        self.shared.set(100);
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        self.shared.set(200);
        self.capture.pixel_target(area)
    }
    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }
    fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
        self.capture.submit_raster(source, area, depth)
    }
}

#[test]
fn callbacks_keep_both_passes_before_any_stateful_sink_query_even_on_late_decline() {
    for (enabled, accepted) in [(false, false), (true, true), (true, false)] {
        let shared = Rc::new(Cell::new(0));
        let events = Rc::new(RefCell::new(Vec::new()));
        let data: Vec<Probe> = (0..3)
            .map(|index| Probe {
                index,
                calls: Cell::new(0),
                events: events.clone(),
                shared: shared.clone(),
                panic_on_second: false,
            })
            .collect();
        let mut sink = SharedSink {
            shared,
            capture: capture(20, 25),
        };
        sink.capture.accept = accepted;
        let mut grid = Grid::new(2, 1);
        let mut clusters = ClusterTable::new();
        Sparkline::new(&data).max(3).pixels(enabled).render(
            Rect::new(0, 0, 2, 1),
            &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(&mut sink),
        );
        assert_eq!(*events.borrow(), [0, 1, 2, 0, 1]);
        if accepted {
            assert_eq!(sink.capture.images.len(), 1);
            assert_eq!(pixel(&sink.capture.images[0].0, 15, 15), [0, 0, 0, 255]);
            assert_eq!(
                pixel(&sink.capture.images[0].0, 15, 20),
                [240, 120, 60, 255]
            );
            assert_eq!(grid.row_display(0, &clusters), "  ");
        } else {
            assert_eq!(grid.row_display(0, &clusters), "▂▂");
        }
    }
}

#[test]
fn callback_panic_keeps_original_partial_grid_and_never_queries_the_sink() {
    for enabled in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let shared = Rc::new(Cell::new(0));
        let data: Vec<Probe> = (0..2)
            .map(|index| Probe {
                index,
                calls: Cell::new(0),
                events: events.clone(),
                shared: shared.clone(),
                panic_on_second: index == 1,
            })
            .collect();
        let mut grid = Grid::new(3, 1);
        let mut clusters = ClusterTable::new();
        grid.fill(Rect::new(0, 0, 3, 1), Style::new(), '.', &mut clusters);
        let mut images = capture(20, 25);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Sparkline::new(&data).pixels(enabled).render(
                Rect::new(0, 0, 2, 1),
                &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(&mut images),
            );
        }));
        assert!(result.is_err());
        assert_eq!(grid.row_display(0, &clusters), "▂..");
        assert_eq!(*events.borrow(), [0, 1, 0, 1]);
        assert_eq!(images.queries.get(), 0);
        assert_eq!(images.attempts, 0);
    }
}

#[test]
fn off_and_incapable_paths_do_not_query_metrics_and_zero_area_does_not_read_values() {
    let data = [1_u64];
    for enabled in [false, true] {
        let mut images = capture(13, 25);
        images.supported = !enabled;
        let (grid, clusters) = draw(&Sparkline::new(&data).pixels(enabled), &mut images, 1, 1);
        assert_eq!(grid.row_display(0, &clusters), "█");
        assert_eq!(images.queries.get(), 0);
        assert_eq!(images.attempts, 0);
    }
    let events = Rc::new(RefCell::new(Vec::new()));
    let data = [Probe {
        index: 0,
        calls: Cell::new(0),
        events: events.clone(),
        shared: Rc::new(Cell::new(0)),
        panic_on_second: false,
    }];
    let mut images = capture(13, 25);
    draw(
        &Sparkline::new(&data).block(Block::bordered()),
        &mut images,
        1,
        1,
    );
    assert_eq!(*events.borrow(), Vec::<usize>::new());
    assert_eq!(images.queries.get(), 0);
}

#[test]
fn oversized_logical_work_and_out_of_grid_columns_use_cells_without_metric_queries() {
    let data = vec![1_u64; 257];
    let mut images = capture(1028, 1024);
    let (grid, clusters) = draw(&Sparkline::new(&data), &mut images, 257, 256);
    assert_eq!(images.queries.get(), 0);
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(255, &clusters), "█".repeat(257));
    let events = Rc::new(RefCell::new(Vec::new()));
    let data: Vec<Probe> = (0..3)
        .map(|index| Probe {
            index,
            calls: Cell::new(0),
            events: events.clone(),
            shared: Rc::new(Cell::new(0)),
            panic_on_second: false,
        })
        .collect();
    let mut images = capture(20, 20);
    let (grid, clusters) = seeded_draw(
        &Sparkline::new(&data),
        &mut images,
        Rect::new(u16::MAX - 1, 1, 2, 2),
    );
    assert_eq!(*events.borrow(), [0, 1, 2, 0, 1]);
    assert_eq!(images.queries.get(), 0);
    assert_eq!(grid.row_display(0, &clusters), "..............");
}

#[test]
fn unresolved_foreground_and_inherited_reverse_leave_literal_cells() {
    let data = [1_u64];
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(
        &Sparkline::new(&data).style(Style::new()),
        &mut images,
        1,
        1,
    );
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), "█");
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let reverse = Style::new()
        .bg(Color::Rgb(1, 2, 3))
        .with_attrs(Attrs::new().set_reverse(true));
    grid.fill(Rect::new(0, 0, 1, 1), reverse, '.', &mut clusters);
    grid.set_style(Rect::new(0, 0, 1, 1), reverse);
    Sparkline::new(&data).render(
        Rect::new(0, 0, 1, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(&mut images),
    );
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), "█");
    assert_eq!(
        grid.get(0, 0).map(|cell| cell.style().attrs.reverse()),
        Some(true)
    );
}
