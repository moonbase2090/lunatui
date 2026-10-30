// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Style};
use lunatui_image::{ImageSource, PixelSize, PixelTarget};
use tiny_skia::{Paint, Pixmap, Transform};

use super::{
    BarDirection, BarPlacement, Ink, Layout, TextPlacement, Visitor, paint_bar_cells, visit_text,
};
use crate::RenderCx;
use crate::render_cx::patch_style;

const MAX_ITEMS: u64 = 65_536;
const MAX_TEXT: u64 = 1_048_576;
const MAX_WORK: u64 = 32 * 1024 * 1024;
const SUBPIXEL: u32 = 256;

struct Budget(u64);

impl Budget {
    fn spend(&mut self, amount: u64) -> Option<()> {
        self.0 = self.0.checked_sub(amount)?;
        Some(())
    }
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && u32::from(inner.x) + u32::from(inner.width)
            <= u32::from(outer.x) + u32::from(outer.width)
        && u32::from(inner.y) + u32::from(inner.height)
            <= u32::from(outer.y) + u32::from(outer.height)
}

fn intersects(area: Rect, x: u16, y: u16, width: usize) -> bool {
    y >= area.y
        && u32::from(y) < u32::from(area.y) + u32::from(area.height)
        && usize::from(x) < usize::from(area.x) + usize::from(area.width)
        && usize::from(x) + width > usize::from(area.x)
}

fn supported(style: Style) -> bool {
    style.attrs.to_bits() & !Attrs::new().set_dim(true).to_bits() == 0
}

fn input_work(layout: &Layout<'_, '_>) -> Option<Budget> {
    let mut items = 0_u64;
    let mut bytes = 0_u64;
    for (label, bars) in layout.chart.groups() {
        items = items
            .checked_add(1)?
            .checked_add(u64::try_from(bars.len()).ok()?)?;
        if items > MAX_ITEMS {
            return None;
        }
        bytes = bytes.checked_add(u64::try_from(label.map_or(0, str::len)).ok()?)?;
        for bar in bars {
            bytes = bytes
                .checked_add(u64::try_from(bar.label.map_or(0, str::len)).ok()?)?
                .checked_add(u64::try_from(bar.text_value.map_or(0, str::len)).ok()?)?;
            if bytes > MAX_TEXT {
                return None;
            }
        }
        if bytes > MAX_TEXT {
            return None;
        }
    }
    let mut budget = Budget(MAX_WORK);
    budget.spend(items.checked_add(bytes.checked_mul(4)?)?)?;
    Some(budget)
}

struct Preflight {
    chart: Rect,
    raster: Rect,
    clear: Style,
    budget: Budget,
    bars: usize,
    overlap: bool,
}

impl Visitor for Preflight {
    type Error = ();

    fn bar(&mut self, bar: BarPlacement) -> Result<(), ()> {
        if !contains(self.raster, bar.track) || !supported(patch_style(self.clear, bar.style)) {
            return Err(());
        }
        self.budget
            .spend(u64::from(bar.track.width) * u64::from(bar.track.height))
            .ok_or(())?;
        self.bars = self.bars.checked_add(1).ok_or(())?;
        Ok(())
    }

    fn text(&mut self, text: TextPlacement<'_>) -> Result<(), ()> {
        self.budget
            .spend(
                u64::try_from(text.text.len())
                    .map_err(|_| ())?
                    .checked_mul(8)
                    .ok_or(())?,
            )
            .ok_or(())?;
        visit_text(text, |x, y, _, style, width| {
            let width = u16::try_from(width).map_err(|_| ())?;
            if !(1..=2).contains(&width) || !contains(self.chart, Rect::new(x, y, width, 1)) {
                return Err(());
            }
            if intersects(self.raster, x, y, usize::from(width)) {
                if !supported(style) {
                    return Err(());
                }
                self.overlap = true;
            }
            Ok(())
        })
    }
}

struct Fill {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rgb: [u8; 3],
}

fn coordinate(cell: u16, origin: u16, cells: u16, pixels: u32) -> Option<u32> {
    let offset = u64::from(cell.checked_sub(origin)?);
    u32::try_from(
        offset
            .checked_mul(u64::from(pixels))?
            .checked_mul(u64::from(SUBPIXEL))?
            / u64::from(cells),
    )
    .ok()
}

fn scaled(value: u64, max: u64, extent: u32) -> Option<u32> {
    if max == 0 {
        return Some(0);
    }
    u32::try_from(u128::from(value.min(max)) * u128::from(extent) / u128::from(max)).ok()
}

fn physical(fixed: u32) -> Option<f32> {
    let whole = u16::try_from(fixed / SUBPIXEL).ok()?;
    let fraction = u8::try_from(fixed % SUBPIXEL).ok()?;
    Some(f32::from(whole) + f32::from(fraction) / 256.0)
}

impl Fill {
    fn new(bar: BarPlacement, area: Rect, size: PixelSize, rgb: [u8; 3]) -> Option<Self> {
        let x = coordinate(bar.track.x, area.x, area.width, size.width())?;
        let y = coordinate(bar.track.y, area.y, area.height, size.height())?;
        let right = coordinate(
            bar.track.x.checked_add(bar.track.width)?,
            area.x,
            area.width,
            size.width(),
        )?;
        let bottom = coordinate(
            bar.track.y.checked_add(bar.track.height)?,
            area.y,
            area.height,
            size.height(),
        )?;
        match bar.direction {
            BarDirection::Vertical => {
                let height = scaled(bar.value, bar.max, bottom.checked_sub(y)?)?;
                Some(Self {
                    x,
                    y: bottom.checked_sub(height)?,
                    width: right.checked_sub(x)?,
                    height,
                    rgb,
                })
            }
            BarDirection::Horizontal => {
                let width = scaled(bar.value, bar.max, right.checked_sub(x)?)?;
                Some(Self {
                    x,
                    y,
                    width,
                    height: bottom.checked_sub(y)?,
                    rgb,
                })
            }
        }
    }

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
        if self.width == 0 || self.height == 0 {
            return Some(());
        }
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

struct Native<'cx, 'frame> {
    cx: &'cx mut RenderCx<'frame>,
    area: Rect,
    target: PixelTarget,
    clear: Style,
    budget: Budget,
    fills: Vec<Fill>,
    text: Vec<bool>,
    overlap: bool,
}

impl Visitor for Native<'_, '_> {
    type Error = ();

    fn bar(&mut self, bar: BarPlacement) -> Result<(), ()> {
        self.budget
            .spend(u64::from(bar.track.width) * u64::from(bar.track.height))
            .ok_or(())?;
        let style = patch_style(self.clear, bar.style);
        if !supported(style) {
            return Err(());
        }
        if bar.value > 0 && bar.max > 0 {
            let mut rgb = self.cx.pixel_rgb(style.fg, None).ok_or(())?;
            if style.attrs.dim() {
                for channel in &mut rgb {
                    *channel /= 2;
                }
            }
            let fill = Fill::new(bar, self.area, self.target.size(), rgb).ok_or(())?;
            self.budget.spend(fill.work().ok_or(())?).ok_or(())?;
            self.fills.push(fill);
        }
        paint_bar_cells(bar, self.cx, Ink::Spaces);
        Ok(())
    }

    fn text(&mut self, text: TextPlacement<'_>) -> Result<(), ()> {
        self.budget
            .spend(
                u64::try_from(text.text.len())
                    .map_err(|_| ())?
                    .checked_mul(8)
                    .ok_or(())?,
            )
            .ok_or(())?;
        visit_text(text, |x, y, glyph, style, width| {
            self.cx.put_char_patched(x, y, glyph, style);
            for offset in 0..width {
                let Some(column) = usize::from(x)
                    .checked_add(offset)
                    .and_then(|column| u16::try_from(column).ok())
                else {
                    return Err(());
                };
                if intersects(self.area, column, y, 1) {
                    let index = usize::from(y - self.area.y) * usize::from(self.area.width)
                        + usize::from(column - self.area.x);
                    self.text[index] = true;
                }
            }
            Ok(())
        })
    }
}

struct Prepared {
    source: ImageSource,
    area: Rect,
    depth: i32,
}

impl Native<'_, '_> {
    fn finish(mut self) -> Option<Prepared> {
        if self.fills.is_empty() {
            return None;
        }
        let size = self.target.size();
        let pixel_count = u64::from(size.width()) * u64::from(size.height());
        self.budget.spend(
            pixel_count
                .checked_mul(8)?
                .checked_add(u64::try_from(self.text.len()).ok()?.checked_mul(2)?)?,
        )?;
        let backgrounds = self.cx.pixel_backdrop(
            self.area,
            self.target.background(),
            self.target.text_underlay(),
        )?;
        let mut pixels = Pixmap::new(size.width(), size.height())?;
        backdrop(&mut pixels, self.area, &backgrounds, None);
        for fill in &self.fills {
            fill.paint(&mut pixels)?;
        }
        backdrop(&mut pixels, self.area, &backgrounds, Some(&self.text));
        let source = ImageSource::rgba8(
            size.width(),
            size.height(),
            pixels.take().into_boxed_slice(),
        )
        .ok()?;
        Some(Prepared {
            source,
            area: self.area,
            depth: if self.overlap { -1 } else { 0 },
        })
    }
}

fn backdrop(pixels: &mut Pixmap, area: Rect, backgrounds: &[[u8; 3]], mask: Option<&[bool]>) {
    let width = usize::try_from(pixels.width()).unwrap_or(0);
    let height = usize::try_from(pixels.height()).unwrap_or(0);
    for (index, pixel) in pixels
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let x = index % width * usize::from(area.width) / width;
        let y = index / width * usize::from(area.height) / height;
        let cell = y * usize::from(area.width) + x;
        if mask.is_some_and(|mask| !mask[cell]) {
            continue;
        }
        pixel[..3].copy_from_slice(&backgrounds[cell]);
        pixel[3] = 255;
    }
}

pub(super) fn render(layout: &Layout<'_, '_>, cx: &mut RenderCx<'_>) -> bool {
    let area = layout.plot.area();
    if area.width == 0 || area.height == 0 || !cx.pixel_area_in_grid(layout.area) {
        return false;
    }
    let Some(target) = cx.pixel_target(area) else {
        return false;
    };
    let size = target.size();
    if size.width() > 4096
        || size.height() > 4096
        || size.width() < u32::from(area.width)
        || size.height() < u32::from(area.height)
        || size
            .width()
            .checked_mul(size.height())
            .is_none_or(|count| count > 1_048_576)
        || !supported(layout.chart.background.unwrap_or_default())
    {
        return false;
    }
    let Some(budget) = input_work(layout) else {
        return false;
    };
    let mut preflight = Preflight {
        chart: layout.area,
        raster: area,
        clear: layout.chart.background.unwrap_or_default(),
        budget,
        bars: 0,
        overlap: false,
    };
    if layout.visit(&mut preflight).is_err() || (preflight.overlap && !target.text_underlay()) {
        return false;
    }
    let cells = usize::from(area.width) * usize::from(area.height);
    let mut fills = Vec::new();
    let mut text = Vec::new();
    if fills.try_reserve_exact(preflight.bars).is_err() || text.try_reserve_exact(cells).is_err() {
        return false;
    }
    text.resize(cells, false);
    let mut native = Native {
        cx,
        area,
        target,
        clear: preflight.clear,
        budget: preflight.budget,
        fills,
        text,
        overlap: preflight.overlap,
    };
    let prepared = if layout.visit(&mut native).is_ok() {
        native.finish()
    } else {
        drop(native);
        None
    };
    if let Some(prepared) = prepared
        && cx.submit_raster(&prepared.source, prepared.area, prepared.depth)
    {
        return true;
    }
    cx.clear_area(layout.area, layout.chart.background.unwrap_or_default());
    false
}
