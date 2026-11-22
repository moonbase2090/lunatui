// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use core::fmt;
use core::num::NonZeroU16;
use std::borrow::Cow;

use lunatui_core::{Color, Rect as CellRect};
use lunatui_image::{ImageSource, PixelSize, PixelTarget};
use tiny_skia::{FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};
use unicode_segmentation::UnicodeSegmentation;

use crate::RenderCx;
use crate::graphite::{CellSize, Face, Rect, TextMetrics};

const MAX_SIDE: u32 = 4096;
const MAX_PIXELS: u32 = 1_048_576;
const ELLIPSIS: &str = "…";
const KAPPA: f32 = 0.552_284_8;

/// A pixel surface over one cell area, handed to the closure of
/// [`RenderCx::paint_pixels`].
///
/// Coordinates are pixels from the top-left corner of the area. Colors are resolved through the
/// frame's theme, and a color that has no RGB value, such as the terminal default with no known
/// background, draws nothing. Each drawing call returns whether it drew.
#[non_exhaustive]
pub struct PixelCx<'a> {
    pixmap: &'a mut Pixmap,
    cell_size: CellSize,
    resolve: &'a dyn Fn(Color) -> Option<[u8; 3]>,
    metrics: &'a dyn TextMetrics,
    #[cfg(feature = "graphite-text")]
    text: &'a crate::graphite::BundledText,
}

impl fmt::Debug for PixelCx<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PixelCx")
            .field("width", &self.pixmap.width())
            .field("height", &self.pixmap.height())
            .field("cell_size", &self.cell_size)
            .finish_non_exhaustive()
    }
}

impl PixelCx<'_> {
    /// Width of the surface in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    /// Height of the surface in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Size of one cell in pixels.
    #[must_use]
    pub fn cell_size(&self) -> CellSize {
        self.cell_size
    }

    /// Fills `rect` with `color`, rounding its corners by `radius` pixels.
    pub fn fill_rect(&mut self, rect: Rect, radius: f32, color: Color) -> bool {
        let Some(path) = rect_f32(rect).and_then(|r| rounded(r, radius)) else {
            return false;
        };
        let Some(paint) = self.paint(color) else {
            return false;
        };
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        true
    }

    /// Strokes the inside edge of `rect` with a line `width` pixels wide, rounding its corners by
    /// `radius` pixels.
    pub fn stroke_rect(&mut self, rect: Rect, radius: f32, width: f32, color: Color) -> bool {
        if !(width.is_finite() && width > 0.0) {
            return false;
        }
        let Some([x, y, w, h]) = rect_f32(rect) else {
            return false;
        };
        let half = width / 2.0;
        let inset = [x + half, y + half, w - width, h - width];
        let Some(path) = rounded(inset, (radius - half).max(0.0)) else {
            return false;
        };
        let Some(paint) = self.paint(color) else {
            return false;
        };
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        true
    }

    /// Draws a straight line `width` pixels wide with square-cut ends.
    pub fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: Color) -> bool {
        if !(width.is_finite() && width > 0.0) {
            return false;
        }
        let mut builder = PathBuilder::new();
        builder.move_to(from.0, from.1);
        builder.line_to(to.0, to.1);
        let Some(path) = builder.finish() else {
            return false;
        };
        let Some(paint) = self.paint(color) else {
            return false;
        };
        let stroke = Stroke {
            width,
            line_cap: LineCap::Butt,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        true
    }

    /// Fills the area between the polyline through `points` and the horizontal line at
    /// `baseline`, as an area chart does.
    pub fn area(&mut self, points: &[(f32, f32)], baseline: f32, color: Color) -> bool {
        let (Some(first), Some(last)) = (points.first(), points.last()) else {
            return false;
        };
        if points.len() < 2 {
            return false;
        }
        let mut builder = PathBuilder::new();
        builder.move_to(first.0, baseline);
        for &(x, y) in points {
            builder.line_to(x, y);
        }
        builder.line_to(last.0, baseline);
        builder.close();
        let Some(path) = builder.finish() else {
            return false;
        };
        let Some(paint) = self.paint(color) else {
            return false;
        };
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        true
    }

    /// Width of `text` in pixels at `face` and `px`, from the frame's text metrics.
    #[must_use]
    pub fn measure(&self, face: Face, px: f32, text: &str) -> f32 {
        self.metrics.width(face, px, text)
    }

    /// `text` cut to fit `max_width` pixels, ending with an ellipsis when it was cut.
    ///
    /// Text is cut between grapheme clusters. When not even the ellipsis fits, the result is
    /// empty.
    #[must_use]
    pub fn fit<'t>(&self, face: Face, px: f32, text: &'t str, max_width: f32) -> Cow<'t, str> {
        if self.measure(face, px, text) <= max_width {
            return Cow::Borrowed(text);
        }
        let mut end = 0;
        for (index, grapheme) in text.grapheme_indices(true) {
            let next = index + grapheme.len();
            let candidate = [&text[..next], ELLIPSIS].concat();
            if self.measure(face, px, &candidate) > max_width {
                break;
            }
            end = next;
        }
        if end == 0 && self.measure(face, px, ELLIPSIS) > max_width {
            return Cow::Borrowed("");
        }
        Cow::Owned([&text[..end], ELLIPSIS].concat())
    }

    /// Draws `text` in `color` starting at `origin`, where `origin.0` is the left edge and
    /// `origin.1` is the baseline, with the bundled fonts.
    ///
    /// Returns `false` when `px` is not a positive size, the color has no RGB value, or the text
    /// has nothing visible.
    #[cfg(feature = "graphite-text")]
    pub fn text(
        &mut self,
        face: Face,
        px: f32,
        origin: (f32, f32),
        text: &str,
        color: Color,
    ) -> bool {
        let Some(rgb) = (self.resolve)(color) else {
            return false;
        };
        let pixmap = &mut *self.pixmap;
        self.text
            .draw(face, px, origin, text, |glyph| blend(pixmap, &glyph, rgb))
    }

    /// Ascent, descent and line gap of `face` at `px` in the bundled fonts, for placing a
    /// baseline. Returns `None` when `px` is not a positive size.
    #[cfg(feature = "graphite-text")]
    #[must_use]
    pub fn line_metrics(&self, face: Face, px: f32) -> Option<crate::graphite::LineMetrics> {
        self.text.line_metrics(face, px)
    }

    fn paint(&self, color: Color) -> Option<Paint<'static>> {
        let [r, g, b] = (self.resolve)(color)?;
        let mut paint = Paint::default();
        paint.set_color_rgba8(r, g, b, 255);
        paint.anti_alias = true;
        Some(paint)
    }
}

#[cfg(feature = "graphite-text")]
fn blend(pixmap: &mut Pixmap, glyph: &crate::graphite::Glyph<'_>, [red, green, blue]: [u8; 3]) {
    let (width, height) = (i64::from(pixmap.width()), i64::from(pixmap.height()));
    let data = pixmap.data_mut();
    for (row, line) in glyph.coverage.chunks_exact(glyph.width).enumerate() {
        let y = glyph.top + i64::try_from(row).unwrap_or(i64::MAX);
        if !(0..height).contains(&y) {
            continue;
        }
        for (column, &coverage) in line.iter().enumerate() {
            let x = glyph.left + i64::try_from(column).unwrap_or(i64::MAX);
            if coverage == 0 || !(0..width).contains(&x) {
                continue;
            }
            let Ok(start) = usize::try_from((y * width + x) * 4) else {
                continue;
            };
            let Some(pixel) = data.get_mut(start..start + 4) else {
                continue;
            };
            let (alpha, rest) = (u16::from(coverage), 255 - u16::from(coverage));
            for (channel, source) in pixel.iter_mut().zip([red, green, blue]) {
                let mixed = (u16::from(source) * alpha + u16::from(*channel) * rest + 127) / 255;
                *channel = u8::try_from(mixed).unwrap_or(u8::MAX);
            }
        }
    }
}

fn rect_f32(rect: Rect) -> Option<[f32; 4]> {
    let value = |pixels: usize| u16::try_from(pixels).ok().map(f32::from);
    let bounds = [
        value(rect.x)?,
        value(rect.y)?,
        value(rect.w)?,
        value(rect.h)?,
    ];
    (bounds[2] > 0.0 && bounds[3] > 0.0).then_some(bounds)
}

fn rounded([left, top, width, height]: [f32; 4], radius: f32) -> Option<Path> {
    if !(width > 0.0 && height > 0.0) {
        return None;
    }
    let corner = if radius.is_finite() {
        radius.clamp(0.0, width.min(height) / 2.0)
    } else {
        0.0
    };
    if corner == 0.0 {
        return Some(PathBuilder::from_rect(tiny_skia::Rect::from_xywh(
            left, top, width, height,
        )?));
    }
    let (right, bottom, handle) = (left + width, top + height, corner * KAPPA);
    let mut path = PathBuilder::new();
    path.move_to(left + corner, top);
    path.line_to(right - corner, top);
    path.cubic_to(
        right - corner + handle,
        top,
        right,
        top + corner - handle,
        right,
        top + corner,
    );
    path.line_to(right, bottom - corner);
    path.cubic_to(
        right,
        bottom - corner + handle,
        right - corner + handle,
        bottom,
        right - corner,
        bottom,
    );
    path.line_to(left + corner, bottom);
    path.cubic_to(
        left + corner - handle,
        bottom,
        left,
        bottom - corner + handle,
        left,
        bottom - corner,
    );
    path.line_to(left, top + corner);
    path.cubic_to(
        left,
        top + corner - handle,
        left + corner - handle,
        top,
        left + corner,
        top,
    );
    path.close();
    path.finish()
}

pub(crate) fn cell_size(area: CellRect, size: PixelSize) -> Option<CellSize> {
    let (columns, rows) = (u32::from(area.width), u32::from(area.height));
    if columns == 0
        || rows == 0
        || !size.width().is_multiple_of(columns)
        || !size.height().is_multiple_of(rows)
    {
        return None;
    }
    let width = NonZeroU16::new(u16::try_from(size.width() / columns).ok()?)?;
    let height = NonZeroU16::new(u16::try_from(size.height() / rows).ok()?)?;
    Some(CellSize::new(width, height))
}

pub(crate) fn paint(
    cx: &RenderCx<'_>,
    area: CellRect,
    target: PixelTarget,
    underlay: bool,
    metrics: &dyn TextMetrics,
    draw: impl FnOnce(&mut PixelCx<'_>),
) -> Option<ImageSource> {
    let size = target.size();
    if size.width() > MAX_SIDE
        || size.height() > MAX_SIDE
        || size.width().checked_mul(size.height())? > MAX_PIXELS
    {
        return None;
    }
    let cell = cell_size(area, size)?;
    let backdrop = cx.pixel_backdrop(area, target.background(), underlay)?;
    let mut pixmap = Pixmap::new(size.width(), size.height())?;
    let width = usize::try_from(size.width()).ok()?;
    let columns = usize::from(area.width);
    let (cell_w, cell_h) = (usize::from(cell.width_px()), usize::from(cell.height_px()));
    for (index, pixel) in pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let (column, row) = (index % width / cell_w, index / width / cell_h);
        let [red, green, blue] = *backdrop.get(row * columns + column)?;
        *pixel = [red, green, blue, 255];
    }
    let background = target.background();
    let resolve = |color: Color| cx.pixel_rgb(color, background);
    let mut surface = PixelCx {
        pixmap: &mut pixmap,
        cell_size: cell,
        resolve: &resolve,
        metrics,
        #[cfg(feature = "graphite-text")]
        text: cx.bundled_text(),
    };
    draw(&mut surface);
    ImageSource::rgba8(
        size.width(),
        size.height(),
        pixmap.take().into_boxed_slice(),
    )
    .ok()
}
