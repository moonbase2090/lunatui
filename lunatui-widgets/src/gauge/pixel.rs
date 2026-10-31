// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, CellWidth, DecodedGlyph, Meta, Rect, Style};
use lunatui_image::{ImageSource, PixelSize, PixelTarget};
use tiny_skia::{Paint, Pixmap, Transform};

use super::Gauge;
use crate::RenderCx;

const MAX_WORK: u64 = 32 * 1024 * 1024;

struct EraseInk {
    x: u16,
    y: u16,
    style: Style,
}

struct Prepared {
    source: ImageSource,
    erase: Vec<EraseInk>,
}

fn supported(style: Style) -> bool {
    style.attrs.to_bits() & !Attrs::new().set_dim(true).to_bits() == 0
}

pub(super) fn render(
    gauge: &Gauge,
    area: Rect,
    whole: u16,
    partial: Option<char>,
    cx: &mut RenderCx<'_>,
) {
    let Some(prepared) = prepare(gauge, area, whole, partial, cx) else {
        return;
    };
    if cx.submit_raster(&prepared.source, area, 0) {
        for ink in prepared.erase {
            cx.put_char_patched(ink.x, ink.y, ' ', ink.style);
        }
    }
}

fn prepare(
    gauge: &Gauge,
    area: Rect,
    whole: u16,
    partial: Option<char>,
    cx: &RenderCx<'_>,
) -> Option<Prepared> {
    let target = target(gauge, area, cx)?;
    let size = target.size();
    let mut foreground = cx.pixel_rgb(gauge.fill_style.fg, None)?;
    if gauge.fill_style.attrs.dim() {
        for channel in &mut foreground {
            *channel /= 2;
        }
    }
    let erase = verify_row(gauge, area, whole, partial, cx)?;
    let backgrounds = cx.pixel_backdrop(area, target.background(), true)?;
    let source = raster(size, area.width, &backgrounds, foreground, gauge.ratio)?;
    Some(Prepared { source, erase })
}

fn target(gauge: &Gauge, area: Rect, cx: &RenderCx<'_>) -> Option<PixelTarget> {
    if !gauge.ratio.is_finite()
        || gauge.ratio <= 0.0
        || !supported(gauge.fill_style)
        || area.width == 0
        || area.width > 4096
        || !cx.has_image_sink()
        || !cx.pixel_area_in_grid(area)
    {
        return None;
    }
    let target = cx.pixel_target(area)?;
    let size = target.size();
    let count = size.width().checked_mul(size.height())?;
    if size.width() > 4096
        || size.height() > 4096
        || count > 1_048_576
        || size.width() < u32::from(area.width)
    {
        return None;
    }
    let work = u64::from(size.width())
        .checked_add(2)?
        .checked_mul(u64::from(size.height()).checked_add(2)?)?
        .checked_mul(4)?
        .checked_add(u64::from(count).checked_mul(8)?)?
        .checked_add(u64::from(area.width).checked_mul(64)?)?;
    if work > MAX_WORK {
        return None;
    }
    Some(target)
}

fn verify_row(
    gauge: &Gauge,
    area: Rect,
    whole: u16,
    partial: Option<char>,
    cx: &RenderCx<'_>,
) -> Option<Vec<EraseInk>> {
    let mut erase = Vec::new();
    erase
        .try_reserve_exact(usize::from(whole) + usize::from(partial.is_some()))
        .ok()?;
    for offset in 0..area.width {
        let x = area.x.checked_add(offset)?;
        let glyph = match offset.cmp(&whole) {
            core::cmp::Ordering::Less => '█',
            core::cmp::Ordering::Equal => partial.unwrap_or(' '),
            core::cmp::Ordering::Greater => ' ',
        };
        let expected = if glyph == ' ' {
            gauge.track_style.unwrap_or_default()
        } else {
            gauge.fill_style
        };
        let cell = cx.pixel_cell(x, area.y)?;
        let normal = Meta {
            width: CellWidth::One,
            attrs: expected.attrs,
            ..Meta::default()
        };
        if cell.glyph().decode() != DecodedGlyph::Inline(glyph)
            || cell.style() != expected
            || cell.meta() != normal
            || !supported(expected)
        {
            return None;
        }
        if glyph != ' ' {
            erase.push(EraseInk {
                x,
                y: area.y,
                style: expected,
            });
        }
    }
    Some(erase)
}

fn raster(
    size: PixelSize,
    columns: u16,
    backgrounds: &[[u8; 3]],
    foreground: [u8; 3],
    ratio: f32,
) -> Option<ImageSource> {
    let mut pixels = Pixmap::new(size.width(), size.height())?;
    let width = usize::try_from(size.width()).ok()?;
    for (index, pixel) in pixels
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let column = index % width * usize::from(columns) / width;
        pixel[..3].copy_from_slice(&backgrounds[column]);
        pixel[3] = 255;
    }
    let rect = tiny_skia::Rect::from_xywh(
        0.0,
        0.0,
        ratio * f32::from(u16::try_from(size.width()).ok()?),
        f32::from(u16::try_from(size.height()).ok()?),
    )?;
    let mut paint = Paint::default();
    let [r, g, b] = foreground;
    paint.set_color_rgba8(r, g, b, 255);
    paint.anti_alias = true;
    pixels.fill_rect(rect, &paint, Transform::identity(), None);
    ImageSource::rgba8(
        size.width(),
        size.height(),
        pixels.take().into_boxed_slice(),
    )
    .ok()
}
