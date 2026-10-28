// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix};
use lunatui_theme::Token;

use crate::raster::{Marker, Raster, Window, glyph};
use crate::{RenderCx, Widget, WidgetState};

/// A drawing a [`Canvas`] can paint: it plots points and lines through a [`Painter`] in the
/// canvas's own coordinates.
pub trait Shape {
    /// Paints this shape.
    fn draw(&self, painter: &mut Painter<'_>);
}

/// Plots into the current layer of a [`Canvas`] in the canvas's coordinates: points and lines
/// outside the bounds are dropped or clipped.
pub struct Painter<'p> {
    raster: &'p mut Raster,
    window: Window,
    marker: Marker,
    styles: &'p mut Vec<Style>,
}

impl Painter<'_> {
    fn tag(&mut self, style: Style) -> usize {
        if let Some(index) = self.styles.iter().position(|known| *known == style) {
            return index;
        }
        self.styles.push(style);
        self.styles.len() - 1
    }

    /// Plots one point.
    pub fn point(&mut self, x: f64, y: f64, style: Style) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        let tag = self.tag(style);
        self.raster.point(self.window, self.marker, (x, y), tag);
    }

    /// Plots a straight line, clipped to the bounds.
    pub fn line(&mut self, from: (f64, f64), to: (f64, f64), style: Style) {
        if ![from.0, from.1, to.0, to.1]
            .iter()
            .all(|value| value.is_finite())
        {
            return;
        }
        let tag = self.tag(style);
        if self.marker == Marker::Braille {
            self.raster
                .segment_centered(self.window, self.marker, from, to, tag);
        } else {
            self.raster.segment(self.window, self.marker, from, to, tag);
        }
    }

    /// Dots across and down the canvas, which depends on the marker.
    #[must_use]
    pub fn resolution(&self) -> (u32, u32) {
        let (width, height) = self.raster.resolution(self.marker);
        (
            u32::try_from(width).unwrap_or(u32::MAX),
            u32::try_from(height).unwrap_or(u32::MAX),
        )
    }
}

/// Separate points, each drawn alone.
#[derive(Clone, Copy, Debug)]
pub struct Points<'a> {
    /// The `(x, y)` coordinates.
    pub coords: &'a [(f64, f64)],
    /// Style of every point.
    pub style: Style,
}

impl Shape for Points<'_> {
    fn draw(&self, painter: &mut Painter<'_>) {
        for &(x, y) in self.coords {
            painter.point(x, y, self.style);
        }
    }
}

/// A straight line between two points.
#[derive(Clone, Copy, Debug)]
pub struct Line {
    /// Start `x`.
    pub x1: f64,
    /// Start `y`.
    pub y1: f64,
    /// End `x`.
    pub x2: f64,
    /// End `y`.
    pub y2: f64,
    /// Style of the line.
    pub style: Style,
}

impl Shape for Line {
    fn draw(&self, painter: &mut Painter<'_>) {
        painter.line((self.x1, self.y1), (self.x2, self.y2), self.style);
    }
}

/// An unfilled rectangle with its lower-left corner at `(x, y)`.
#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    /// Lower-left `x`.
    pub x: f64,
    /// Lower-left `y`.
    pub y: f64,
    /// Extent along `x`.
    pub width: f64,
    /// Extent along `y`.
    pub height: f64,
    /// Style of the outline.
    pub style: Style,
}

impl Shape for Rectangle {
    fn draw(&self, painter: &mut Painter<'_>) {
        let (left, bottom) = (self.x, self.y);
        let (right, top) = (self.x + self.width, self.y + self.height);
        painter.line((left, bottom), (left, top), self.style);
        painter.line((left, top), (right, top), self.style);
        painter.line((right, bottom), (right, top), self.style);
        painter.line((left, bottom), (right, bottom), self.style);
    }
}

/// An unfilled circle around `(x, y)`.
#[derive(Clone, Copy, Debug)]
pub struct Circle {
    /// Center `x`.
    pub x: f64,
    /// Center `y`.
    pub y: f64,
    /// Radius in canvas units, along `x` and `y` alike.
    pub radius: f64,
    /// Style of the outline.
    pub style: Style,
}

impl Shape for Circle {
    fn draw(&self, painter: &mut Painter<'_>) {
        let (width, height) = painter.resolution();
        let steps = 360_u32.max(width.max(height).saturating_mul(4));
        for step in 0..steps {
            let angle = f64::from(step) / f64::from(steps) * core::f64::consts::TAU;
            painter.point(
                self.radius.mul_add(angle.cos(), self.x),
                self.radius.mul_add(angle.sin(), self.y),
                self.style,
            );
        }
    }
}

struct Label {
    x: f64,
    y: f64,
    text: String,
    style: Style,
}

/// What a [`Canvas`] paint closure draws with: shapes, layers, and text labels.
pub struct Context<'c> {
    window: Window,
    marker: Marker,
    committed: Raster,
    current: Raster,
    styles: &'c mut Vec<Style>,
    labels: Vec<Label>,
}

impl Context<'_> {
    /// Draws `shape` into the current layer.
    pub fn draw<S: Shape + ?Sized>(&mut self, shape: &S) {
        let mut painter = Painter {
            raster: &mut self.current,
            window: self.window,
            marker: self.marker,
            styles: self.styles,
        };
        shape.draw(&mut painter);
    }

    /// Sets the marker for shapes drawn next. The canvas marker is used initially; switch to a
    /// block marker for filled regions, then back to braille for outlines and paths.
    pub fn marker(&mut self, marker: Marker) {
        self.marker = marker;
    }

    /// Starts a new layer. A cell a later layer draws in replaces what an earlier layer drew
    /// there, so later layers sit on top.
    pub fn layer(&mut self) {
        self.committed.overlay(&self.current);
        let rect = self.current.rect;
        self.current = Raster::new(rect);
    }

    /// Prints `text` with its first cell at `(x, y)`, over every layer.
    pub fn print(&mut self, x: f64, y: f64, text: impl Into<String>, style: Style) {
        self.labels.push(Label {
            x,
            y,
            text: text.into(),
            style,
        });
    }

    /// Range of `x` the canvas shows.
    #[must_use]
    pub const fn x_bounds(&self) -> [f64; 2] {
        self.window.x
    }

    /// Range of `y` the canvas shows.
    #[must_use]
    pub const fn y_bounds(&self) -> [f64; 2] {
        self.window.y
    }
}

/// A free-form drawing surface: shapes and text placed by coordinates, drawn with sub-cell
/// markers.
///
/// The paint closure runs on every render. It draws [`Shape`]s through its [`Context`], starts
/// new layers with [`Context::layer`], and prints labels with [`Context::print`]. `x_bounds` and
/// `y_bounds` say which range of coordinates the area shows; shapes are clipped to it. Every cell
/// of the area is rewritten, and a [`background`](Self::background) keeps a panel color.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect, Style};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::canvas::Line;
/// use lunatui_widgets::{Canvas, Marker, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(4, 2);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// Canvas::new(|ctx| {
///     ctx.draw(&Line { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0, style: Style::new() });
/// })
/// .x_bounds([0.0, 1.0])
/// .y_bounds([0.0, 1.0])
/// .marker(Marker::Block)
/// .render(Rect::new(0, 0, 4, 2), &mut cx);
/// assert_eq!(grid.row_display(0, &clusters), "  ██");
/// assert_eq!(grid.row_display(1, &clusters), "██  ");
/// ```
pub struct Canvas<F> {
    paint: F,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    marker: Marker,
    background: Option<Style>,
    state: WidgetState,
}

impl<F> Canvas<F>
where
    F: Fn(&mut Context<'_>),
{
    /// A canvas that calls `paint` on every render, showing `0.0` to `1.0` on both axes.
    #[must_use]
    pub const fn new(paint: F) -> Self {
        Self {
            paint,
            x_bounds: [0.0, 1.0],
            y_bounds: [0.0, 1.0],
            marker: Marker::Braille,
            background: None,
            state: WidgetState::Normal,
        }
    }

    /// Range of `x` the canvas shows. A range of zero width draws nothing; a reversed range flips
    /// the axis.
    #[must_use]
    pub const fn x_bounds(mut self, bounds: [f64; 2]) -> Self {
        self.x_bounds = bounds;
        self
    }

    /// Range of `y` the canvas shows. A range of zero height draws nothing; a reversed range
    /// flips the axis.
    #[must_use]
    pub const fn y_bounds(mut self, bounds: [f64; 2]) -> Self {
        self.y_bounds = bounds;
        self
    }

    /// Glyphs the drawing is made of; Braille by default.
    #[must_use]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// Style the area is reset to before anything is drawn, such as a panel background. Without
    /// one, cells are reset to the default style.
    #[must_use]
    pub const fn background(mut self, style: Style) -> Self {
        self.background = Some(style);
        self
    }

    /// Interaction state used for colors that are not set explicitly.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }
}

impl<F> Widget for Canvas<F>
where
    F: Fn(&mut Context<'_>),
{
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let background = self.background.unwrap_or_default();
        cx.clear_area(area, background);
        let window = Window {
            x: self.x_bounds,
            y: self.y_bounds,
        };
        if area.width.min(area.height) == 0 || !window.usable() {
            return;
        }
        let mut styles = Vec::new();
        let mut context = Context {
            window,
            marker: self.marker,
            committed: Raster::new(area),
            current: Raster::new(area),
            styles: &mut styles,
            labels: Vec::new(),
        };
        (self.paint)(&mut context);
        context.layer();
        let Context {
            committed, labels, ..
        } = context;
        let dim = self.state == WidgetState::Disabled;
        let ink = |style: Style, cx: &RenderCx<'_>| {
            let mut style = style;
            if style.fg == lunatui_core::Color::Default {
                style = style.fg(cx.token(Token::TextPrimary));
            }
            if dim {
                style = style.with_attrs(style.attrs.set_dim(true));
            }
            if background.bg != lunatui_core::Color::Default
                && style.bg == lunatui_core::Color::Default
            {
                style = style.bg(background.bg);
            }
            style
        };
        for row in 0..area.height {
            for column in 0..area.width {
                let index = usize::from(row) * usize::from(area.width) + usize::from(column);
                let spot = committed.spots[index];
                if spot.ink == crate::raster::Ink::Empty {
                    continue;
                }
                let mut buffer = [0_u8; 4];
                let text = glyph(spot).encode_utf8(&mut buffer);
                let style = ink(styles[spot.series], cx);
                cx.put_str(area.x + column, area.y + row, text, style);
            }
        }
        for label in &labels {
            put_label(cx, area, window, label, ink(label.style, cx));
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn put_label(cx: &mut RenderCx<'_>, area: Rect, window: Window, label: &Label, style: Style) {
    if !label.x.is_finite() || !label.y.is_finite() {
        return;
    }
    if !window.contains((label.x, label.y)) {
        return;
    }
    let column = crate::raster::round_to_tick(
        ((label.x - window.x[0]) * crate::raster::span(i64::from(area.width) - 1) / window.width())
            .floor(),
    );
    let row = crate::raster::round_to_tick(
        ((window.y[1] - label.y) * crate::raster::span(i64::from(area.height) - 1)
            / window.height())
        .floor(),
    );
    let (Ok(column), Ok(row)) = (u16::try_from(column), u16::try_from(row)) else {
        return;
    };
    let room = area.width - column;
    let text = clip_display_prefix(&label.text, room);
    if !text.is_empty() {
        cx.put_str(area.x + column, area.y + row, text, style);
    }
}
