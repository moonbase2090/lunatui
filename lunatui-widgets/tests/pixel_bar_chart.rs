// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::cell::Cell;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Bar, BarChart, BarDirection, BarGroup, RenderCx, Widget, WidgetState};

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
    chart: &BarChart<'_>,
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
fn default_vertical_fills_use_raw_values_beyond_eighth_cell_precision() {
    let bars = [Bar::new(1)];
    let chart = BarChart::from_bars(&bars).max(3);
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(&chart, &mut images, 1, 1);
    assert_eq!(images.images.len(), 1);
    let (source, area, depth) = &images.images[0];
    assert_eq!((*area, *depth), (Rect::new(0, 0, 1, 1), 0));
    assert_eq!(grid.row_display(0, &clusters), " ");
    assert_eq!(pixel(source, 6, 15), [0, 0, 0, 255]);
    let edge = pixel(source, 6, 16);
    assert!(edge[0] > 0 && edge[0] < 240, "AA edge: {edge:?}");
    assert_eq!(pixel(source, 6, 17), [240, 120, 60, 255]);
    assert_eq!(pixel(source, 6, 24), [240, 120, 60, 255]);
    let (cells, clusters) = draw(&chart.pixels(false), &mut capture(13, 25), 1, 1);
    assert_eq!(cells.row_display(0, &clusters), "▂");
}

#[test]
fn horizontal_fills_grow_right_with_fractional_pixel_edges() {
    let bars = [Bar::new(1)];
    let chart = BarChart::from_bars(&bars)
        .direction(BarDirection::Horizontal)
        .max(3);
    let mut images = capture(13, 25);
    draw(&chart, &mut images, 1, 1);
    assert_eq!(images.images.len(), 1);
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 3, 12), [240, 120, 60, 255]);
    let edge = pixel(source, 4, 12);
    assert!(edge[0] > 0 && edge[0] < 240, "AA edge: {edge:?}");
    assert_eq!(pixel(source, 5, 12), [0, 0, 0, 255]);
}

#[test]
fn values_with_the_same_cell_glyph_have_distinct_pixel_heights() {
    let first = [Bar::new(25)];
    let second = [Bar::new(30)];
    let mut a = capture(13, 100);
    let mut b = capture(13, 100);
    let chart_a = BarChart::from_bars(&first).max(100);
    let chart_b = BarChart::from_bars(&second).max(100);
    let (cells_a, clusters_a) = draw(&chart_a.clone().pixels(false), &mut a, 1, 1);
    let (cells_b, clusters_b) = draw(&chart_b.clone().pixels(false), &mut b, 1, 1);
    assert_eq!(cells_a.row_display(0, &clusters_a), "▂");
    assert_eq!(cells_b.row_display(0, &clusters_b), "▂");
    draw(&chart_a, &mut a, 1, 1);
    draw(&chart_b, &mut b, 1, 1);
    assert_eq!(pixel(&a.images[0].0, 6, 72), [0, 0, 0, 255]);
    assert_eq!(pixel(&b.images[0].0, 6, 72), [240, 120, 60, 255]);
}

#[test]
fn group_gaps_clipped_bars_and_labels_keep_the_original_layout() {
    let a = [Bar::new(8).label("a"), Bar::new(4).label("b")];
    let b = [Bar::new(8).label("c"), Bar::new(8).label("d")];
    let groups = [BarGroup::new(&a).label("A"), BarGroup::new(&b).label("B")];
    let chart = BarChart::from_groups(&groups).bar_width(2).group_gap(2);
    let mut images = capture(90, 40);
    let (grid, clusters) = draw(&chart, &mut images, 9, 4);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].1, Rect::new(0, 0, 9, 2));
    assert_eq!(grid.row_display(2, &clusters), "a  b   c ");
    assert_eq!(grid.row_display(3, &clusters), "  A      ");
    let source = &images.images[0].0;
    assert_eq!(pixel(source, 15, 5), [240, 120, 60, 255]);
    assert_eq!(pixel(source, 25, 35), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 35, 5), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 35, 35), [240, 120, 60, 255]);
    assert_eq!(pixel(source, 65, 35), [0, 0, 0, 255]);
    assert_eq!(pixel(source, 85, 5), [240, 120, 60, 255]);
}

#[test]
fn max_zero_clamping_and_large_values_do_not_overflow() {
    let bars = [Bar::new(u64::MAX)];
    for max in [1, u64::MAX] {
        let mut images = capture(13, 25);
        draw(&BarChart::from_bars(&bars).max(max), &mut images, 1, 1);
        assert_eq!(images.images.len(), 1);
        assert!(
            images.images[0]
                .0
                .rgba8_pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [240, 120, 60, 255])
        );
    }
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(&BarChart::from_bars(&bars).max(0), &mut images, 1, 1);
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), " ");
}

#[test]
fn value_text_preserves_block_glyphs_spaces_wide_cells_and_on_bar_background() {
    let bars = [Bar::new(10).text_value("█ 界")];
    let chart = BarChart::from_bars(&bars).bar_width(5);
    let mut images = capture(50, 20);
    let (grid, clusters) = draw(&chart, &mut images, 5, 2);
    assert_eq!(images.images.len(), 1);
    assert_eq!(images.images[0].2, -1);
    assert_eq!(grid.row_display(0, &clusters), "█ 界 ");
    assert_eq!(
        grid.get(2, 0).map(|c| c.style().bg),
        Some(Color::Rgb(240, 120, 60))
    );
    for x in 0..40 {
        assert_eq!(pixel(&images.images[0].0, x, 5), [240, 120, 60, 255]);
    }
    assert_eq!(pixel(&images.images[0].0, 45, 15), [240, 120, 60, 255]);
    assert_eq!(
        grid.get(3, 0).map(|c| c.meta().width),
        Some(lunatui_core::CellWidth::Continuation)
    );
}

#[test]
fn default_bar_foreground_keeps_the_raw_on_bar_background_inheritance() {
    let bars = [Bar::new(10).text_value("x").style(Style::new())];
    let chart = BarChart::from_bars(&bars).bar_width(2).background(
        Style::new()
            .fg(Color::Rgb(90, 80, 70))
            .bg(Color::Rgb(1, 2, 3)),
    );
    let mut images = capture(20, 20);
    let (grid, clusters) = draw(&chart, &mut images, 2, 2);
    assert_eq!(images.images.len(), 1);
    assert_eq!(grid.row_display(0, &clusters), "x ");
    assert_eq!(
        grid.get(0, 0).map(|c| c.style().bg),
        Some(Color::Rgb(1, 2, 3))
    );
    assert_eq!(pixel(&images.images[0].0, 5, 5), [1, 2, 3, 255]);
    assert_eq!(pixel(&images.images[0].0, 15, 5), [90, 80, 70, 255]);
}

#[test]
fn full_text_spans_restore_background_including_blank_and_wide_tail_cells() {
    let bars = [Bar::new(10).text_value("█ 界").style(Style::new())];
    let chart = BarChart::from_bars(&bars).bar_width(5).background(
        Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(1, 2, 3)),
    );
    let mut images = capture(50, 20);
    let (grid, clusters) = draw(&chart, &mut images, 5, 2);
    assert_eq!(images.images.len(), 1);
    assert_eq!(grid.row_display(0, &clusters), "█ 界 ");
    for x in 0..40 {
        assert_eq!(pixel(&images.images[0].0, x, 5), [1, 2, 3, 255]);
    }
    assert_eq!(pixel(&images.images[0].0, 45, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 25, 15), [255, 0, 0, 255]);
}

#[test]
fn explicit_background_allows_unknown_terminal_default_and_indexed_foreground() {
    let bars = [Bar::new(1).style(Style::new().fg(Color::Indexed(196)))];
    let chart = BarChart::from_bars(&bars).background(Style::new().bg(Color::Rgb(4, 5, 6)));
    let mut images = capture(20, 20);
    images.background = None;
    draw(&chart, &mut images, 2, 1);
    assert_eq!(images.images.len(), 1);
    assert_eq!(pixel(&images.images[0].0, 5, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&images.images[0].0, 15, 5), [4, 5, 6, 255]);
}

#[test]
fn per_bar_backgrounds_cover_vertical_tracks_but_only_horizontal_ink() {
    let bars = [Bar::new(1).style(
        Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 200)),
    )];
    for (direction, top, far) in [
        (BarDirection::Vertical, [0, 0, 200, 255], [0, 0, 200, 255]),
        (BarDirection::Horizontal, [255, 0, 0, 255], [0, 0, 0, 255]),
    ] {
        let mut images = capture(40, 40);
        draw(
            &BarChart::from_bars(&bars).direction(direction).max(4),
            &mut images,
            4,
            4,
        );
        assert_eq!(images.images.len(), 1);
        assert_eq!(pixel(&images.images[0].0, 5, 5), top);
        assert_eq!(pixel(&images.images[0].0, 5, 15), far);
    }
}

#[test]
fn disabled_state_dims_foreground_without_dimming_background_or_explicit_style() {
    let plain = [Bar::new(1)];
    let explicit = [Bar::new(1).style(Style::new().fg(Color::Rgb(255, 100, 50)))];
    for (state, bars, expected) in [
        (WidgetState::Normal, &plain[..], [240, 120, 60, 255]),
        (WidgetState::Focused, &plain[..], [100, 200, 80, 255]),
        (WidgetState::Disabled, &plain[..], [100, 90, 80, 255]),
        (WidgetState::Disabled, &explicit[..], [255, 100, 50, 255]),
    ] {
        let mut images = capture(20, 20);
        draw(
            &BarChart::from_bars(bars)
                .state(state)
                .background(Style::new().bg(Color::Rgb(10, 20, 30))),
            &mut images,
            2,
            1,
        );
        assert_eq!(images.images.len(), 1);
        assert_eq!(pixel(&images.images[0].0, 5, 10), expected);
        assert_eq!(pixel(&images.images[0].0, 15, 10), [10, 20, 30, 255]);
    }
}

#[test]
fn immediate_presenters_accept_outside_labels_and_decline_overlapping_text() {
    let plain = [Bar::new(1).label("A")];
    let value = [Bar::new(1).label("A").text_value("x")];
    for (bars, accepted) in [(&plain[..], true), (&value[..], false)] {
        let mut images = capture(20, 20);
        images.underlay = false;
        let chart = BarChart::from_bars(bars).bar_width(2);
        let (grid, clusters) = draw(&chart, &mut images, 2, 3);
        assert_eq!(!images.images.is_empty(), accepted);
        assert_eq!(grid.row_display(2, &clusters), "A ");
        if !accepted {
            assert_eq!(grid.row_display(0, &clusters), "x█");
        }
    }
    let bars = [Bar::new(2)];
    let groups = [BarGroup::new(&bars).label("heading")];
    let mut images = capture(80, 40);
    images.underlay = false;
    draw(
        &BarChart::from_groups(&groups).direction(BarDirection::Horizontal),
        &mut images,
        8,
        4,
    );
    assert_eq!(images.attempts, 0);
}

fn seeded_draw(chart: &BarChart<'_>, images: &mut Capture, area: Rect) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(14, 7);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, 14, 7),
        Style::new().fg(Color::Rgb(7, 8, 9)),
        '.',
        &mut clusters,
    );
    grid.put_str(1, 1, Style::new(), "界", &mut clusters);
    grid.put_str(10, 2, Style::new(), "界", &mut clusters);
    grid.put_str(0, 6, Style::new(), "e\u{301}", &mut clusters);
    grid.clear_dirty();
    chart.render(
        area,
        &mut RenderCx::new(&mut grid, &mut clusters, &theme()).image_sink(images),
    );
    (grid, clusters)
}

#[test]
fn all_early_and_late_declines_reproduce_complete_cells_and_dirty_bounds() {
    for mode in 0..15 {
        let mut images = capture(90, 50);
        let mut style = Style::new().fg(Color::Rgb(200, 100, 50));
        let mut label = "wide界";
        let mut area = Rect::new(2, 1, 9, 5);
        match mode {
            0 => images.accept = false,
            1 => images.size = None,
            2 => images.background = None,
            3 => images.underlay = false,
            4 => images.size = PixelSize::new(4097, 50),
            5 => images.size = PixelSize::new(2048, 1024),
            6 => images.size = PixelSize::new(8, 50),
            7 => style = style.with_attrs(Attrs::new().set_reverse(true)),
            8 => style = style.with_attrs(Attrs::new().set_bold(true)),
            9 => style = Style::new(),
            10 => label = "e\u{301}",
            11 => label = "\u{7}x",
            12 => area = Rect::new(2, 1, 9, 1),
            13 => area = Rect::new(2, 1, 14, 5),
            _ => (),
        }
        let bars = [
            Bar::new(3).label(label).text_value("v").style(style),
            Bar::new(1)
                .label("B")
                .style(if mode == 14 { Style::new() } else { style }),
        ];
        let chart = BarChart::from_bars(&bars).bar_width(4).max(3);
        let (actual, clusters) = seeded_draw(&chart, &mut images, area);
        let (expected, expected_clusters) =
            seeded_draw(&chart.pixels(false), &mut capture(90, 50), area);
        assert_eq!(images.images.len(), 0, "decline mode {mode}");
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
        assert_eq!(actual.row_display(6, &clusters), "e\u{301}.............");
    }
}

#[test]
fn explicit_cells_and_incapable_sinks_do_not_query_native_metrics() {
    let bars = [Bar::new(1)];
    for enabled in [false, true] {
        let mut images = capture(13, 25);
        images.supported = !enabled;
        let (grid, clusters) = draw(
            &BarChart::from_bars(&bars).pixels(enabled),
            &mut images,
            1,
            1,
        );
        assert_eq!(grid.row_display(0, &clusters), "█");
        assert_eq!(images.queries.get(), 0);
        assert_eq!(images.attempts, 0);
    }
}

#[test]
fn unbounded_input_is_declined_before_native_allocation_or_submission() {
    let bars = vec![Bar::new(1); 65_536];
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(&BarChart::from_bars(&bars), &mut images, 1, 1);
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), "█");
    let text = "x".repeat(1_048_577);
    let bars = [Bar::new(1).text_value(&text)];
    let mut images = capture(13, 25);
    let (grid, clusters) = draw(&BarChart::from_bars(&bars), &mut images, 1, 1);
    assert_eq!(images.attempts, 0);
    assert_eq!(grid.row_display(0, &clusters), "…");
}
