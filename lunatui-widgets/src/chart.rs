// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Rect, Size, Style, clip_display_prefix, text_display_width};
use lunatui_layout::Constraint;
use lunatui_theme::Token;

use crate::raster::{Ink, Marker, Raster, Window, glyph};
use crate::{Block, BorderType, RenderCx, Widget, WidgetState};

#[cfg(feature = "pixel-chart")]
mod pixel;

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

/// How a dataset's points are drawn.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphType {
    /// Each point alone.
    #[default]
    Scatter,
    /// Points joined by straight lines.
    Line,
    /// A bar from the bottom of the plot up to each point, using eighth blocks in cell mode.
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
    /// A scatter series over `data`, with a Braille cell fallback and a series theme color.
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

    /// Glyphs used by the cell fallback. Pixel plots use native shapes and smooth strokes.
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
/// With the default `pixel-chart` feature and a measured image presenter, plots and legend
/// swatches use antialiased pixels. Axes and labels remain text. [`pixels(false)`](Self::pixels)
/// selects the cell renderer. Missing metrics, rejected images, excessive work, unresolved
/// foregrounds, per-series backgrounds, or unsupported style attributes also use cells. DIM
/// is rendered at half intensity; other series attributes, including REVERSE, use cells.
/// iTerm2/Sixel use cells for an in-plot legend because they cannot put images below its text.
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
    pixels: bool,
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
            pixels: true,
        }
    }

    /// Uses antialiased pixels when supported (the default), or explicitly selects cells.
    #[must_use]
    pub const fn pixels(mut self, enabled: bool) -> Self {
        self.pixels = enabled;
        self
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
        let axis_x = plot.x.saturating_sub(1);
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
        let gutter = plot.x.saturating_sub(1).saturating_sub(area.x);
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
            let start = start.min(right.saturating_sub(width));
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
            put_fit(
                cx,
                right.saturating_sub(width),
                title_row,
                width,
                title,
                style,
            );
        }
    }

    fn rasterize(&self, plot: &mut Raster) {
        let window = Window {
            x: self.x_axis.bounds,
            y: self.y_axis.bounds,
        };
        if !window.usable() {
            return;
        }
        for (series, set) in self.datasets.iter().enumerate() {
            let mut layer = Raster::new(plot.rect);
            set.plot_into(&mut CellPlot {
                raster: &mut layer,
                window,
                marker: set.marker,
                series,
            });
            plot.overlay(&layer);
        }
    }

    fn draw_plot(&self, plot: &Raster, cx: &mut RenderCx<'_>) {
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

    fn legend_layout(&self, plot: Rect) -> Option<LegendLayout<'_>> {
        let position = self.legend?;
        let entries: Vec<(usize, &str)> = self
            .datasets
            .iter()
            .enumerate()
            .filter_map(|(index, set)| set.name.map(|name| (index, name)))
            .collect();
        if entries.is_empty() {
            return None;
        }
        let inner = entries
            .iter()
            .map(|(_, name)| label_width(name))
            .max()
            .unwrap_or(0)
            .saturating_add(4);
        let width = inner.saturating_add(2);
        let height = u16::try_from(entries.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2);
        if width > resolve(self.hidden.0, plot.width)
            || height > resolve(self.hidden.1, plot.height)
            || width > plot.width
            || height > plot.height
        {
            return None;
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
        Some(LegendLayout {
            area: Rect::new(x, y, width, height),
            entries,
        })
    }

    fn draw_legend(
        &self,
        layout: &LegendLayout<'_>,
        look: &Look,
        cx: &mut RenderCx<'_>,
        swatches: bool,
    ) {
        let Rect {
            x,
            y,
            width,
            height,
        } = layout.area;
        let blank = " ".repeat(usize::from(width));
        let panel = Style::new().fg(look.text.fg).bg(look.legend_bg);
        for row in 0..height {
            cx.put_str(x, y.saturating_add(row), &blank, panel);
        }
        Block::bordered()
            .border_type(BorderType::Rounded)
            .style(panel)
            .border_style(look.axis.bg(look.legend_bg))
            .render(layout.area, cx);
        for (row, (index, name)) in (0_u16..).zip(&layout.entries) {
            let swatch = self.series_style(cx, *index).bg(look.legend_bg);
            let label_y = y.saturating_add(1).saturating_add(row);
            if swatches {
                put_text(cx, x.saturating_add(2), label_y, "■", swatch);
            }
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
        let left = y_labels
            .max(first_x)
            .min(area.width.saturating_sub(2))
            .saturating_add(1);
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
        let legend = self.legend_layout(rect);
        #[cfg(feature = "pixel-chart")]
        if self.pixels && self.draw_pixels(rect, legend.as_ref(), &look, cx) {
            return;
        }
        let mut plot = Raster::new(rect);
        self.rasterize(&mut plot);
        self.draw_plot(&plot, cx);
        if let Some(legend) = legend {
            self.draw_legend(&legend, &look, cx, true);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, avail.height).clamp(avail)
    }
}

struct LegendLayout<'a> {
    area: Rect,
    entries: Vec<(usize, &'a str)>,
}

trait PlotTarget {
    fn point(&mut self, point: (f64, f64));
    fn segment(&mut self, from: (f64, f64), to: (f64, f64));
    fn bar(&mut self, point: (f64, f64));
}

impl Dataset<'_> {
    fn plot_into(&self, target: &mut impl PlotTarget) {
        let mut points = self
            .data
            .iter()
            .copied()
            .filter(|(x, y)| x.is_finite() && y.is_finite());
        match self.graph_type {
            GraphType::Scatter => {
                for point in points {
                    target.point(point);
                }
            }
            GraphType::Line => {
                let Some(mut before) = points.next() else {
                    return;
                };
                let mut paired = false;
                for after in points {
                    target.segment(before, after);
                    before = after;
                    paired = true;
                }
                if !paired {
                    target.point(before);
                }
            }
            GraphType::Bar => {
                for point in points {
                    target.bar(point);
                }
            }
        }
    }
}

struct CellPlot<'a> {
    raster: &'a mut Raster,
    window: Window,
    marker: Marker,
    series: usize,
}

impl PlotTarget for CellPlot<'_> {
    fn point(&mut self, point: (f64, f64)) {
        self.raster
            .point(self.window, self.marker, point, self.series);
    }

    fn segment(&mut self, from: (f64, f64), to: (f64, f64)) {
        self.raster
            .segment(self.window, self.marker, from, to, self.series);
    }

    fn bar(&mut self, (x, y): (f64, f64)) {
        if self.window.contains((x, y)) {
            let width = i64::from(self.raster.rect.width);
            let height = i64::from(self.raster.rect.height);
            let column = self.window.column(x, width);
            let ticks = self.window.ticks(y, height * 8);
            self.raster
                .eighths(u16::try_from(column).unwrap_or(0), ticks, self.series);
        }
    }
}
