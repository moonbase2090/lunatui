// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;
use lunatui_image::{ImageSource, PixelSize};
use tiny_skia::Pixmap;

use super::{EIGHTHS, ProgressBar, ProgressTone, amount};
use crate::{RenderCx, WidgetState};

const SUBPIXEL: u32 = 256;
const MAX_ITEMS: usize = 65_536;
const MAX_WORK: u64 = 32 * 1024 * 1024;

struct Band {
    left: u32,
    right: u32,
    rgb: [u8; 3],
}

pub(super) fn render(
    bar: &ProgressBar<'_>,
    track: Rect,
    spans: &[(u16, u16, Option<ProgressTone>)],
    cx: &mut RenderCx<'_>,
) -> bool {
    prepare(bar, track, spans, cx).is_some_and(|source| cx.submit_raster(&source, track, 0))
}

fn prepare(
    bar: &ProgressBar<'_>,
    track: Rect,
    spans: &[(u16, u16, Option<ProgressTone>)],
    cx: &RenderCx<'_>,
) -> Option<ImageSource> {
    let size = target(bar, track, cx)?;
    let bands = bands(bar, track.width, size.width(), spans, cx)?;
    raster(size, &bands)
}

fn target(bar: &ProgressBar<'_>, track: Rect, cx: &RenderCx<'_>) -> Option<PixelSize> {
    let cells = usize::from(track.width).checked_mul(usize::from(track.height))?;
    if track.width == 0
        || track.height == 0
        || track.width > 4096
        || track.height > 4096
        || cells > MAX_ITEMS
        || bar.segments.len() > MAX_ITEMS
        || !cx.pixel_area_in_grid(track)
    {
        return None;
    }
    let size = cx.pixel_target(track)?.size();
    let pixels = size.width().checked_mul(size.height())?;
    if size.width() > 4096
        || size.height() > 4096
        || pixels > 1_048_576
        || size.width() < u32::from(track.width)
        || size.height() < u32::from(track.height)
    {
        return None;
    }
    let segments = u64::try_from(bar.segments.len()).ok()?;
    let bands = if bar.sweep.is_some() { 3 } else { segments + 1 };
    let work = bands
        .checked_mul(u64::from(size.width()))?
        .checked_mul(8)?
        .checked_add(segments.checked_mul(64)?)?
        .checked_add(u64::try_from(cells).ok()?.checked_mul(8)?)?
        .checked_add(u64::from(pixels).checked_mul(8)?)?;
    (work <= MAX_WORK).then_some(size)
}

fn bands(
    bar: &ProgressBar<'_>,
    columns: u16,
    pixels: u32,
    spans: &[(u16, u16, Option<ProgressTone>)],
    cx: &RenderCx<'_>,
) -> Option<Vec<Band>> {
    let extent = pixels.checked_mul(SUBPIXEL)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(if bar.sweep.is_some() {
            3
        } else {
            bar.segments.len() + 1
        })
        .ok()?;
    let mut left = 0;
    if bar.sweep.is_some() {
        let &(start, end, tone) = spans.first()?;
        let total = u64::from(columns.checked_mul(EIGHTHS)?);
        let start = u32::try_from(u64::from(start) * u64::from(extent) / total).ok()?;
        let end = u32::try_from(u64::from(end) * u64::from(extent) / total).ok()?;
        append(&mut result, bar, 0, start, None, cx)?;
        append(&mut result, bar, start, end, tone, cx)?;
        left = end;
    } else {
        let mut cumulative = 0.0;
        for segment in &bar.segments {
            cumulative += amount(segment.amount);
            let right = edge(cumulative.min(1.0), extent);
            append(&mut result, bar, left, right, Some(segment.tone), cx)?;
            left = right;
            if left == extent {
                break;
            }
        }
    }
    append(&mut result, bar, left, extent, None, cx)?;
    Some(result)
}

fn append(
    bands: &mut Vec<Band>,
    bar: &ProgressBar<'_>,
    left: u32,
    right: u32,
    tone: Option<ProgressTone>,
    cx: &RenderCx<'_>,
) -> Option<()> {
    if left == right {
        return Some(());
    }
    let mut rgb = cx.pixel_rgb(bar.color(cx, tone), None)?;
    if matches!(bar.state, WidgetState::Disabled) {
        for channel in &mut rgb {
            *channel /= 2;
        }
    }
    bands.push(Band { left, right, rgb });
    Some(())
}

fn edge(fraction: f64, extent: u32) -> u32 {
    let mut low = 0;
    let mut high = extent;
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if f64::from(mid) / f64::from(extent) <= fraction {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

fn raster(size: PixelSize, bands: &[Band]) -> Option<ImageSource> {
    let width = usize::try_from(size.width()).ok()?;
    let mut coverage = Vec::new();
    coverage.try_reserve_exact(width).ok()?;
    coverage.resize(width, [0_u64; 4]);
    for band in bands {
        for x in band.left / SUBPIXEL..band.right.div_ceil(SUBPIXEL) {
            let left = band.left.max(x * SUBPIXEL);
            let right = band.right.min((x + 1) * SUBPIXEL);
            let weight = u64::from(right.checked_sub(left)?);
            let total = coverage.get_mut(usize::try_from(x).ok()?)?;
            for (sum, channel) in total[..3].iter_mut().zip(band.rgb) {
                *sum += u64::from(channel) * weight;
            }
            total[3] += weight;
        }
    }
    let mut strip = Vec::new();
    strip.try_reserve_exact(width.checked_mul(4)?).ok()?;
    for total in coverage {
        let weight = total[3];
        if weight != u64::from(SUBPIXEL) {
            return None;
        }
        for sum in &total[..3] {
            strip.push(u8::try_from((sum + weight / 2) / weight).ok()?);
        }
        strip.push(255);
    }
    let mut pixels = Pixmap::new(size.width(), size.height())?;
    for row in pixels.data_mut().chunks_exact_mut(strip.len()) {
        row.copy_from_slice(&strip);
    }
    ImageSource::rgba8(
        size.width(),
        size.height(),
        pixels.take().into_boxed_slice(),
    )
    .ok()
}
