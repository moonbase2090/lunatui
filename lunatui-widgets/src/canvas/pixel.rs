// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Rect, Style};
use lunatui_image::{ImageSource, PixelTarget};
use lunatui_theme::Token;
use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::{LabelPlacement, draw_labels};
use crate::raster::{Marker, Window, round_to_tick};
use crate::{RenderCx, WidgetState};

const MAX_WORK: u64 = 32 * 1024 * 1024;
const MAX_OPS: usize = 65_536;

pub(super) struct Native {
    cells: Rect,
    target: PixelTarget,
    ops: Option<Vec<Op>>,
    work: u64,
}

struct Op {
    style: usize,
    kind: Kind,
}

enum Kind {
    Dot { center: (f32, f32), radius: f32 },
    Fill(tiny_skia::Rect),
    Segment { from: (f32, f32), to: (f32, f32) },
    Circle(tiny_skia::Rect),
}

pub(super) struct Drawing<'a> {
    pub area: Rect,
    pub styles: &'a [Style],
    pub labels: &'a [LabelPlacement<'a>],
    pub background: Style,
    pub state: WidgetState,
}

impl Native {
    pub(super) fn new(cells: Rect, cx: &RenderCx<'_>) -> Option<Self> {
        let target = cx.pixel_target(cells)?;
        let size = target.size();
        let pixels = size.width().checked_mul(size.height())?;
        if size.width() > 4096 || size.height() > 4096 || pixels > 1_048_576 {
            return None;
        }
        Some(Self {
            cells,
            target,
            ops: Some(Vec::new()),
            work: u64::from(pixels) * 3,
        })
    }

    fn width(&self) -> f64 {
        f64::from(self.target.size().width())
    }
    fn height(&self) -> f64 {
        f64::from(self.target.size().height())
    }
    fn stroke(&self) -> f32 {
        scalar((self.width() / f64::from(self.cells.width) * 0.16).clamp(1.5, 3.0))
    }

    fn position(&self, window: Window, point: (f64, f64)) -> (f64, f64) {
        let (horizontal, vertical) = window.normalized(point);
        (
            horizontal * (self.width() - 1.0) + 0.5,
            (1.0 - vertical) * (self.height() - 1.0) + 0.5,
        )
    }

    fn push(&mut self, kind: Kind, style: usize, cost: Option<u64>) {
        let Some(ops) = self.ops.as_mut() else {
            return;
        };
        let Some(work) = cost
            .and_then(|cost| self.work.checked_add(cost))
            .filter(|work| *work <= MAX_WORK)
        else {
            self.ops = None;
            return;
        };
        if ops.len() == MAX_OPS {
            self.ops = None;
            return;
        }
        self.work = work;
        ops.push(Op { style, kind });
    }

    fn area_cost(&self, bounds: [f64; 4]) -> Option<u64> {
        let [left, top, right, bottom] = bounds;
        if !bounds.iter().all(|coordinate| coordinate.is_finite()) {
            return None;
        }
        let width = (right.min(self.width()) - left.max(0.0)).max(0.0).ceil();
        let height = (bottom.min(self.height()) - top.max(0.0)).max(0.0).ceil();
        u64::try_from(round_to_tick(width))
            .ok()?
            .checked_mul(u64::try_from(round_to_tick(height)).ok()?)
    }

    pub(super) fn point(
        &mut self,
        window: Window,
        marker: Marker,
        point: (f64, f64),
        style: usize,
    ) {
        if self.ops.is_none() || !window.contains(point) {
            return;
        }
        if !matches!(marker, Marker::Braille | Marker::Dot) {
            self.fill_point(window, marker, point, style);
            return;
        }
        let (horizontal, vertical) = self.position(window, point);
        let radius = (self.width() / f64::from(self.cells.width) * 0.18).clamp(1.5, 3.0);
        let cost = self
            .area_cost([
                horizontal - radius - 1.0,
                vertical - radius - 1.0,
                horizontal + radius + 1.0,
                vertical + radius + 1.0,
            ])
            .and_then(|cost| cost.checked_mul(4));
        self.push(
            Kind::Dot {
                center: (scalar(horizontal), scalar(vertical)),
                radius: scalar(radius),
            },
            style,
            cost,
        );
    }

    fn fill_point(&mut self, window: Window, marker: Marker, point: (f64, f64), style: usize) {
        let (columns, rows) = match marker {
            Marker::Block => (u32::from(self.cells.width), u32::from(self.cells.height)),
            Marker::HalfBlock => (
                u32::from(self.cells.width),
                u32::from(self.cells.height) * 2,
            ),
            Marker::Quadrant => (
                u32::from(self.cells.width) * 2,
                u32::from(self.cells.height) * 2,
            ),
            Marker::Sextant => (
                u32::from(self.cells.width) * 2,
                u32::from(self.cells.height) * 3,
            ),
            Marker::Dot | Marker::Braille => return,
        };
        let Ok(column) = u32::try_from(window.column(point.0, i64::from(columns))) else {
            return;
        };
        let Ok(row) = u32::try_from(window.row(point.1, i64::from(rows))) else {
            return;
        };
        let size = self.target.size();
        let left = column * size.width() / columns;
        let right = (column + 1) * size.width() / columns;
        let top = row * size.height() / rows;
        let bottom = (row + 1) * size.height() / rows;
        if left == right || top == bottom {
            self.ops = None;
            return;
        }
        if let Some(rect) = tiny_skia::Rect::from_xywh(
            scalar(f64::from(left)),
            scalar(f64::from(top)),
            scalar(f64::from(right - left)),
            scalar(f64::from(bottom - top)),
        ) {
            self.push(
                Kind::Fill(rect),
                style,
                Some(u64::from(right - left) * u64::from(bottom - top)),
            );
        }
    }

    pub(super) fn line(&mut self, window: Window, from: (f64, f64), to: (f64, f64), style: usize) {
        if self.ops.is_none() {
            return;
        }
        let Some((from, to)) = window.clip(from, to) else {
            return;
        };
        if from == to || (self.target.size().width() == 1 && self.target.size().height() == 1) {
            self.point(window, Marker::Braille, from, style);
            return;
        }
        let from = self.position(window, from);
        let to = self.position(window, to);
        let padding = f64::from(self.stroke()) / 2.0 + 1.0;
        let area = self.area_cost([
            from.0.min(to.0) - padding,
            from.1.min(to.1) - padding,
            from.0.max(to.0) + padding,
            from.1.max(to.1) + padding,
        ]);
        let strip = ((from.0 - to.0).abs() + (from.1 - to.1).abs() + 2.0 * padding + 1.0)
            * (2.0 * padding + 1.0);
        let cost = area
            .and_then(|area| Some(area.min(u64::try_from(round_to_tick(strip.ceil())).ok()?)))
            .and_then(|cost| cost.checked_mul(4));
        self.push(
            Kind::Segment {
                from: (scalar(from.0), scalar(from.1)),
                to: (scalar(to.0), scalar(to.1)),
            },
            style,
            cost,
        );
    }

    pub(super) fn circle(
        &mut self,
        window: Window,
        marker: Marker,
        center: (f64, f64),
        radius: f64,
        style: usize,
    ) {
        if self.ops.is_none()
            || ![center.0, center.1, radius]
                .iter()
                .all(|value| value.is_finite())
        {
            return;
        }
        if radius == 0.0 {
            self.point(window, marker, center, style);
            return;
        }
        let center = self.position(window, center);
        let horizontal = (radius / window.width() * (self.width() - 1.0)).abs();
        let vertical = (radius / window.height() * (self.height() - 1.0)).abs();
        let bounds = [
            center.0 - horizontal,
            center.1 - vertical,
            center.0 + horizontal,
            center.1 + vertical,
        ];
        if !bounds
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 32_768.0)
            || horizontal == 0.0
            || vertical == 0.0
        {
            self.ops = None;
            return;
        }
        let padding = f64::from(self.stroke()) / 2.0 + 1.0;
        let padded = [
            bounds[0] - padding,
            bounds[1] - padding,
            bounds[2] + padding,
            bounds[3] + padding,
        ];
        let Some(cost) = self.area_cost(padded) else {
            self.ops = None;
            return;
        };
        if cost == 0 {
            return;
        }
        let Some(rect) = tiny_skia::Rect::from_xywh(
            scalar(bounds[0]),
            scalar(bounds[1]),
            scalar(horizontal * 2.0),
            scalar(vertical * 2.0),
        ) else {
            self.ops = None;
            return;
        };
        self.push(Kind::Circle(rect), style, cost.checked_mul(4));
    }

    pub(super) fn draw(&self, drawing: &Drawing<'_>, cx: &mut RenderCx<'_>) -> bool {
        let Some(ops) = self.ops.as_ref().filter(|ops| !ops.is_empty()) else {
            return false;
        };
        let has_labels = drawing.labels.iter().any(|label| label.cells.width != 0);
        if has_labels && !self.target.text_underlay() {
            return false;
        }
        let Some(mask) = label_mask(drawing.area, drawing.labels, self.work) else {
            return false;
        };
        let colors: Option<Vec<_>> = ops
            .iter()
            .map(|op| primitive_color(drawing.styles[op.style], drawing.state, cx))
            .collect();
        let Some(colors) = colors else {
            return false;
        };
        draw_labels(drawing.labels, drawing.background, drawing.state, cx);
        let Some(backdrop) = cx.pixel_backdrop(
            drawing.area,
            self.target.background(),
            self.target.text_underlay(),
        ) else {
            return false;
        };
        let size = self.target.size();
        let Some(mut pixels) = Pixmap::new(size.width(), size.height()) else {
            return false;
        };
        backdrop_pixels(&mut pixels, drawing.area, &backdrop, None);
        for (op, rgb) in ops.iter().zip(colors) {
            paint(&mut pixels, &op.kind, rgb, self.stroke());
        }
        backdrop_pixels(&mut pixels, drawing.area, &backdrop, Some(&mask));
        let Ok(source) = ImageSource::rgba8(
            size.width(),
            size.height(),
            pixels.take().into_boxed_slice(),
        ) else {
            return false;
        };
        cx.submit_raster(&source, drawing.area, if has_labels { -1 } else { 0 })
    }
}

fn primitive_color(mut raw: Style, state: WidgetState, cx: &RenderCx<'_>) -> Option<[u8; 3]> {
    let supported = Attrs::new().set_dim(true).to_bits();
    if raw.bg != Color::Default || raw.attrs.to_bits() & !supported != 0 {
        return None;
    }
    if raw.fg == Color::Default {
        raw.fg = cx.token(Token::TextPrimary);
    }
    let mut rgb = cx.pixel_rgb(raw.fg, None)?;
    if raw.attrs.dim() || state == WidgetState::Disabled {
        for channel in &mut rgb {
            *channel /= 2;
        }
    }
    Some(rgb)
}

fn label_mask(cells: Rect, labels: &[LabelPlacement<'_>], work: u64) -> Option<Vec<bool>> {
    if labels.len() > MAX_OPS {
        return None;
    }
    let mut work = work;
    let mut mask = vec![false; usize::from(cells.width) * usize::from(cells.height)];
    for label in labels {
        work = work
            .checked_add(u64::from(label.cells.width))
            .filter(|work| *work <= MAX_WORK)?;
        let start = usize::from(label.cells.y.checked_sub(cells.y)?) * usize::from(cells.width)
            + usize::from(label.cells.x.checked_sub(cells.x)?);
        let end = start.checked_add(usize::from(label.cells.width))?;
        mask.get_mut(start..end)?.fill(true);
    }
    Some(mask)
}

fn backdrop_pixels(pixels: &mut Pixmap, cells: Rect, colors: &[[u8; 3]], mask: Option<&[bool]>) {
    let (width, height) = (
        usize::try_from(pixels.width()).unwrap_or(0),
        usize::try_from(pixels.height()).unwrap_or(0),
    );
    for (index, pixel) in pixels
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let column = index % width * usize::from(cells.width) / width;
        let row = index / width * usize::from(cells.height) / height;
        let cell = row * usize::from(cells.width) + column;
        if mask.is_some_and(|mask| !mask[cell]) {
            continue;
        }
        let rgb = colors[cell];
        pixel.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
}

fn paint(pixels: &mut Pixmap, kind: &Kind, rgb: [u8; 3], width: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], 255);
    paint.anti_alias = !matches!(kind, Kind::Fill(_));
    if let Kind::Fill(rect) = kind {
        pixels.fill_rect(*rect, &paint, Transform::identity(), None);
        return;
    }
    let mut path = PathBuilder::new();
    match kind {
        Kind::Dot { center, radius } => path.push_circle(center.0, center.1, *radius),
        Kind::Segment { from, to } => {
            path.move_to(from.0, from.1);
            path.line_to(to.0, to.1);
        }
        Kind::Circle(rect) => path.push_oval(*rect),
        Kind::Fill(_) => return,
    }
    if let Some(path) = path.finish() {
        if matches!(kind, Kind::Dot { .. }) {
            pixels.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        } else {
            pixels.stroke_path(
                &path,
                &paint,
                &Stroke {
                    width,
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite native coordinates and extents are bounded to 65536 pixels before path construction"
)]
fn scalar(value: f64) -> f32 {
    value as f32
}
