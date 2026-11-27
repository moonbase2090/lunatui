// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Style};
use lunatui_widgets::PixelCx;
use lunatui_widgets::graphite::{Face, Rect};

use crate::Marker;

const TEXT_SIZE: f32 = 0.56;
const KEY_TEXT_SIZE: f32 = 0.5;
const TINT: f32 = 0.2;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Rounds a finite pixel length, clamped to the surface limit, up to whole pixels"
)]
fn whole(px: f32) -> usize {
    if px.is_finite() {
        px.clamp(0.0, 4096.0).ceil() as usize
    } else {
        0
    }
}

fn pixels(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

struct Row {
    cell_w: f32,
    cell_h: f32,
    top: usize,
    height: usize,
}

impl Row {
    fn new(px: &PixelCx<'_>) -> Self {
        let cell = px.cell_size();
        let cell_h = usize::from(cell.height_px());
        let inset = cell_h.div_ceil(8);
        Self {
            cell_w: f32::from(cell.width_px()),
            cell_h: pixels(cell_h),
            top: inset,
            height: cell_h.saturating_sub(inset * 2).max(1),
        }
    }

    fn middle(&self) -> f32 {
        pixels(self.top) + pixels(self.height) / 2.0
    }

    fn baseline(&self, px: &PixelCx<'_>, face: Face, size: f32) -> f32 {
        px.line_metrics(face, size)
            .map_or(self.middle() + size / 3.0, |line| {
                self.middle() + (line.ascent - line.descent) / 2.0
            })
    }
}

pub(crate) fn chip(
    px: &mut PixelCx<'_>,
    text: &str,
    marker: Option<Marker>,
    ink: Color,
    fill: Color,
    tinted: bool,
    face: Face,
) {
    let row = Row::new(px);
    let size = row.cell_h * TEXT_SIZE;
    let pad = pixels(row.height) * 0.5;
    let mark = size * 0.62;
    let gap = size * 0.4;
    let lead = marker.map_or(0.0, |_| mark + gap);
    let width = pixels(usize::try_from(px.width()).unwrap_or(usize::MAX));
    let shown = px
        .fit(face, size, text, width - 2.0 * pad - lead)
        .into_owned();
    let content = lead + px.measure(face, size, &shown);
    let rect = Rect {
        x: 0,
        y: row.top,
        w: whole(width),
        h: row.height,
    };
    let radius = pixels(row.height) / 2.0;
    if tinted {
        px.fill_rect_with_opacity(rect, radius, fill, TINT);
    } else {
        px.fill_rect(rect, radius, fill);
    }
    let mut x = ((width - content) / 2.0).max(pad);
    if let Some(marker) = marker {
        draw_marker(px, marker, (x + mark / 2.0, row.middle()), mark, ink);
        x += lead;
    }
    px.text(face, size, (x, row.baseline(px, face, size)), &shown, ink);
}

fn draw_marker(px: &mut PixelCx<'_>, marker: Marker, (x, y): (f32, f32), size: f32, ink: Color) {
    let half = size / 2.0;
    let square = |side: f32| Rect {
        x: whole(x - side / 2.0),
        y: whole(y - side / 2.0),
        w: whole(side),
        h: whole(side),
    };
    match marker {
        Marker::Dot => {
            let edge = size * 0.8;
            px.fill_rect(square(edge), edge / 2.0, ink);
        }
        Marker::Ring => {
            let edge = size * 0.8;
            px.stroke_rect(square(edge), edge / 2.0, (size * 0.14).max(1.0), ink);
        }
        Marker::Diamond => {
            px.polygon(
                &[(x, y - half), (x + half, y), (x, y + half), (x - half, y)],
                ink,
            );
        }
        Marker::Cross => {
            let arm = half * 0.7;
            let width = (size * 0.16).max(1.0);
            px.line((x - arm, y - arm), (x + arm, y + arm), width, ink);
            px.line((x - arm, y + arm), (x + arm, y - arm), width, ink);
        }
        Marker::Square => {
            let edge = size * 0.7;
            px.fill_rect(square(edge), edge * 0.18, ink);
        }
        Marker::Bang => {
            let width = (size * 0.18).max(1.0);
            px.line((x, y - half), (x, y + half * 0.35), width, ink);
            let dot = Rect {
                x: whole(x - width / 2.0),
                y: whole(y + half * 0.62),
                w: whole(width),
                h: whole(width),
            };
            px.fill_rect(dot, 0.0, ink);
        }
    }
}

pub(crate) fn keycap(
    px: &mut PixelCx<'_>,
    key: &str,
    (column, columns): (u16, u16),
    style: Style,
    edge: Color,
) {
    let row = Row::new(px);
    let size = row.cell_h * KEY_TEXT_SIZE;
    let face = Face::SemiBold;
    let left = row.cell_w * f32::from(column);
    let slot = row.cell_w * f32::from(columns);
    let pad = pixels(row.height) * 0.32;
    let shown = px.fit(face, size, key, slot - 2.0 * pad).into_owned();
    let text = px.measure(face, size, &shown);
    let width = (text + 2.0 * pad).max(pixels(row.height)).min(slot);
    let rect = Rect {
        x: whole(left + (slot - width) / 2.0),
        y: row.top,
        w: whole(width),
        h: row.height,
    };
    let radius = pixels(row.height) * 0.22;
    px.fill_rect(rect, radius, style.bg);
    px.stroke_rect(rect, radius, (row.cell_h / 24.0).max(1.0), edge);
    let x = pixels(rect.x) + (pixels(rect.w) - text) / 2.0;
    px.text(
        face,
        size,
        (x, row.baseline(px, face, size)),
        &shown,
        style.fg,
    );
}
