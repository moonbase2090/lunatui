// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Style};

use crate::RenderCx;
use crate::graphite::{Face, Rect as PixelRect};
use crate::pixel_cx::PixelCx;

const TEXT_SIZE: f32 = 0.56;

pub(super) struct Row<'t> {
    pub(super) selected: bool,
    pub(super) base: Style,
    pub(super) style: Style,
    pub(super) bar: Option<Style>,
    pub(super) icon: Option<(u16, &'t str, Style)>,
    pub(super) text: (u16, u16, &'t str),
    pub(super) mark: Option<(u16, u16, &'t str, Style)>,
}

fn pixels(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn face(style: Style) -> Face {
    if style.attrs.bold() {
        Face::SemiBold
    } else {
        Face::Regular
    }
}

fn baseline(px: &PixelCx<'_>, face: Face, size: f32, height: f32) -> f32 {
    px.line_metrics(face, size).map_or(height * 0.7, |line| {
        (height + line.ascent - line.descent) / 2.0
    })
}

fn selection(px: &mut PixelCx<'_>, row: &Row<'_>) {
    let cell = px.cell_size();
    let cell_h = usize::from(cell.height_px());
    let Ok(width) = usize::try_from(px.width()) else {
        return;
    };
    px.fill_rect(
        PixelRect {
            x: 0,
            y: 0,
            w: width,
            h: cell_h,
        },
        0.0,
        row.base.bg,
    );
    let inset = (cell_h / 16).max(1);
    let band = PixelRect {
        x: 0,
        y: inset,
        w: width,
        h: cell_h.saturating_sub(inset * 2).max(1),
    };
    px.fill_rect(band, pixels(band.h) * 0.25, row.style.bg);
    if let Some(bar) = row.bar {
        let thick = (cell_h * 3 / 28).max(2);
        px.fill_rect(
            PixelRect {
                x: 0,
                y: band.y,
                w: thick,
                h: band.h,
            },
            pixels(thick) / 2.0,
            bar.fg,
        );
    }
}

pub(super) fn row(cx: &mut RenderCx<'_>, area: Rect, row: &Row<'_>) {
    cx.replace_with_pixels(area, |px| {
        let cell = px.cell_size();
        let (cell_w, cell_h) = (f32::from(cell.width_px()), f32::from(cell.height_px()));
        let size = cell_h * TEXT_SIZE;
        if row.selected {
            selection(px, row);
        }
        if let Some((column, icon, style)) = row.icon {
            let face = face(style);
            let room = cell_w * f32::from(u16::try_from(lunatui_core::width(icon)).unwrap_or(1));
            let x = cell_w * f32::from(column) + (room - px.measure(face, size, icon)) / 2.0;
            let y = baseline(px, face, size, cell_h);
            px.text(face, size, (x, y), icon, style.fg);
        }
        let (from, to, text) = row.text;
        let face_text = face(row.style);
        let room = cell_w * f32::from(to.saturating_sub(from));
        let shown = px.fit(face_text, size, text, room).into_owned();
        let y = baseline(px, face_text, size, cell_h);
        px.text(
            face_text,
            size,
            (cell_w * f32::from(from), y),
            &shown,
            row.style.fg,
        );
        if let Some((from, right, mark, style)) = row.mark {
            let face = face(style);
            let room = cell_w * f32::from(right.saturating_sub(from));
            let shown = px.fit(face, size, mark, room).into_owned();
            let x = cell_w * f32::from(right) - px.measure(face, size, &shown);
            let y = baseline(px, face, size, cell_h);
            px.text(face, size, (x, y), &shown, style.fg);
        }
    });
}
