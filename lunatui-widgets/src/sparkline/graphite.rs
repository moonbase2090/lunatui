// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Color, Rect};
use lunatui_runtime::RenderMode;
use lunatui_theme::Token;

use super::{RenderDirection, Sparkline, SparklineValue};
use crate::RenderCx;
use crate::graphite::Rect as PixelRect;
use crate::render_cx::patch_style;

const BASELINE_OPACITY: f32 = 0.25;

fn pixels(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn scaled(value: u64, scale: u64, room: usize) -> usize {
    if scale == 0 || value == 0 {
        return 0;
    }
    let room_wide = u128::try_from(room).unwrap_or(u128::MAX);
    let height = (u128::from(value).saturating_mul(room_wide) / u128::from(scale)).min(room_wide);
    usize::try_from(height).unwrap_or(room).max(1)
}

pub(super) fn paint<T: SparklineValue>(
    chart: &Sparkline<'_, T>,
    area: Rect,
    limit: usize,
    cx: &mut RenderCx<'_>,
) -> bool {
    if cx.mode() != RenderMode::Graphite {
        return false;
    }
    let Ok(columns) = u16::try_from(limit) else {
        return false;
    };
    if columns == 0 || columns > area.width {
        return false;
    }
    let left = match chart.direction {
        RenderDirection::LeftToRight => area.x,
        RenderDirection::RightToLeft => area.x + (area.width - columns),
    };
    let (floor, scale) = chart.scale_ends();
    let bars: Vec<(usize, Option<u64>, Color)> = (0..limit)
        .map(|index| {
            let bar = chart.data[index].sparkline_bar();
            let column = match chart.direction {
                RenderDirection::LeftToRight => index,
                RenderDirection::RightToLeft => limit - 1 - index,
            };
            let style = patch_style(
                chart.column_style(cx, index + 1 == limit),
                bar.style.unwrap_or_default(),
            );
            (
                column,
                bar.sample().map(|value| value.saturating_sub(floor)),
                style.fg,
            )
        })
        .collect();
    let baseline = cx.token(Token::TextPrimary);
    cx.replace_with_pixels(Rect::new(left, area.y, columns, area.height), |px| {
        let cell = px.cell_size();
        let (cell_w, cell_h) = (usize::from(cell.width_px()), usize::from(cell.height_px()));
        let Ok(height) = usize::try_from(px.height()) else {
            return;
        };
        let Ok(width) = usize::try_from(px.width()) else {
            return;
        };
        let hairline = (cell_h / 16).max(1);
        let base = height.saturating_sub(hairline);
        let room = base.saturating_sub(cell_h / 8);
        let bar = (cell_w * 5 / 8).max(1);
        let inset = (cell_w - bar) / 2;
        px.fill_rect_with_opacity(
            PixelRect {
                x: 0,
                y: base,
                w: width,
                h: hairline,
            },
            0.0,
            baseline,
            BASELINE_OPACITY,
        );
        let reuse = px.is_uniform(PixelRect {
            x: 0,
            y: 0,
            w: width,
            h: base,
        });
        let mut drawn: Vec<(usize, Color, usize)> = Vec::new();
        for &(column, value, color) in &bars {
            let rise = value.map_or(0, |value| scaled(value, scale, room));
            if rise == 0 {
                continue;
            }
            let rect = PixelRect {
                x: column * cell_w + inset,
                y: base - rise,
                w: bar,
                h: rise,
            };
            let earlier = drawn
                .iter()
                .find(|&&(height, ink, _)| height == rise && ink == color);
            if let Some(&(_, _, x)) = earlier
                && px.copy_block(PixelRect { x, ..rect }, rect.x)
            {
                continue;
            }
            px.fill_rect(rect, pixels(bar) / 2.0, color);
            if reuse {
                drawn.push((rise, color, rect.x));
            }
        }
    })
}
