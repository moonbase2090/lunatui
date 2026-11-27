// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Baseline, List, RenderCx, RenderDirection, Sparkline, SparklineBar, VecSource, Widget,
};

const CELL: (u32, u32) = (16, 32);
const BACK: Color = Color::Rgb(10, 20, 30);
const RED: Color = Color::Rgb(220, 40, 30);
const GREEN: Color = Color::Rgb(40, 200, 90);
const BLUE: Color = Color::Rgb(60, 120, 250);
const SELECTED: Color = Color::Rgb(50, 50, 70);

struct Host {
    accept: bool,
    images: Vec<(Rect, ImageSource)>,
}

impl Host {
    const fn new(accept: bool) -> Self {
        Self {
            accept,
            images: Vec::new(),
        }
    }
}

impl ImageSink for Host {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        Some(PixelTarget::new(
            PixelSize::new(
                u32::from(area.width) * CELL.0,
                u32::from(area.height) * CELL.1,
            )?,
            false,
            Some([0, 0, 0]),
        ))
    }
    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }
    fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
        if self.accept {
            self.images.push((area, source.clone()));
        }
        self.accept
    }
}

fn render(
    widget: &dyn Widget,
    area: Rect,
    mode: RenderMode,
    host: Option<&mut Host>,
) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(area.x + area.width + 2, area.y + area.height + 1);
    let mut clusters = ClusterTable::new();
    let all = Rect::new(0, 0, grid.width(), grid.height());
    grid.fill(all, Style::new().bg(BACK), 'x', &mut clusters);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).render_mode(mode);
    if let Some(host) = host {
        cx = cx.image_sink(host);
    }
    widget.render(area, &mut cx);
    (grid, clusters)
}

fn cells(grid: &Grid) -> Vec<u128> {
    (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .map(|(x, y)| grid.cell_bits(x, y))
        .collect()
}

fn pixel(image: &ImageSource, x: u32, y: u32) -> [u8; 3] {
    let index = usize::try_from((y * image.size().width() + x) * 4).unwrap_or(usize::MAX);
    let bytes = image.rgba8_pixels();
    [bytes[index], bytes[index + 1], bytes[index + 2]]
}

fn rgb(color: Color) -> [u8; 3] {
    assert!(
        matches!(color, Color::Rgb(..)),
        "{color:?} is not an RGB color"
    );
    match color {
        Color::Rgb(red, green, blue) => [red, green, blue],
        _ => [0, 0, 0],
    }
}

fn column_ink(image: &ImageSource, x: u32, color: [u8; 3]) -> u32 {
    (0..image.size().height())
        .filter(|&y| pixel(image, x, y) == color)
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn bars(values: &[Option<u64>], color: Color) -> Vec<SparklineBar> {
    values
        .iter()
        .map(|value| SparklineBar::from(*value).style(Style::new().fg(color)))
        .collect()
}

fn assert_same_cells_unless_painted(widget: &dyn Widget, area: Rect) {
    let (expected, _) = render(widget, area, RenderMode::Cell, None);
    let (plain, _) = render(widget, area, RenderMode::Graphite, None);
    assert_eq!(
        cells(&plain),
        cells(&expected),
        "Graphite mode without a sink"
    );
    let (cell_declined, _) = render(widget, area, RenderMode::Cell, Some(&mut Host::new(false)));
    let (graphite_declined, _) = render(
        widget,
        area,
        RenderMode::Graphite,
        Some(&mut Host::new(false)),
    );
    assert_eq!(
        cells(&graphite_declined),
        cells(&cell_declined),
        "a declining sink"
    );
}

#[test]
fn a_sparkline_draws_the_cell_mode_cells_when_pixels_are_not_shown() {
    let data = bars(&[Some(0), Some(3), None, Some(8), Some(5)], RED);
    let variants = [
        Sparkline::new(&data),
        Sparkline::new(&data).max(4),
        Sparkline::new(&data).pixels(false),
        Sparkline::new(&data).direction(RenderDirection::RightToLeft),
        Sparkline::new(&data).baseline(Baseline::Minimum),
        Sparkline::new(&data).style(Style::new().fg(GREEN).bg(BACK)),
    ];
    for sparkline in &variants {
        for area in [
            Rect::new(1, 1, 5, 1),
            Rect::new(0, 0, 8, 3),
            Rect::new(2, 0, 3, 2),
        ] {
            assert_same_cells_unless_painted(sparkline, area);
        }
    }
}

#[test]
fn a_sparkline_without_pixels_keeps_its_cells_in_graphite_mode() {
    let data = bars(&[Some(0), Some(3), None, Some(8), Some(5)], RED);
    let chart = Sparkline::new(&data).pixels(false);
    let area = Rect::new(1, 1, 5, 2);
    let (expected, _) = render(&chart, area, RenderMode::Cell, None);
    let mut host = Host::new(true);
    let (graphite, _) = render(&chart, area, RenderMode::Graphite, Some(&mut host));
    assert!(
        host.images.is_empty(),
        "no image for a cells-only sparkline"
    );
    assert_eq!(cells(&graphite), cells(&expected));
}

#[test]
fn graphite_lane_bars_rise_from_a_hairline_in_their_own_colors() {
    let mut data = bars(&[Some(0), Some(4), Some(8)], RED);
    data.push(SparklineBar::value(8).style(Style::new().fg(GREEN)));
    data.push(SparklineBar::absent());
    let sparkline = Sparkline::new(&data).max(8);
    let area = Rect::new(1, 0, 5, 2);
    let mut host = Host::new(true);
    let (grid, clusters) = render(&sparkline, area, RenderMode::Graphite, Some(&mut host));
    assert_eq!(host.images.len(), 1, "one image for the lane");
    let (at, image) = &host.images[0];
    assert_eq!(*at, area);
    assert_eq!(grid.row_display(0, &clusters), "x     xx");
    let centre = |column: u32| column * CELL.0 + CELL.0 / 2;
    let red = rgb(RED);
    let [empty, half, full] = [0, 1, 2].map(|column| column_ink(image, centre(column), red));
    assert_eq!(empty, 0, "a zero sample draws no bar");
    assert!(half > 10 && full > 10, "{half} {full}");
    assert!(
        full.abs_diff(half * 2) <= 2,
        "a sample twice as large stands twice as tall: {half} {full}"
    );
    assert_eq!(
        column_ink(image, centre(3), rgb(GREEN)),
        full,
        "the bar's own color"
    );
    assert_eq!(
        column_ink(image, centre(4), red),
        0,
        "an absent sample draws nothing"
    );
    let height = image.size().height();
    assert!(
        full * 4 > height * 3,
        "the maximum fills most of the lane: {full} of {height}"
    );
    assert_eq!(
        pixel(image, centre(2), height - 3),
        red,
        "bars stand on the bottom"
    );
    assert_eq!(
        pixel(image, centre(2), 1),
        rgb(BACK),
        "and stop below the top"
    );
    let gap = centre(1) - CELL.0 / 2;
    let line = pixel(image, gap, height - 1);
    assert_ne!(line, rgb(BACK), "the hairline crosses the gap between bars");
    assert_eq!(
        pixel(image, gap, height - 4),
        rgb(BACK),
        "above the hairline"
    );
}

#[test]
fn a_right_to_left_lane_puts_the_first_sample_at_the_right() {
    let mut data = bars(&[Some(8)], RED);
    data.extend(bars(&[Some(0), Some(0)], RED));
    let sparkline = Sparkline::new(&data)
        .max(8)
        .direction(RenderDirection::RightToLeft);
    let mut host = Host::new(true);
    render(
        &sparkline,
        Rect::new(0, 0, 5, 1),
        RenderMode::Graphite,
        Some(&mut host),
    );
    let (at, image) = &host.images[0];
    assert_eq!(*at, Rect::new(2, 0, 3, 1), "the three columns at the right");
    let red = rgb(RED);
    assert!(column_ink(image, 2 * CELL.0 + CELL.0 / 2, red) > 0);
    assert_eq!(column_ink(image, CELL.0 / 2, red), 0);
}

fn list(selected: Option<usize>) -> List<VecSource> {
    let rows = ["builder", "reviewer", "worker"]
        .iter()
        .map(|name| vec![(*name).to_owned()])
        .collect();
    List::new(VecSource::from_rows(rows))
        .style(Style::new().fg(Color::Rgb(220, 220, 230)).bg(BACK))
        .highlight_style(
            Style::new()
                .fg(Color::Rgb(250, 250, 250))
                .bg(SELECTED)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .highlight_symbol("▌", Style::new().fg(BLUE))
        .icons(vec![
            ("×".to_owned(), Style::new().fg(RED)),
            ("◆".to_owned(), Style::new().fg(Color::Rgb(250, 190, 50))),
            ("●".to_owned(), Style::new().fg(GREEN)),
        ])
        .marks(
            vec![
                Some(("54s".to_owned(), Style::new().fg(RED))),
                None,
                Some(("52s".to_owned(), Style::new())),
            ],
            1,
        )
        .selected(selected)
}

#[test]
fn a_list_draws_the_cell_mode_cells_when_pixels_are_not_shown() {
    for selected in [None, Some(0), Some(2)] {
        for area in [
            Rect::new(0, 0, 20, 3),
            Rect::new(1, 1, 4, 2),
            Rect::new(0, 0, 12, 5),
        ] {
            assert_same_cells_unless_painted(&list(selected), area);
        }
    }
}

#[test]
fn a_graphite_selected_row_is_rounded_with_a_focus_bar() {
    let area = Rect::new(0, 0, 20, 3);
    let mut host = Host::new(true);
    let (grid, clusters) = render(&list(Some(1)), area, RenderMode::Graphite, Some(&mut host));
    assert_eq!(host.images.len(), 3, "one image per row");
    for (index, (at, _)) in host.images.iter().enumerate() {
        let row = u16::try_from(index).unwrap_or(0);
        assert_eq!(*at, Rect::new(0, row, 20, 1));
        assert_eq!(
            grid.row_display(row, &clusters),
            format!("{}xx", " ".repeat(20))
        );
    }
    let selected = &host.images[1].1;
    let middle = CELL.1 / 2;
    assert_eq!(
        pixel(selected, 0, 0),
        rgb(BACK),
        "the rounded corner shows the list"
    );
    assert_eq!(pixel(selected, 1, middle), rgb(BLUE), "the focus bar");
    assert_eq!(pixel(selected, 12, middle), rgb(SELECTED), "the selection");
    assert_eq!(
        pixel(selected, 200, 3),
        rgb(SELECTED),
        "the selection spans the row"
    );
    assert_eq!(pixel(selected, 200, 0), rgb(BACK), "above the selection");
    let (right, top) = (20 * CELL.0 - 1, CELL.1 / 16);
    assert_eq!(
        pixel(selected, right, top),
        rgb(BACK),
        "the band's right corner is rounded"
    );
    assert_eq!(
        pixel(selected, right, middle),
        rgb(SELECTED),
        "the band reaches the right edge"
    );
    let plain = &host.images[0].1;
    assert_eq!(
        pixel(plain, 1, middle),
        rgb(BACK),
        "no bar on an unselected row"
    );
    assert_eq!(
        pixel(plain, 200, 3),
        rgb(BACK),
        "no selection on an unselected row"
    );
}

#[test]
fn graphite_rows_draw_the_icon_text_and_a_right_aligned_mark() {
    let area = Rect::new(0, 0, 20, 3);
    let mut host = Host::new(true);
    render(&list(None), area, RenderMode::Graphite, Some(&mut host));
    let row = &host.images[0].1;
    let red = rgb(RED);
    let ink: Vec<u32> = (0..row.size().width())
        .filter(|&x| column_ink(row, x, red) > 0)
        .collect();
    let mark: Vec<u32> = ink.iter().copied().filter(|&x| x >= 2 * CELL.0).collect();
    let reddish = (CELL.0..2 * CELL.0)
        .flat_map(|x| (0..CELL.1).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            let [r, g, b] = pixel(row, x, y);
            r > 120 && u16::from(r) > u16::from(g) + 60 && u16::from(r) > u16::from(b) + 60
        })
        .count();
    assert!(
        reddish > 4,
        "the icon is drawn in its own color in the cell after the gutter: {reddish}"
    );
    let left_of_gutter = (0..CELL.0)
        .flat_map(|x| (0..CELL.1).map(move |y| (x, y)))
        .filter(|&(x, y)| pixel(row, x, y) != rgb(BACK))
        .count();
    assert_eq!(
        left_of_gutter, 0,
        "an unselected row leaves the gutter empty"
    );
    let right = 19 * CELL.0;
    assert!(
        mark.last().is_some_and(|&x| x < right && x + 6 >= right),
        "the mark ends at the padding: {mark:?}"
    );
    assert!(
        mark.first().is_some_and(|&x| x > 15 * CELL.0),
        "the mark is right-aligned, not drawn from its cells' left edge: {mark:?}"
    );
    let text = rgb(Color::Rgb(220, 220, 230));
    let text_ink = (2 * CELL.0..10 * CELL.0)
        .filter(|&x| column_ink(row, x, text) > 0)
        .count();
    assert!(text_ink > 20, "the row text is drawn: {text_ink} columns");
}

fn lane_image(values: &[Option<u64>], striped: bool) -> ImageSource {
    let data = bars(values, RED);
    let chart = Sparkline::new(&data).max(8);
    let area = Rect::new(0, 0, u16::try_from(values.len()).unwrap_or(0), 2);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(area.width, area.height);
    let mut clusters = ClusterTable::new();
    for column in 0..area.width {
        let back = if striped && column % 2 == 1 {
            Color::Rgb(30, 40, 60)
        } else {
            BACK
        };
        grid.fill(
            Rect::new(column, 0, 1, area.height),
            Style::new().bg(back),
            ' ',
            &mut clusters,
        );
    }
    let mut host = Host::new(true);
    {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme)
            .render_mode(RenderMode::Graphite)
            .image_sink(&mut host);
        chart.render(area, &mut cx);
    }
    assert_eq!(host.images.len(), 1, "one image for the lane");
    host.images.remove(0).1
}

fn column_block(image: &ImageSource, column: u32) -> Vec<[u8; 3]> {
    (0..image.size().height())
        .flat_map(|y| (column * CELL.0..(column + 1) * CELL.0).map(move |x| (x, y)))
        .map(|(x, y)| pixel(image, x, y))
        .collect()
}

#[test]
fn repeated_lane_bars_match_bars_drawn_alone() {
    let values = [
        Some(5),
        Some(5),
        Some(8),
        Some(5),
        Some(8),
        Some(2),
        Some(5),
        None,
        Some(8),
        Some(2),
    ];
    for striped in [false, true] {
        let lane = lane_image(&values, striped);
        assert_eq!(
            pixel(&lane, 0, 0) != pixel(&lane, CELL.0, 0),
            striped,
            "the backdrop under the lane is striped: {striped}"
        );
        for column in 0..values.len() {
            let alone: Vec<Option<u64>> = (0..values.len())
                .map(|index| if index == column { values[index] } else { None })
                .collect();
            let single = lane_image(&alone, striped);
            let at = u32::try_from(column).unwrap_or(0);
            assert!(
                column_block(&lane, at) == column_block(&single, at),
                "striped {striped}: column {column} differs from the bar drawn alone"
            );
        }
    }
}
