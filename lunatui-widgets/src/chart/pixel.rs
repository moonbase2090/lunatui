// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Color, Rect, Style};
use lunatui_image::{ImageSource, PixelSize};
use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::{Chart, GraphType, LegendLayout, Look, PlotTarget};
use crate::RenderCx;
use crate::raster::Window;

impl Chart<'_> {
    pub(super) fn draw_pixels(
        &self,
        plot: Rect,
        legend: Option<&LegendLayout<'_>>,
        look: &Look,
        cx: &mut RenderCx<'_>,
    ) -> bool {
        let Some(target) = cx.pixel_target(plot) else {
            return false;
        };
        let size = target.size();
        if size.width() > 4096
            || size.height() > 4096
            || size
                .width()
                .checked_mul(size.height())
                .is_none_or(|n| n > 1_048_576)
            || self.datasets.len() > 256
            || work(self, plot, size).is_none_or(|cost| cost > 32 * 1024 * 1024)
            || (legend.is_some() && !target.text_underlay())
        {
            return false;
        }
        let window = Window {
            x: self.x_axis.bounds,
            y: self.y_axis.bounds,
        };
        if !window.usable() {
            return false;
        }
        let colors: Option<Vec<_>> = (0..self.datasets.len())
            .map(|index| color(self.series_style(cx, index), cx))
            .collect();
        let Some(colors) = colors else {
            return false;
        };
        if let Some(legend) = legend {
            self.draw_legend(legend, look, cx, false);
        }
        let Some(backdrop) = cx.pixel_backdrop(plot, target.background(), target.text_underlay())
        else {
            return false;
        };
        let Some(mut pixels) = Pixmap::new(size.width(), size.height()) else {
            return false;
        };
        background(&mut pixels, plot, &backdrop, None);
        for (set, rgb) in self.datasets.iter().zip(&colors) {
            let mut paths = PixelPlot {
                fills: PathBuilder::new(),
                strokes: PathBuilder::new(),
                window,
                size,
                cells: plot,
            };
            set.plot_into(&mut paths);
            paths.paint(&mut pixels, *rgb);
        }
        if let Some(legend) = legend {
            background(&mut pixels, plot, &backdrop, Some(legend.area));
            swatches(&mut pixels, plot, legend, &colors);
        }
        let Ok(source) = ImageSource::rgba8(
            size.width(),
            size.height(),
            pixels.take().into_boxed_slice(),
        ) else {
            return false;
        };
        cx.submit_raster(&source, plot, if legend.is_some() { -1 } else { 0 })
    }
}

fn work(chart: &Chart<'_>, plot: Rect, size: PixelSize) -> Option<u64> {
    let width = u64::from(size.width());
    let height = u64::from(size.height());
    let pixels = width.checked_mul(height)?;
    let mut total = pixels.checked_mul(3)?;
    let mut count = 0_u64;
    for set in chart.datasets {
        let points = u64::try_from(set.data.len()).ok()?;
        count = count.checked_add(points).filter(|value| *value <= 65_536)?;
        if points == 0 {
            continue;
        }
        let primitive = match set.graph_type {
            GraphType::Line if points > 1 => points
                .checked_sub(1)?
                .checked_mul(width.checked_add(height)?)?
                .checked_mul(12)?,
            GraphType::Line | GraphType::Scatter => points.checked_mul(64)?,
            GraphType::Bar => points
                .checked_mul(width.div_ceil(u64::from(plot.width)).checked_add(2)?)?
                .checked_mul(height.checked_add(2)?)?,
        };
        total = total.checked_add(pixels)?.checked_add(primitive)?;
    }
    Some(total)
}

fn color(style: Style, cx: &RenderCx<'_>) -> Option<[u8; 3]> {
    let supported = Attrs::new().set_dim(true).to_bits();
    if style.attrs.to_bits() & !supported != 0 || style.bg != Color::Default {
        return None;
    }
    let mut rgb = cx.pixel_rgb(style.fg, None)?;
    if style.attrs.dim() {
        for channel in &mut rgb {
            *channel /= 2;
        }
    }
    Some(rgb)
}

fn background(pixels: &mut Pixmap, plot: Rect, colors: &[[u8; 3]], only: Option<Rect>) {
    let width = usize::try_from(pixels.width()).unwrap_or(0);
    let height = usize::try_from(pixels.height()).unwrap_or(0);
    for (index, pixel) in pixels
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let x = index % width * usize::from(plot.width) / width;
        let y = index / width * usize::from(plot.height) / height;
        if let Some(only) = only {
            let column = usize::from(plot.x) + x;
            let row = usize::from(plot.y) + y;
            if column < usize::from(only.x)
                || column >= usize::from(only.x) + usize::from(only.width)
                || row < usize::from(only.y)
                || row >= usize::from(only.y) + usize::from(only.height)
            {
                continue;
            }
        }
        pixel[..3].copy_from_slice(&colors[y * usize::from(plot.width) + x]);
        pixel[3] = 255;
    }
}

fn swatches(pixels: &mut Pixmap, plot: Rect, legend: &LegendLayout<'_>, colors: &[[u8; 3]]) {
    let width = f64::from(pixels.width()) / f64::from(plot.width);
    let height = f64::from(pixels.height()) / f64::from(plot.height);
    let side = (width * 0.6).min(height * 0.6);
    for (row, (series, _)) in (0_u16..).zip(&legend.entries) {
        let x = f64::from(legend.area.x - plot.x + 2) * width + (width - side) / 2.0;
        let y = f64::from(legend.area.y - plot.y + 1 + row) * height + (height - side) / 2.0;
        if let Some(rect) =
            tiny_skia::Rect::from_xywh(scalar(x), scalar(y), scalar(side), scalar(side))
        {
            let mut paint = Paint::default();
            let [red, green, blue] = colors[*series];
            paint.set_color_rgba8(red, green, blue, 255);
            pixels.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

struct PixelPlot {
    fills: PathBuilder,
    strokes: PathBuilder,
    window: Window,
    size: PixelSize,
    cells: Rect,
}

impl PixelPlot {
    fn position(&self, point: (f64, f64)) -> Option<(f32, f32)> {
        let (x, y) = self.window.normalized(point);
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        Some((
            scalar(x.clamp(0.0, 1.0) * f64::from(self.size.width() - 1) + 0.5),
            scalar((1.0 - y.clamp(0.0, 1.0)) * f64::from(self.size.height() - 1) + 0.5),
        ))
    }

    fn paint(self, pixels: &mut Pixmap, [red, green, blue]: [u8; 3]) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(red, green, blue, 255);
        if let Some(path) = self.fills.finish() {
            pixels.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        if let Some(path) = self.strokes.finish() {
            let stroke = Stroke {
                width: scalar(
                    (f64::from(self.size.width()) / f64::from(self.cells.width) * 0.16)
                        .clamp(1.5, 3.0),
                ),
                line_cap: LineCap::Round,
                ..Stroke::default()
            };
            pixels.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

impl PlotTarget for PixelPlot {
    fn point(&mut self, point: (f64, f64)) {
        if self.window.contains(point)
            && let Some((x, y)) = self.position(point)
        {
            let radius =
                (f64::from(self.size.width()) / f64::from(self.cells.width) * 0.18).clamp(1.5, 3.0);
            self.fills.push_circle(x, y, scalar(radius));
        }
    }

    fn segment(&mut self, from: (f64, f64), to: (f64, f64)) {
        if let Some((from, to)) = self.window.clip(from, to)
            && let (Some((x0, y0)), Some((x1, y1))) = (self.position(from), self.position(to))
        {
            self.strokes.move_to(x0, y0);
            self.strokes.line_to(x1, y1);
        }
    }

    fn bar(&mut self, point: (f64, f64)) {
        if !self.window.contains(point) {
            return;
        }
        let (x, y) = self.window.normalized(point);
        let width = f64::from(self.size.width()) / f64::from(self.cells.width);
        let height = y.clamp(0.0, 1.0) * f64::from(self.size.height());
        let left = x.clamp(0.0, 1.0) * (f64::from(self.size.width()) - width);
        if let Some(rect) = tiny_skia::Rect::from_xywh(
            scalar(left),
            scalar(f64::from(self.size.height()) - height),
            scalar(width),
            scalar(height),
        ) {
            self.fills.push_rect(rect);
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite clipped coordinates and raster dimensions are bounded to 4096 pixels"
)]
fn scalar(value: f64) -> f32 {
    value as f32
}
