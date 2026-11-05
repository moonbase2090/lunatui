// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::NodeGraphView;
use super::canvas::{Glyph, Role};
use crate::RenderCx;
use lunatui_core::{Attrs, Rect, Style};
use lunatui_image::{ImageSource, PixelTarget};
use tiny_skia::{FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

pub(super) fn draw(
    view: &NodeGraphView<'_>,
    area: Rect,
    scroll: (u16, u16),
    cx: &mut RenderCx<'_>,
) -> bool {
    if view.graph.placements.is_empty() {
        return false;
    }
    let Some(target) = cx.pixel_target(area) else {
        return false;
    };
    if !target.text_underlay() {
        return false;
    }
    let Some(source) = raster(view, area, scroll, target, cx) else {
        return false;
    };
    cx.submit_raster(&source, area, -1)
}

struct Raster {
    pixels: Pixmap,
    cell: (f32, f32),
    offset: (f32, f32),
    stroke: Stroke,
}

impl Raster {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.offset.0) * self.cell.0,
            (y - self.offset.1) * self.cell.1,
        )
    }

    fn fill_cell(&mut self, x: u16, y: u16, rgb: [u8; 3]) {
        if let Some(rect) = tiny_skia::Rect::from_xywh(
            f32::from(x) * self.cell.0,
            f32::from(y) * self.cell.1,
            self.cell.0,
            self.cell.1,
        ) {
            self.pixels
                .fill_rect(rect, &paint(rgb), Transform::identity(), None);
        }
    }

    fn node(&mut self, rect: Rect, border: [u8; 3], background: [u8; 3]) {
        let (x, y) = self.point(f32::from(rect.x) + 0.5, f32::from(rect.y) + 0.5);
        let (w, h) = (
            f32::from(rect.width - 1) * self.cell.0,
            f32::from(rect.height - 1) * self.cell.1,
        );
        let radius = (self.cell.0 * 0.6).min(w / 2.0).min(h / 2.0);
        let mut outline = PathBuilder::new();
        outline.move_to(x + radius, y);
        outline.line_to(x + w - radius, y);
        outline.quad_to(x + w, y, x + w, y + radius);
        outline.line_to(x + w, y + h - radius);
        outline.quad_to(x + w, y + h, x + w - radius, y + h);
        outline.line_to(x + radius, y + h);
        outline.quad_to(x, y + h, x, y + h - radius);
        outline.line_to(x, y + radius);
        outline.quad_to(x, y, x + radius, y);
        outline.close();
        if let Some(path) = outline.finish() {
            self.pixels.fill_path(
                &path,
                &paint(background),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            self.pixels.stroke_path(
                &path,
                &paint(border),
                &self.stroke,
                Transform::identity(),
                None,
            );
        }
    }

    fn edges(&mut self, view: &NodeGraphView<'_>, rgb: [u8; 3]) {
        let mut paths = PathBuilder::new();
        let width = usize::from(view.graph.size.width);
        if width == 0 {
            return;
        }
        for (index, cell) in view.graph.canvas.cells.iter().enumerate() {
            let Glyph::Char(glyph) = cell.glyph else {
                continue;
            };
            let mask = match cell.role {
                Role::Edge => arms(glyph),
                Role::Border(_) if glyph == '├' => 2,
                Role::Border(_) if glyph == '┤' => 8,
                _ => continue,
            };
            let x = f32::from(u16::try_from(index % width).unwrap_or(0));
            let y = f32::from(u16::try_from(index / width).unwrap_or(0));
            let center = self.point(x + 0.5, y + 0.5);
            for (bit, end) in [
                (1, (x + 0.5, y)),
                (2, (x + 1.0, y + 0.5)),
                (4, (x + 0.5, y + 1.0)),
                (8, (x, y + 0.5)),
            ] {
                if mask & bit == 0 {
                    continue;
                }
                let end = self.point(end.0, end.1);
                paths.move_to(center.0, center.1);
                paths.line_to(end.0, end.1);
            }
        }
        self.stroke_path(paths.finish(), rgb);
    }

    fn stroke_path(&mut self, path: Option<Path>, rgb: [u8; 3]) {
        if let Some(path) = path {
            self.pixels.stroke_path(
                &path,
                &paint(rgb),
                &self.stroke,
                Transform::identity(),
                None,
            );
        }
    }
}

fn raster(
    view: &NodeGraphView<'_>,
    area: Rect,
    scroll: (u16, u16),
    target: PixelTarget,
    cx: &RenderCx<'_>,
) -> Option<ImageSource> {
    let size = target.size();
    if size.width() > 4096
        || size.height() > 4096
        || u64::from(size.width()) * u64::from(size.height()) > 1_048_576
    {
        return None;
    }
    let width = f32::from(u16::try_from(size.width()).ok()?);
    let height = f32::from(u16::try_from(size.height()).ok()?);
    let mut r = Raster {
        pixels: Pixmap::new(size.width(), size.height())?,
        cell: (
            width / f32::from(area.width),
            height / f32::from(area.height),
        ),
        offset: (f32::from(scroll.0), f32::from(scroll.1)),
        stroke: Stroke {
            width: (width / f32::from(area.width) * 0.16).clamp(1.5, 3.0),
            line_cap: LineCap::Round,
            ..Stroke::default()
        },
    };
    let ground = view.style(Role::Ground, cx);
    let background = cx.pixel_rgb(ground.bg, target.background())?;
    r.pixels.fill(tiny_skia::Color::from_rgba8(
        background[0],
        background[1],
        background[2],
        255,
    ));
    let card = view.style(Role::Body, cx).bg;
    for (index, &rect) in view.graph.placements.iter().enumerate() {
        let style = view.style(Role::Border(index), cx);
        if style.bg != card {
            return None;
        }
        r.node(
            rect,
            color(style, target, cx)?,
            cx.pixel_rgb(card, target.background())?,
        );
    }
    r.edges(view, color(view.style(Role::Edge, cx), target, cx)?);
    for y in 0..area.height.min(view.graph.size.height) {
        for x in 0..area.width.min(view.graph.size.width) {
            let index = usize::from(y + scroll.1) * usize::from(view.graph.size.width)
                + usize::from(x + scroll.0);
            if let Some(cell) = view.graph.canvas.cells.get(index)
                && matches!(cell.role, Role::Title(_))
            {
                r.fill_cell(
                    x,
                    y,
                    cx.pixel_rgb(view.style(cell.role, cx).bg, target.background())?,
                );
            }
        }
    }
    ImageSource::rgba8(
        size.width(),
        size.height(),
        r.pixels.take().into_boxed_slice(),
    )
    .ok()
}

fn color(style: Style, target: PixelTarget, cx: &RenderCx<'_>) -> Option<[u8; 3]> {
    let supported = Attrs::new().set_bold(true).set_dim(true).to_bits();
    if style.attrs.to_bits() & !supported != 0 {
        return None;
    }
    let mut rgb = cx.pixel_rgb(style.fg, target.background())?;
    if style.attrs.dim() {
        for channel in &mut rgb {
            *channel /= 2;
        }
    }
    Some(rgb)
}

fn paint(rgb: [u8; 3]) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], 255);
    paint.anti_alias = true;
    paint
}

fn arms(glyph: char) -> u8 {
    match glyph {
        '│' => 5,
        '─' => 10,
        '╰' => 3,
        '╭' => 6,
        '╯' => 9,
        '╮' => 12,
        '├' => 7,
        '┴' => 11,
        '┤' => 13,
        '┬' => 14,
        '┼' => 15,
        _ => 0,
    }
}
