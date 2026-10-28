// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Rect, Size, Style, clip_display_prefix, text_display_width};
use lunatui_layout::Constraint;
use lunatui_theme::Token;

use crate::{Block, BorderType, RenderCx, Widget, WidgetState};

fn clip_area_to_coordinates(area: Rect) -> Rect {
    Rect::new(
        area.x,
        area.y,
        area.width.min(u16::MAX - area.x),
        area.height.min(u16::MAX - area.y),
    )
}

const SERIES: [Token; 6] = [
    Token::Accent,
    Token::FocusRing,
    Token::Warn,
    Token::Fail,
    Token::Ok,
    Token::TextPrimary,
];

/// Glyphs a dataset is drawn with.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Marker {
    /// One `•` per point, a cell each.
    Dot,
    /// One full block per point, a cell each.
    Block,
    /// A cell split into two stacked halves.
    HalfBlock,
    /// A cell split into a 2×4 grid of Braille dots, the finest resolution.
    #[default]
    Braille,
}

/// How a dataset's points are drawn.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphType {
    /// Each point alone.
    #[default]
    Scatter,
    /// Points joined by straight lines.
    Line,
    /// A bar from the bottom of the plot up to each point, using eighth blocks.
    Bar,
}

/// Where the legend sits inside the plot.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegendPosition {
    /// Top edge, centered.
    Top,
    /// Top-right corner.
    #[default]
    TopRight,
    /// Top-left corner.
    TopLeft,
    /// Left edge, centered vertically.
    Left,
    /// Right edge, centered vertically.
    Right,
    /// Bottom edge, centered.
    Bottom,
    /// Bottom-right corner.
    BottomRight,
    /// Bottom-left corner.
    BottomLeft,
}

/// One series of `(x, y)` points.
#[derive(Clone, Copy, Debug)]
pub struct Dataset<'a> {
    name: Option<&'a str>,
    data: &'a [(f64, f64)],
    marker: Marker,
    graph_type: GraphType,
    style: Option<Style>,
}

impl<'a> Dataset<'a> {
    /// A Braille scatter series over `data`, colored by its position in the chart.
    #[must_use]
    pub const fn new(data: &'a [(f64, f64)]) -> Self {
        Self {
            name: None,
            data,
            marker: Marker::Braille,
            graph_type: GraphType::Scatter,
            style: None,
        }
    }

    /// Name shown in the legend. A dataset without a name is left out of the legend.
    #[must_use]
    pub const fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Glyphs the series is drawn with.
    #[must_use]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// How the points are drawn.
    #[must_use]
    pub const fn graph_type(mut self, graph_type: GraphType) -> Self {
        self.graph_type = graph_type;
        self
    }

    /// Style of the series, replacing the theme color for its position.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

/// One axis: the data range it shows, a title, and evenly spaced labels.
#[derive(Clone, Copy, Debug)]
pub struct Axis<'a> {
    title: Option<&'a str>,
    bounds: [f64; 2],
    labels: &'a [&'a str],
    style: Option<Style>,
    label_style: Option<Style>,
}

impl<'a> Axis<'a> {
    /// An axis showing `0.0` to `1.0`, without a title or labels.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            title: None,
            bounds: [0.0, 1.0],
            labels: &[],
            style: None,
            label_style: None,
        }
    }

    /// Title: above the y axis, right-aligned under the x axis.
    #[must_use]
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Range of data values the axis covers. A range of zero width plots nothing, and a reversed
    /// range flips the axis.
    #[must_use]
    pub const fn bounds(mut self, bounds: [f64; 2]) -> Self {
        self.bounds = bounds;
        self
    }

    /// Labels spaced evenly along the axis, first at the origin end and last at the far end.
    #[must_use]
    pub const fn labels(mut self, labels: &'a [&'a str]) -> Self {
        self.labels = labels;
        self
    }

    /// Style of the axis line.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Style of the axis labels and title.
    #[must_use]
    pub const fn label_style(mut self, style: Style) -> Self {
        self.label_style = Some(style);
        self
    }
}

impl Default for Axis<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Line, scatter and bar chart over one or more datasets, with axes and a legend.
///
/// Datasets are drawn in order, so later ones sit on top. Series colors cycle through theme
/// tokens unless a dataset sets a style; axes use dimmed text, the focus ring when focused, and
/// everything dims when disabled. The legend lists named datasets with a `■` swatch and is hidden
/// when it would take more than half of the plot, which
/// [`hidden_legend_constraints`](Self::hidden_legend_constraints) changes.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{Axis, Chart, Dataset, GraphType, Marker, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(12, 5);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// let data = [(0.0, 0.0), (1.0, 1.0)];
/// let sets = [Dataset::new(&data).marker(Marker::Block).graph_type(GraphType::Line)];
/// Chart::new(&sets)
///     .x_axis(Axis::new().bounds([0.0, 1.0]))
///     .y_axis(Axis::new().bounds([0.0, 1.0]))
///     .render(Rect::new(0, 0, 12, 5), &mut cx);
/// assert_eq!(grid.row_display(0, &clusters), "│         ██");
/// ```
#[derive(Clone, Debug)]
pub struct Chart<'a> {
    datasets: &'a [Dataset<'a>],
    x_axis: Axis<'a>,
    y_axis: Axis<'a>,
    legend: Option<LegendPosition>,
    hidden: (Constraint, Constraint),
    state: WidgetState,
    background: Option<Style>,
}

impl<'a> Chart<'a> {
    /// A chart of `datasets`, with default axes and a top-right legend.
    #[must_use]
    pub const fn new(datasets: &'a [Dataset<'a>]) -> Self {
        Self {
            datasets,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            legend: Some(LegendPosition::TopRight),
            hidden: (Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)),
            state: WidgetState::Normal,
            background: None,
        }
    }

    /// Style the area is reset to before anything is drawn, such as a panel background. Without
    /// one, cells are reset to the default style.
    #[must_use]
    pub const fn background(mut self, style: Style) -> Self {
        self.background = Some(style);
        self
    }

    /// The horizontal axis.
    #[must_use]
    pub const fn x_axis(mut self, axis: Axis<'a>) -> Self {
        self.x_axis = axis;
        self
    }

    /// The vertical axis.
    #[must_use]
    pub const fn y_axis(mut self, axis: Axis<'a>) -> Self {
        self.y_axis = axis;
        self
    }

    /// Where the legend sits; `None` hides it.
    #[must_use]
    pub const fn legend_position(mut self, position: Option<LegendPosition>) -> Self {
        self.legend = position;
        self
    }

    /// Largest width and height the legend may take, measured against the plot. A larger legend
    /// is not drawn. The default is half of each.
    #[must_use]
    pub const fn hidden_legend_constraints(
        mut self,
        width: Constraint,
        height: Constraint,
    ) -> Self {
        self.hidden = (width, height);
        self
    }

    /// Interaction state used for colors that are not set explicitly.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ink {
    Empty,
    Dot,
    Block,
    Half,
    Braille,
    Eighths,
}

#[derive(Clone, Copy)]
struct Spot {
    ink: Ink,
    mask: u8,
    series: usize,
}

const EMPTY_SPOT: Spot = Spot {
    ink: Ink::Empty,
    mask: 0,
    series: 0,
};

const BRAILLE_BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
const EIGHTHS_UP: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

struct Plot {
    rect: Rect,
    spots: Vec<Spot>,
}

impl Plot {
    fn new(rect: Rect) -> Self {
        Self {
            rect,
            spots: vec![EMPTY_SPOT; usize::from(rect.width) * usize::from(rect.height)],
        }
    }

    fn spot(&mut self, column: u16, row: u16) -> Option<&mut Spot> {
        let index = usize::from(row) * usize::from(self.rect.width) + usize::from(column);
        self.spots.get_mut(index)
    }

    fn resolution(&self, marker: Marker) -> (i64, i64) {
        let (width, height) = (i64::from(self.rect.width), i64::from(self.rect.height));
        match marker {
            Marker::Braille => (width * 2, height * 4),
            Marker::HalfBlock => (width, height * 2),
            Marker::Dot | Marker::Block => (width, height),
        }
    }

    fn mark(&mut self, marker: Marker, x: i64, y: i64, series: usize) {
        let (ink, cell_x, cell_y, mask) = match marker {
            Marker::Braille => (
                Ink::Braille,
                x / 2,
                y / 4,
                BRAILLE_BITS[usize::try_from(x % 2).unwrap_or(0)]
                    [usize::try_from(y % 4).unwrap_or(0)],
            ),
            Marker::HalfBlock => (Ink::Half, x, y / 2, 1 << (y % 2)),
            Marker::Dot => (Ink::Dot, x, y, 1),
            Marker::Block => (Ink::Block, x, y, 1),
        };
        let Some(spot) = self.spot(
            u16::try_from(cell_x).unwrap_or(0),
            u16::try_from(cell_y).unwrap_or(0),
        ) else {
            return;
        };
        if spot.ink == ink {
            spot.mask |= mask;
        } else {
            spot.mask = mask;
        }
        spot.ink = ink;
        spot.series = series;
    }

    fn overlay(&mut self, layer: &Self) {
        for (spot, over) in self.spots.iter_mut().zip(&layer.spots) {
            if over.ink != Ink::Empty {
                *spot = *over;
            }
        }
    }

    fn eighths(&mut self, column: u16, ticks: i64, series: usize) {
        let height = i64::from(self.rect.height);
        let ticks = ticks.clamp(0, height * 8);
        for row in 0..height {
            let from_bottom = height - 1 - row;
            let filled = (ticks - from_bottom * 8).clamp(0, 8);
            if filled == 0 {
                continue;
            }
            let Some(spot) = self.spot(column, u16::try_from(row).unwrap_or(0)) else {
                continue;
            };
            spot.ink = Ink::Eighths;
            spot.mask = u8::try_from(filled).unwrap_or(8);
            spot.series = series;
        }
    }
}

#[derive(Clone, Copy)]
struct Window {
    x: [f64; 2],
    y: [f64; 2],
}

const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const BOTTOM: u8 = 4;
const TOP: u8 = 8;

fn round_to_tick(value: f64) -> i64 {
    let bits = value.round().to_bits();
    let exponent = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0) - 1023;
    if exponent < 0 {
        return 0;
    }
    let mantissa = (bits & ((1_u64 << 52) - 1)) + (1_u64 << 52);
    let shift = u32::try_from(52 - exponent.min(52)).unwrap_or(0);
    i64::try_from(mantissa >> shift).unwrap_or(i64::MAX)
}

fn span(cells: i64) -> f64 {
    f64::from(i32::try_from(cells).unwrap_or(i32::MAX))
}

impl Window {
    fn width(self) -> f64 {
        self.x[1] - self.x[0]
    }

    fn height(self) -> f64 {
        self.y[1] - self.y[0]
    }

    fn usable(self) -> bool {
        self.width().is_finite()
            && self.height().is_finite()
            && self.width() != 0.0
            && self.height() != 0.0
    }

    fn region(self, (x, y): (f64, f64)) -> u8 {
        let (left, right) = (self.x[0].min(self.x[1]), self.x[0].max(self.x[1]));
        let (bottom, top) = (self.y[0].min(self.y[1]), self.y[0].max(self.y[1]));
        let mut region = 0;
        if x < left {
            region |= LEFT;
        } else if x > right {
            region |= RIGHT;
        }
        if y < bottom {
            region |= BOTTOM;
        } else if y > top {
            region |= TOP;
        }
        region
    }

    fn contains(self, point: (f64, f64)) -> bool {
        self.region(point) == 0
    }

    fn cut(self, from: (f64, f64), toward: (f64, f64), region: u8) -> (f64, f64) {
        let (left, right) = (self.x[0].min(self.x[1]), self.x[0].max(self.x[1]));
        let (bottom, top) = (self.y[0].min(self.y[1]), self.y[0].max(self.y[1]));
        let (dx, dy) = (toward.0 - from.0, toward.1 - from.1);
        if region & LEFT != 0 {
            (left, from.1 + (left - from.0) * dy / dx)
        } else if region & RIGHT != 0 {
            (right, from.1 + (right - from.0) * dy / dx)
        } else if region & BOTTOM != 0 {
            (from.0 + (bottom - from.1) * dx / dy, bottom)
        } else {
            (from.0 + (top - from.1) * dx / dy, top)
        }
    }

    fn clip(self, mut a: (f64, f64), mut b: (f64, f64)) -> Option<((f64, f64), (f64, f64))> {
        for _ in 0..8 {
            let (ra, rb) = (self.region(a), self.region(b));
            if ra & rb != 0 {
                return None;
            }
            if ra != 0 {
                a = self.cut(a, b, ra);
            } else if rb != 0 {
                b = self.cut(b, a, rb);
            } else {
                return Some((a, b));
            }
        }
        None
    }

    fn column(self, x: f64, cells: i64) -> i64 {
        round_to_tick((x - self.x[0]) * span(cells - 1) / self.width())
    }

    fn row(self, y: f64, cells: i64) -> i64 {
        round_to_tick((self.y[1] - y) * span(cells - 1) / self.height())
    }

    fn ticks(self, y: f64, eighths: i64) -> i64 {
        round_to_tick((y - self.y[0]) * span(eighths) / self.height())
    }
}

fn glyph(spot: Spot) -> char {
    match spot.ink {
        Ink::Empty => ' ',
        Ink::Dot => '•',
        Ink::Block => '█',
        Ink::Half => match spot.mask {
            1 => '▀',
            2 => '▄',
            _ => '█',
        },
        Ink::Braille => char::from_u32(0x2800 + u32::from(spot.mask)).unwrap_or(' '),
        Ink::Eighths => EIGHTHS_UP[usize::from(spot.mask.min(8))],
    }
}

struct Look {
    axis: Style,
    text: Style,
    legend_bg: Color,
}

fn put_text(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    let mut column = x;
    for ch in text.chars() {
        cx.put_char_patched(column, y, ch, style);
        let mut buffer = [0_u8; 4];
        let used = text_display_width(ch.encode_utf8(&mut buffer));
        column = column.saturating_add(u16::try_from(used).unwrap_or(1));
    }
}

fn put_fit(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    put_text(cx, x, y, clip_display_prefix(text, width), style);
}

fn label_width(text: &str) -> u16 {
    u16::try_from(text_display_width(text)).unwrap_or(u16::MAX)
}

fn resolve(limit: Constraint, total: u16) -> u16 {
    match limit {
        Constraint::Min(cells) | Constraint::Max(cells) | Constraint::Length(cells) => cells,
        Constraint::Percentage(percent) => {
            u16::try_from(u32::from(total) * u32::from(percent) / 100).unwrap_or(u16::MAX)
        }
        Constraint::Ratio(numer, denom) => {
            u16::try_from(u64::from(total) * u64::from(numer) / u64::from(denom.max(1)))
                .unwrap_or(u16::MAX)
        }
        Constraint::Fill(_) => total,
    }
}

impl Chart<'_> {
    fn look(&self, cx: &RenderCx<'_>) -> Look {
        let dim = Attrs::new().set_dim(true);
        let (axis_token, attrs) = match self.state {
            WidgetState::Focused => (Token::FocusRing, Attrs::new()),
            _ => (Token::TextPrimary, dim),
        };
        let text = Style::new().fg(cx.token(Token::TextPrimary)).with_attrs(
            if self.state == WidgetState::Disabled {
                dim
            } else {
                Attrs::new()
            },
        );
        Look {
            axis: Style::new().fg(cx.token(axis_token)).with_attrs(attrs),
            text,
            legend_bg: cx.token(Token::Surface0),
        }
    }

    fn series_style(&self, cx: &RenderCx<'_>, index: usize) -> Style {
        if let Some(style) = self.datasets.get(index).and_then(|set| set.style) {
            return style;
        }
        let attrs = Attrs::new().set_dim(self.state == WidgetState::Disabled);
        Style::new()
            .fg(cx.token(SERIES[index % SERIES.len()]))
            .with_attrs(attrs)
    }

    fn draw_axes(&self, area: Rect, plot: Rect, look: &Look, cx: &mut RenderCx<'_>) {
        let axis_x = plot.x - 1;
        let axis_y = plot.y.saturating_add(plot.height);
        let x_style = self.x_axis.style.unwrap_or(look.axis);
        let y_style = self.y_axis.style.unwrap_or(look.axis);
        for row in 0..plot.height {
            put_text(cx, axis_x, plot.y.saturating_add(row), "│", y_style);
        }
        put_text(cx, axis_x, axis_y, "└", x_style);
        for column in 0..plot.width {
            put_text(cx, plot.x.saturating_add(column), axis_y, "─", x_style);
        }
        let labels = self.y_axis.labels;
        let gutter = plot.x - 1 - area.x;
        for (index, label) in labels.iter().enumerate() {
            let row = if labels.len() <= 1 {
                plot.height - 1
            } else {
                plot.height
                    - 1
                    - u16::try_from(index * usize::from(plot.height - 1) / (labels.len() - 1))
                        .unwrap_or(0)
            };
            let width = label_width(label).min(gutter);
            put_fit(
                cx,
                area.x.saturating_add(gutter).saturating_sub(width),
                plot.y.saturating_add(row),
                width,
                label,
                self.y_axis.label_style.unwrap_or(look.text),
            );
        }
        if let Some(title) = self.y_axis.title {
            put_fit(
                cx,
                area.x,
                area.y,
                area.width,
                title,
                self.y_axis.label_style.unwrap_or(look.text),
            );
        }
        self.draw_x_labels(area, plot, axis_y, look, cx);
    }

    fn draw_x_labels(
        &self,
        area: Rect,
        plot: Rect,
        axis_y: u16,
        look: &Look,
        cx: &mut RenderCx<'_>,
    ) {
        let labels = self.x_axis.labels;
        let style = self.x_axis.label_style.unwrap_or(look.text);
        let row = axis_y.saturating_add(1);
        let right = plot.x.saturating_add(plot.width);
        let mut last_end = plot.x;
        for (index, label) in labels.iter().enumerate() {
            let width = label_width(label).min(plot.width);
            let anchor = if labels.len() <= 1 {
                0
            } else {
                u16::try_from(index * usize::from(plot.width) / (labels.len() - 1)).unwrap_or(0)
            };
            let start = if index == 0 {
                plot.x.saturating_sub(width).max(area.x)
            } else {
                plot.x
                    .saturating_add(anchor)
                    .saturating_sub(width / 2)
                    .max(plot.x)
            };
            let start = start.min(right - width);
            if index > 0 && start <= last_end {
                continue;
            }
            put_fit(cx, start, row, width, label, style);
            last_end = start.saturating_add(width);
        }
        if let Some(title) = self.x_axis.title {
            let width = label_width(title).min(plot.width);
            let title_row = axis_y
                .saturating_add(1)
                .saturating_add(u16::from(!labels.is_empty()));
            put_fit(cx, right - width, title_row, width, title, style);
        }
    }

    fn rasterize(&self, plot: &mut Plot) {
        let window = Window {
            x: self.x_axis.bounds,
            y: self.y_axis.bounds,
        };
        if !window.usable() {
            return;
        }
        for (series, set) in self.datasets.iter().enumerate() {
            let mut layer = Plot::new(plot.rect);
            let points: Vec<(f64, f64)> = set
                .data
                .iter()
                .copied()
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect();
            match set.graph_type {
                GraphType::Scatter => {
                    for &point in &points {
                        Self::point(&mut layer, window, set.marker, point, series);
                    }
                }
                GraphType::Line => {
                    for pair in points.windows(2) {
                        Self::segment(&mut layer, window, set.marker, pair[0], pair[1], series);
                    }
                    if let [only] = points[..] {
                        Self::point(&mut layer, window, set.marker, only, series);
                    }
                }
                GraphType::Bar => {
                    let (width, height) =
                        (i64::from(layer.rect.width), i64::from(layer.rect.height));
                    for &(x, y) in points.iter().filter(|&&point| window.contains(point)) {
                        let column = window.column(x, width);
                        let ticks = window.ticks(y, height * 8);
                        layer.eighths(u16::try_from(column).unwrap_or(0), ticks, series);
                    }
                }
            }
            plot.overlay(&layer);
        }
    }

    fn point(plot: &mut Plot, window: Window, marker: Marker, (x, y): (f64, f64), series: usize) {
        if !window.contains((x, y)) {
            return;
        }
        let (sub_w, sub_h) = plot.resolution(marker);
        plot.mark(
            marker,
            window.column(x, sub_w),
            window.row(y, sub_h),
            series,
        );
    }

    fn segment(
        plot: &mut Plot,
        window: Window,
        marker: Marker,
        a: (f64, f64),
        b: (f64, f64),
        series: usize,
    ) {
        let Some((a, b)) = window.clip(a, b) else {
            return;
        };
        let (sub_w, sub_h) = plot.resolution(marker);
        let (x0, y0) = (window.column(a.0, sub_w), window.row(a.1, sub_h));
        let (x1, y1) = (window.column(b.0, sub_w), window.row(b.1, sub_h));
        let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
        if dy < dx {
            let ((x0, y0), (x1, y1)) = if x0 > x1 {
                ((x1, y1), (x0, y0))
            } else {
                ((x0, y0), (x1, y1))
            };
            let step = if y1 < y0 { -1 } else { 1 };
            let (mut y, mut error) = (y0, 2 * dy - dx);
            for x in x0..=x1 {
                plot.mark(marker, x, y, series);
                if error > 0 {
                    y += step;
                    error -= 2 * dx;
                }
                error += 2 * dy;
            }
        } else {
            let ((x0, y0), (x1, y1)) = if y0 > y1 {
                ((x1, y1), (x0, y0))
            } else {
                ((x0, y0), (x1, y1))
            };
            let step = if x1 < x0 { -1 } else { 1 };
            let (mut x, mut error) = (x0, 2 * dx - dy);
            for y in y0..=y1 {
                plot.mark(marker, x, y, series);
                if error > 0 {
                    x += step;
                    error -= 2 * dy;
                }
                error += 2 * dx;
            }
        }
    }

    fn draw_plot(&self, plot: &Plot, cx: &mut RenderCx<'_>) {
        for row in 0..plot.rect.height {
            for column in 0..plot.rect.width {
                let index = usize::from(row) * usize::from(plot.rect.width) + usize::from(column);
                let spot = plot.spots[index];
                if spot.ink == Ink::Empty {
                    continue;
                }
                let mut buffer = [0_u8; 4];
                let text = glyph(spot).encode_utf8(&mut buffer);
                put_text(
                    cx,
                    plot.rect.x.saturating_add(column),
                    plot.rect.y.saturating_add(row),
                    text,
                    self.series_style(cx, spot.series),
                );
            }
        }
    }

    fn draw_legend(&self, plot: Rect, look: &Look, cx: &mut RenderCx<'_>) {
        let Some(position) = self.legend else {
            return;
        };
        let entries: Vec<(usize, &str)> = self
            .datasets
            .iter()
            .enumerate()
            .filter_map(|(index, set)| set.name.map(|name| (index, name)))
            .collect();
        if entries.is_empty() {
            return;
        }
        let inner = entries
            .iter()
            .map(|(_, name)| label_width(name))
            .max()
            .unwrap_or(0)
            + 4;
        let width = inner + 2;
        let height = u16::try_from(entries.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2);
        if width > resolve(self.hidden.0, plot.width)
            || height > resolve(self.hidden.1, plot.height)
            || width > plot.width
            || height > plot.height
        {
            return;
        }
        let (left, middle) = (plot.x, plot.x.saturating_add((plot.width - width) / 2));
        let right = plot.x.saturating_add(plot.width).saturating_sub(width);
        let (top, center) = (plot.y, plot.y.saturating_add((plot.height - height) / 2));
        let bottom = plot.y.saturating_add(plot.height).saturating_sub(height);
        let (x, y) = match position {
            LegendPosition::Top => (middle, top),
            LegendPosition::TopRight => (right, top),
            LegendPosition::TopLeft => (left, top),
            LegendPosition::Left => (left, center),
            LegendPosition::Right => (right, center),
            LegendPosition::Bottom => (middle, bottom),
            LegendPosition::BottomRight => (right, bottom),
            LegendPosition::BottomLeft => (left, bottom),
        };
        let area = Rect::new(x, y, width, height);
        let blank = " ".repeat(usize::from(width));
        let panel = Style::new().fg(look.text.fg).bg(look.legend_bg);
        for row in 0..height {
            cx.put_str(x, y.saturating_add(row), &blank, panel);
        }
        Block::bordered()
            .border_type(BorderType::Rounded)
            .style(panel)
            .border_style(look.axis.bg(look.legend_bg))
            .render(area, cx);
        for (row, (index, name)) in (0_u16..).zip(&entries) {
            let swatch = self.series_style(cx, *index).bg(look.legend_bg);
            let label_y = y.saturating_add(1).saturating_add(row);
            put_text(cx, x.saturating_add(2), label_y, "■", swatch);
            put_text(
                cx,
                x.saturating_add(4),
                label_y,
                name,
                look.text.bg(look.legend_bg),
            );
        }
    }
}

impl Widget for Chart<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_area_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.clear_area(area, self.background.unwrap_or_default());
        let y_labels = self
            .y_axis
            .labels
            .iter()
            .map(|label| label_width(label))
            .max()
            .unwrap_or(0);
        let top = u16::from(self.y_axis.title.is_some());
        let bottom =
            1 + u16::from(!self.x_axis.labels.is_empty()) + u16::from(self.x_axis.title.is_some());
        let first_x = self
            .x_axis
            .labels
            .first()
            .map_or(0, |label| label_width(label).saturating_sub(1));
        let left = y_labels.max(first_x).min(area.width.saturating_sub(2)) + 1;
        if area.width <= left || area.height <= top + bottom {
            return;
        }
        let rect = Rect::new(
            area.x.saturating_add(left),
            area.y.saturating_add(top),
            area.width - left,
            area.height - top - bottom,
        );
        let look = self.look(cx);
        self.draw_axes(area, rect, &look, cx);
        let mut plot = Plot::new(rect);
        self.rasterize(&mut plot);
        self.draw_plot(&plot, cx);
        self.draw_legend(rect, &look, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, avail.height).clamp(avail)
    }
}
