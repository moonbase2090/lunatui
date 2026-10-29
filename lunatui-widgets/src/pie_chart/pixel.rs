// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::cell::RefCell;
use std::f64::consts::{FRAC_PI_2, TAU};

use lunatui_core::{Rect, Size};
use lunatui_image::{Fit, ImageId, ImageRequest, ImageSource, PixelSize};
use tiny_skia::{FillRule, Mask, Path, PathBuilder, Transform};

use super::{Disc, EXPLODE, PieChart, Sectors, palette};
use crate::{Marker, RenderCx, Widget};

/// An antialiased pie or donut, using the frame's image presenter when available.
///
/// Enable `pixel-pie-chart` and attach an image sink to [`RenderCx`]. The chart uses
/// measured physical pixels and preserves the backgrounds in its cell rectangle.
/// Missing metrics, unsupported images, resource limits, or rejected requests use
/// Braille automatically. iTerm2/Sixel also use Braille for center/caption text or
/// existing text inside the disc rectangle, since they cannot place images below text.
/// Depth-zero images cover later overlapping widgets on those protocols; compose
/// panels beside the chart rather than over it. Kitty supports text above the chart.
///
/// Keep one instance per stable, unique image ID. Raster caching and revisions are
/// internal; update the specification through [`set_chart`](Self::set_chart).
#[derive(Debug)]
pub struct PixelPieChart {
    chart: PieChart,
    id: ImageId,
    cache: RefCell<Option<RasterCache>>,
}

#[derive(Debug, PartialEq)]
struct RasterKey {
    chart: PieChart,
    size: PixelSize,
    cells: Size,
    colors: Vec<[u8; 3]>,
    backdrop: Vec<[u8; 3]>,
}

#[derive(Debug)]
struct RasterCache {
    key: RasterKey,
    source: ImageSource,
    generation: u64,
}

impl PixelPieChart {
    /// Owns the pixel rendering of `chart` under a unique, stable image identity.
    #[must_use]
    pub const fn new(id: ImageId, chart: PieChart) -> Self {
        Self {
            chart,
            id,
            cache: RefCell::new(None),
        }
    }

    /// The chart specification, including layout and cell-center hit testing.
    #[must_use]
    pub const fn chart(&self) -> &PieChart {
        &self.chart
    }

    /// Replaces the specification; changed raster inputs invalidate the cache automatically.
    pub fn set_chart(&mut self, chart: PieChart) {
        self.chart = chart;
    }

    fn draw_pixels(&self, area: Rect, cx: &mut RenderCx<'_>) -> bool {
        let layout = self.chart.layout(area);
        let Some(target) = cx.pixel_target(layout.pie) else {
            return false;
        };
        let size = target.size();
        if size.width() > 4096
            || size.height() > 4096
            || size
                .width()
                .checked_mul(size.height())
                .is_none_or(|n| n > 1_048_576)
            || u64::from(size.width())
                * u64::from(size.height())
                * u64::try_from(self.chart.slices.len().saturating_add(1)).unwrap_or(u64::MAX)
                > 32 * 1024 * 1024
            || self.chart.slices.len() > 4096
            || !self.chart.total().is_finite()
        {
            return false;
        }
        let has_text = self.chart.center.as_ref().is_some_and(|s| !s.is_empty())
            || self.chart.caption.as_ref().is_some_and(|s| !s.is_empty());
        if has_text && !target.text_underlay() {
            return false;
        }
        let Some(backdrop) =
            cx.pixel_backdrop(layout.pie, target.background(), target.text_underlay())
        else {
            return false;
        };
        let chart_palette = palette(cx);
        let colors: Option<Vec<_>> = (0..=self.chart.slices.len())
            .map(|index| {
                cx.pixel_rgb(
                    self.chart.slice_color(cx, &chart_palette, index),
                    target.background(),
                )
            })
            .collect();
        let Some(colors) = colors else {
            return false;
        };
        let mut spec = self.chart.clone();
        spec.marker = Marker::Braille;
        let key = RasterKey {
            chart: spec,
            size,
            cells: Size::new(layout.pie.width, layout.pie.height),
            colors,
            backdrop,
        };
        let mut cache = self.cache.borrow_mut();
        if cache.as_ref().is_none_or(|cached| cached.key != key) {
            let Some(generation) = cache
                .as_ref()
                .map_or(Some(0), |old| old.generation.checked_add(1))
            else {
                return false;
            };
            let Some(source) = rasterize(&key, layout.pie) else {
                return false;
            };
            *cache = Some(RasterCache {
                key,
                source,
                generation,
            });
        }
        let Some(cached) = cache.as_ref() else {
            return false;
        };
        let source = cached.source.clone();
        let generation = cached.generation;
        drop(cache);
        let depth = if has_text { -1 } else { 0 };
        if !cx.submit_image(ImageRequest::new(
            self.id,
            generation,
            &source,
            layout.pie,
            Fit::Stretch,
            depth,
            255,
        )) {
            return false;
        }
        self.chart.draw_center(cx, layout.pie);
        cx.keep_backgrounds(layout.legend, |cx| {
            self.chart.draw_legend(cx, &chart_palette, layout.legend);
        });
        true
    }
}

impl Widget for PixelPieChart {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if !self.draw_pixels(area, cx) {
            self.chart.clone().marker(Marker::Braille).render(area, cx);
        }
    }
    fn measure(&self, avail: Size) -> Size {
        self.chart.measure(avail)
    }
}

fn rasterize(key: &RasterKey, pie: Rect) -> Option<ImageSource> {
    let mut disc = key.chart.disc(pie);
    disc.cx -= f64::from(pie.x);
    disc.cy -= f64::from(pie.y) * key.chart.aspect;
    let sectors = key.chart.sectors();
    let size = key.size;
    let count = usize::try_from(size.width().checked_mul(size.height())?).ok()?;
    let mut coverage = vec![[0_u32; 4]; count];
    let mut mask = Mask::new(size.width(), size.height())?;
    let transform = raster_transform(size, pie, key.chart.aspect);
    for index in 0..=key.chart.slices.len() {
        if sectors.pull.is_some_and(|pull| pull.0 == index) {
            continue;
        }
        let Some(path) = sector_path(disc, &sectors, index) else {
            continue;
        };
        mask.data_mut().fill(0);
        mask.fill_path(&path, FillRule::EvenOdd, true, transform);
        for (total, alpha) in coverage.iter_mut().zip(mask.data()) {
            for (sum, color) in total[..3].iter_mut().zip(key.colors[index]) {
                *sum += u32::from(color) * u32::from(*alpha);
            }
            total[3] += u32::from(*alpha);
        }
    }
    let mut pixels = Vec::with_capacity(count.checked_mul(4)?);
    let width = usize::try_from(size.width()).ok()?;
    let height = usize::try_from(size.height()).ok()?;
    for (index, total) in coverage.iter().enumerate() {
        let x = index % width * usize::from(pie.width) / width;
        let y = index / width * usize::from(pie.height) / height;
        let bg = key.backdrop[y * usize::from(pie.width) + x];
        let weight = total[3].max(255);
        for (sum, background) in total[..3].iter().zip(bg) {
            pixels.push(
                u8::try_from(
                    (sum + u32::from(background) * 255_u32.saturating_sub(total[3]) + weight / 2)
                        / weight,
                )
                .ok()?,
            );
        }
        pixels.push(255);
    }
    if let Some((index, dx, dy)) = sectors.pull {
        disc.cx += dx * disc.radius * EXPLODE;
        disc.cy += dy * disc.radius * EXPLODE;
        if let Some(path) = sector_path(disc, &sectors, index) {
            mask.data_mut().fill(0);
            mask.fill_path(&path, FillRule::EvenOdd, true, transform);
            for (pixel, alpha) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(mask.data()) {
                for (channel, color) in pixel[..3].iter_mut().zip(key.colors[index]) {
                    *channel = u8::try_from(
                        (u32::from(color) * u32::from(*alpha)
                            + u32::from(*channel) * (255 - u32::from(*alpha))
                            + 127)
                            / 255,
                    )
                    .ok()?;
                }
            }
        }
    }
    ImageSource::rgba8(size.width(), size.height(), pixels.into_boxed_slice()).ok()
}

fn raster_transform(size: PixelSize, pie: Rect, aspect: f64) -> Transform {
    Transform::from_scale(
        scalar(f64::from(size.width()) / f64::from(pie.width)),
        scalar(f64::from(size.height()) / (f64::from(pie.height) * aspect)),
    )
}

fn ray_angle(diamond: f64) -> f64 {
    let d = diamond.rem_euclid(4.0);
    let (right, up) = if d < 1.0 {
        (d, 1.0 - d)
    } else if d < 2.0 {
        (2.0 - d, 1.0 - d)
    } else if d < 3.0 {
        (2.0 - d, d - 3.0)
    } else {
        (d - 4.0, d - 3.0)
    };
    right.atan2(up).rem_euclid(TAU)
}

fn sector_path(disc: Disc, sectors: &Sectors, index: usize) -> Option<Path> {
    let begin = if index == 0 {
        0.0
    } else {
        *sectors.ends.get(index - 1)?
    };
    let end = sectors.ends.get(index).copied().unwrap_or(sectors.track);
    if end <= begin {
        return None;
    }
    let start = ray_angle(sectors.start + begin);
    let span = if end - begin >= 4.0 {
        TAU
    } else {
        (ray_angle(sectors.start + end) - start).rem_euclid(TAU)
    };
    let mut path = PathBuilder::new();
    let (x, y) = point(disc, disc.radius, start);
    path.move_to(x, y);
    arc(&mut path, disc, disc.radius, start, span);
    let (x, y) = point(disc, disc.hole, start + span);
    path.line_to(x, y);
    arc(&mut path, disc, disc.hole, start + span, -span);
    path.close();
    path.finish()
}

fn point(disc: Disc, radius: f64, angle: f64) -> (f32, f32) {
    (
        scalar(disc.cx + radius * angle.sin()),
        scalar(disc.cy - radius * angle.cos()),
    )
}

fn arc(path: &mut PathBuilder, disc: Disc, radius: f64, start: f64, span: f64) {
    let mut at = start;
    let mut remaining = span;
    while remaining.abs() > 1e-12 {
        let step = remaining.clamp(-FRAC_PI_2, FRAC_PI_2);
        let end = at + step;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let (x, y) = point(disc, radius, end);
        path.cubic_to(
            scalar(disc.cx + radius * (at.sin() + k * at.cos())),
            scalar(disc.cy + radius * (-at.cos() + k * at.sin())),
            scalar(disc.cx + radius * (end.sin() - k * end.cos())),
            scalar(disc.cy + radius * (-end.cos() - k * end.sin())),
            x,
            y,
        );
        at = end;
        remaining -= step;
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite chart coordinates are bounded by cell and raster dimensions"
)]
fn scalar(value: f64) -> f32 {
    value as f32
}
