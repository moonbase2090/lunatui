// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

use lunatui_core::{Color, Rect, Size, Style};

use crate::{RenderCx, Widget};

const TRAIL_DOTS: usize = 48;

/// A time-sampled lightcycle trail rasterized into Braille's 2×4 dot cells.
///
/// The widget is stateless: pass the same elapsed time to reproduce the same frame. Keep the
/// animation clock in the application and request redraws at the desired frame rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lightcycle {
    elapsed: Duration,
    period: Option<Duration>,
    head: Style,
    tail: Style,
}

impl Lightcycle {
    /// Creates a frame sampled at `elapsed`, advancing at up to 60 sub-cell steps per second.
    #[must_use]
    pub fn new(elapsed: Duration) -> Self {
        Self {
            elapsed,
            period: None,
            head: Style::new().fg(Color::Rgb(90, 255, 190)),
            tail: Style::new().fg(Color::Rgb(20, 80, 110)),
        }
    }

    /// Sets the cycle duration. A zero duration is treated as one nanosecond.
    #[must_use]
    pub const fn period(mut self, period: Duration) -> Self {
        self.period = Some(period);
        self
    }

    /// Sets the head and oldest-trail styles. Intermediate trail colors are interpolated in RGB.
    #[must_use]
    pub const fn trail(mut self, head: Style, tail: Style) -> Self {
        self.head = head;
        self.tail = tail;
        self
    }
}

impl Widget for Lightcycle {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let sub_width = usize::from(area.width) * 2;
        let sub_height = usize::from(area.height) * 4;
        let Some(path) = Octagon::new(sub_width, sub_height) else {
            return;
        };
        let period_ns = self
            .period
            .map_or_else(
                || {
                    u128::try_from(path.len)
                        .unwrap_or(u128::MAX)
                        .saturating_mul(1_000_000_000)
                        / 60
                },
                |period| period.as_nanos(),
            )
            .max(1);
        let head_step =
            usize::try_from(self.elapsed.as_nanos().saturating_mul(path.len as u128) / period_ns)
                .unwrap_or(usize::MAX)
                % path.len;

        let cell_count = usize::from(area.width) * usize::from(area.height);
        let mut masks = vec![0u8; cell_count];
        let mut ages = vec![u8::MAX; cell_count];
        for age in 0..TRAIL_DOTS.min(path.len) {
            let (sx, sy) = path.point((head_step + path.len - age % path.len) % path.len);
            let cell_x = sx / 2;
            let cell_y = sy / 4;
            let cell = cell_y * usize::from(area.width) + cell_x;
            let dot = braille_bit(sx % 2, sy % 4);
            masks[cell] |= dot;
            ages[cell] = ages[cell].min(u8::try_from(age).unwrap_or(u8::MAX));
        }

        for cell_y in 0..usize::from(area.height) {
            for cell_x in 0..usize::from(area.width) {
                let index = cell_y * usize::from(area.width) + cell_x;
                let mask = masks[index];
                if mask == 0 {
                    continue;
                }
                let age = usize::from(ages[index]);
                let progress = (age.min(TRAIL_DOTS - 1), TRAIL_DOTS - 1);
                let glyph = char::from_u32(0x2800 + u32::from(mask)).unwrap_or(' ');
                let x = area
                    .x
                    .saturating_add(u16::try_from(cell_x).unwrap_or(u16::MAX));
                let y = area
                    .y
                    .saturating_add(u16::try_from(cell_y).unwrap_or(u16::MAX));
                cx.put_str(
                    x,
                    y,
                    &glyph.to_string(),
                    interpolate_style(self.head, self.tail, progress),
                );
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(1, 1).clamp(avail)
    }
}

#[derive(Clone, Copy)]
struct Octagon {
    points: [(usize, usize); 8],
    lengths: [usize; 8],
    len: usize,
}

impl Octagon {
    fn new(width: usize, height: usize) -> Option<Self> {
        if width < 2 || height < 2 {
            return None;
        }
        let chamfer = (width.min(height) / 4)
            .max(1)
            .min((width - 1) / 2)
            .min((height - 1) / 2);
        let right = width - 1;
        let bottom = height - 1;
        let points = [
            (chamfer, 0),
            (right - chamfer, 0),
            (right, chamfer),
            (right, bottom - chamfer),
            (right - chamfer, bottom),
            (chamfer, bottom),
            (0, bottom - chamfer),
            (0, chamfer),
        ];
        let lengths = std::array::from_fn(|index| {
            let from = points[index];
            let to = points[(index + 1) % points.len()];
            from.0.abs_diff(to.0).max(from.1.abs_diff(to.1)).max(1)
        });
        let len = lengths.iter().sum();
        Some(Self {
            points,
            lengths,
            len,
        })
    }

    fn point(&self, mut step: usize) -> (usize, usize) {
        for segment in 0..self.lengths.len() {
            let length = self.lengths[segment];
            if step < length {
                let from = self.points[segment];
                let to = self.points[(segment + 1) % self.points.len()];
                return (
                    interpolate_coord(from.0, to.0, step, length),
                    interpolate_coord(from.1, to.1, step, length),
                );
            }
            step -= length;
        }
        self.points[0]
    }
}

fn interpolate_coord(from: usize, to: usize, step: usize, length: usize) -> usize {
    if to >= from {
        from + (to - from) * step / length
    } else {
        from - (from - to) * step / length
    }
}

fn braille_bit(x: usize, y: usize) -> u8 {
    const DOTS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
    DOTS[x][y]
}

fn interpolate_style(head: Style, tail: Style, progress: (usize, usize)) -> Style {
    let fg = interpolate_color(head.fg, tail.fg, progress);
    let bg = interpolate_color(head.bg, tail.bg, progress);
    Style {
        fg,
        bg,
        attrs: if progress.0 < progress.1 / 2 {
            head.attrs
        } else {
            tail.attrs
        },
    }
}

fn interpolate_color(head: Color, tail: Color, progress: (usize, usize)) -> Color {
    match (head, tail) {
        (Color::Rgb(hr, hg, hb), Color::Rgb(tr, tg, tb)) => Color::Rgb(
            blend(hr, tr, progress),
            blend(hg, tg, progress),
            blend(hb, tb, progress),
        ),
        _ if progress.0 < progress.1 / 2 => head,
        _ => tail,
    }
}

fn blend(head: u8, tail: u8, progress: (usize, usize)) -> u8 {
    let numerator = u32::try_from(progress.0).unwrap_or(u32::MAX);
    let denominator = u32::try_from(progress.1).unwrap_or(u32::MAX).max(1);
    let head = u32::from(head);
    let tail = u32::from(tail);
    let value =
        (head * (denominator - numerator) + tail * numerator + denominator / 2) / denominator;
    u8::try_from(value).unwrap_or(u8::MAX)
}
