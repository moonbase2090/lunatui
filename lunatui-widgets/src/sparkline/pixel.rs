// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, CellWidth, Color, DecodedGlyph, Meta, Rect, Style};
use lunatui_image::{ImageSource, PixelSize};
use tiny_skia::{Paint, Pixmap, Transform};

use super::{NINE_LEVELS, RenderDirection, Sparkline, SparklineValue, first_char, scale_height};
use crate::RenderCx;

const MAX_CELLS: usize = 65_536;
const MAX_WORK: u64 = 32 * 1024 * 1024;
const SUBPIXEL: u32 = 256;

pub(super) struct Recording {
    area: Rect,
    columns: Vec<(u16, Option<u64>)>,
}

struct EraseInk {
    x: u16,
    y: u16,
    style: Style,
}

struct Fill {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rgb: [u8; 3],
}

fn edge(offset: u16, cells: u16, pixels: u32) -> Option<u32> {
    u32::try_from(u64::from(offset) * u64::from(pixels) * u64::from(SUBPIXEL) / u64::from(cells))
        .ok()
}

fn physical(fixed: u32) -> Option<f32> {
    let whole = u16::try_from(fixed / SUBPIXEL).ok()?;
    let fraction = u8::try_from(fixed % SUBPIXEL).ok()?;
    Some(f32::from(whole) + f32::from(fraction) / 256.0)
}

fn fraction(glyph: char) -> Option<u32> {
    Some(match glyph {
        ' ' => 0,
        '▁' => 1,
        '▂' => 2,
        '▃' => 3,
        '▄' => 4,
        '▅' => 5,
        '▆' => 6,
        '▇' => 7,
        '█' => 8,
        _ => return None,
    })
}

fn supported(style: Style) -> bool {
    let allowed = Attrs::new().set_bold(true).set_dim(true).to_bits();
    style.attrs.to_bits() & !allowed == 0
        && (!style.attrs.bold() || matches!(style.fg, Color::Rgb(..)))
}

fn verified_style(cx: &RenderCx<'_>, x: u16, y: u16, glyph: char) -> Option<Style> {
    let cell = cx.pixel_cell(x, y)?;
    let style = cell.style();
    let normal = Meta {
        width: CellWidth::One,
        attrs: style.attrs,
        ..Meta::default()
    };
    if cell.glyph().decode() != DecodedGlyph::Inline(glyph)
        || cell.meta() != normal
        || !supported(style)
    {
        return None;
    }
    Some(style)
}

impl Recording {
    pub(super) fn new(
        area: Rect,
        count: usize,
        direction: RenderDirection,
        cx: &RenderCx<'_>,
    ) -> Option<Self> {
        let width = u16::try_from(count).ok()?;
        if width == 0 || width > 4096 || area.height > 4096 || !cx.has_image_sink() {
            return None;
        }
        let x = match direction {
            RenderDirection::LeftToRight => area.x,
            RenderDirection::RightToLeft => area.x.checked_add(area.width)?.checked_sub(width)?,
        };
        let area = Rect::new(x, area.y, width, area.height);
        if usize::from(width) * usize::from(area.height) > MAX_CELLS || !cx.pixel_area_in_grid(area)
        {
            return None;
        }
        let mut columns = Vec::new();
        columns.try_reserve_exact(count).ok()?;
        Some(Self { area, columns })
    }

    pub(super) fn record(&mut self, x: u16, value: Option<u64>) {
        self.columns.push((x, value));
    }

    pub(super) fn finish<T: SparklineValue>(
        self,
        chart: &Sparkline<'_, T>,
        floor: u64,
        scale: u64,
        cx: &mut RenderCx<'_>,
    ) {
        let Some((source, erase)) = self.prepare(chart, floor, scale, cx) else {
            return;
        };
        if cx.submit_raster(&source, self.area, 0) {
            for cell in erase {
                cx.put_char_patched(cell.x, cell.y, ' ', cell.style);
            }
        }
    }

    fn prepare<T: SparklineValue>(
        &self,
        chart: &Sparkline<'_, T>,
        floor: u64,
        scale: u64,
        cx: &RenderCx<'_>,
    ) -> Option<(ImageSource, Vec<EraseInk>)> {
        let target = cx.pixel_target(self.area)?;
        let size = target.size();
        let count = size.width().checked_mul(size.height())?;
        if size.width() > 4096
            || size.height() > 4096
            || count > 1_048_576
            || size.width() < u32::from(self.area.width)
            || size.height() < u32::from(self.area.height)
        {
            return None;
        }
        let cells = usize::from(self.area.width) * usize::from(self.area.height);
        let mut work = u64::try_from(cells)
            .ok()?
            .checked_mul(64)?
            .checked_add(u64::from(count).checked_mul(8)?)?;
        if work > MAX_WORK {
            return None;
        }
        let backgrounds = cx.pixel_backdrop(self.area, target.background(), true)?;
        let mut erase = Vec::new();
        let mut fills = Vec::new();
        erase.try_reserve_exact(cells).ok()?;
        fills.try_reserve_exact(cells).ok()?;
        for &(x, sample) in &self.columns {
            let relative = x.checked_sub(self.area.x)?;
            let left = edge(relative, self.area.width, size.width())?;
            let right = edge(relative.checked_add(1)?, self.area.width, size.width())?;
            let value = sample.unwrap_or(0).saturating_sub(floor);
            let native_height = if scale == 0 {
                0
            } else {
                u32::try_from(
                    u128::from(value.min(scale)) * u128::from(size.height()) * u128::from(SUBPIXEL)
                        / u128::from(scale),
                )
                .ok()?
            };
            let top = size
                .height()
                .checked_mul(SUBPIXEL)?
                .checked_sub(native_height)?;
            let mut ticks = scale_height(value, scale, self.area.height);
            for row in (0..self.area.height).rev() {
                let glyph = first_char(if sample.is_none() {
                    chart.absent_symbol
                } else {
                    chart.bars.glyph(ticks)
                });
                ticks = ticks.saturating_sub(8);
                let level = fraction(glyph)?;
                let y = self.area.y.checked_add(row)?;
                let style = verified_style(cx, x, y, glyph)?;
                if glyph != ' ' {
                    erase.push(EraseInk { x, y, style });
                }
                let row_top = edge(row, self.area.height, size.height())?;
                let bottom = edge(row.checked_add(1)?, self.area.height, size.height())?;
                let fill_top = if chart.bars == NINE_LEVELS && sample.is_some() {
                    top.max(row_top).min(bottom)
                } else {
                    bottom.checked_sub((bottom - row_top).checked_mul(level)? / 8)?
                };
                if fill_top < bottom {
                    let mut rgb = cx.pixel_rgb(style.fg, None)?;
                    if style.attrs.dim() {
                        for channel in &mut rgb {
                            *channel /= 2;
                        }
                    }
                    let fill = Fill {
                        x: left,
                        y: fill_top,
                        width: right - left,
                        height: bottom - fill_top,
                        rgb,
                    };
                    work = work.checked_add(fill.work()?)?;
                    if work > MAX_WORK {
                        return None;
                    }
                    fills.push(fill);
                }
            }
        }
        if fills.is_empty() {
            return None;
        }
        let mut pixels = Pixmap::new(size.width(), size.height())?;
        backdrop(&mut pixels, self.area, size, &backgrounds);
        for fill in fills {
            fill.paint(&mut pixels)?;
        }
        let source = ImageSource::rgba8(
            size.width(),
            size.height(),
            pixels.take().into_boxed_slice(),
        )
        .ok()?;
        Some((source, erase))
    }
}

impl Fill {
    fn work(&self) -> Option<u64> {
        let width = self
            .x
            .checked_add(self.width)?
            .div_ceil(SUBPIXEL)
            .checked_sub(self.x / SUBPIXEL)?
            .checked_add(2)?;
        let height = self
            .y
            .checked_add(self.height)?
            .div_ceil(SUBPIXEL)
            .checked_sub(self.y / SUBPIXEL)?
            .checked_add(2)?;
        u64::from(width)
            .checked_mul(u64::from(height))?
            .checked_mul(4)
    }

    fn paint(&self, pixels: &mut Pixmap) -> Option<()> {
        let rect = tiny_skia::Rect::from_xywh(
            physical(self.x)?,
            physical(self.y)?,
            physical(self.width)?,
            physical(self.height)?,
        )?;
        let mut paint = Paint::default();
        let [r, g, b] = self.rgb;
        paint.set_color_rgba8(r, g, b, 255);
        paint.anti_alias = true;
        pixels.fill_rect(rect, &paint, Transform::identity(), None);
        Some(())
    }
}

fn backdrop(pixels: &mut Pixmap, area: Rect, size: PixelSize, colors: &[[u8; 3]]) {
    let width = usize::try_from(size.width()).unwrap_or(0);
    let height = usize::try_from(size.height()).unwrap_or(0);
    for (index, pixel) in pixels
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let x = index % width * usize::from(area.width) / width;
        let y = index / width * usize::from(area.height) / height;
        pixel[..3].copy_from_slice(&colors[y * usize::from(area.width) + x]);
        pixel[3] = 255;
    }
}
