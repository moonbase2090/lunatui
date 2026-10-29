// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[cfg(any(feature = "chart", feature = "canvas"))]
use lunatui_core::Rect;
#[cfg(any(feature = "chart", feature = "canvas"))]
use lunatui_core::subcell::GlyphSet;

/// Glyphs a drawing is made of.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Marker {
    /// One `•` per point, a cell each.
    Dot,
    /// One full block per point, a cell each.
    Block,
    /// A cell split into two stacked halves.
    HalfBlock,
    /// A cell split into a 2×2 grid of quadrant blocks.
    Quadrant,
    /// A cell split into a 2×3 grid of sextant blocks.
    Sextant,
    /// A cell split into a 2×4 grid of solid octant blocks, Braille's resolution without gaps.
    Octant,
    /// A cell split into a 2×4 grid of Braille dots, the finest resolution.
    #[default]
    Braille,
}

#[cfg(any(feature = "chart", feature = "canvas"))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ink {
    Empty,
    Dot,
    Block,
    Half,
    Quadrant,
    Sextant,
    Octant,
    Braille,
    #[cfg(feature = "chart")]
    Eighths,
}

#[cfg(any(feature = "chart", feature = "canvas"))]
#[derive(Clone, Copy)]
pub(crate) struct Spot {
    pub(crate) ink: Ink,
    pub(crate) mask: u8,
    pub(crate) series: usize,
}

#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) const EMPTY_SPOT: Spot = Spot {
    ink: Ink::Empty,
    mask: 0,
    series: 0,
};

pub(crate) const BRAILLE_BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
pub(crate) const QUADRANT: [char; 16] = [
    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
];

pub(crate) fn sextant(mask: u8) -> char {
    let code = match mask & 63 {
        0 => return ' ',
        21 => return '▌',
        42 => return '▐',
        63 => return '█',
        m @ 1..=20 => 0x1FB00 + u32::from(m) - 1,
        m @ 22..=41 => 0x1FB00 + u32::from(m) - 2,
        m => 0x1FB00 + u32::from(m) - 3,
    };
    char::from_u32(code).unwrap_or(' ')
}

#[cfg(feature = "chart")]
pub(crate) const EIGHTHS_UP: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) struct Raster {
    pub(crate) rect: Rect,
    pub(crate) spots: Vec<Spot>,
}

#[cfg(any(feature = "chart", feature = "canvas"))]
impl Raster {
    pub(crate) fn new(rect: Rect) -> Self {
        Self {
            rect,
            spots: vec![EMPTY_SPOT; usize::from(rect.width) * usize::from(rect.height)],
        }
    }

    pub(crate) fn spot(&mut self, column: u16, row: u16) -> Option<&mut Spot> {
        let index = usize::from(row) * usize::from(self.rect.width) + usize::from(column);
        self.spots.get_mut(index)
    }

    pub(crate) fn resolution(&self, marker: Marker) -> (i64, i64) {
        let (width, height) = (i64::from(self.rect.width), i64::from(self.rect.height));
        match marker {
            Marker::Braille | Marker::Octant => (width * 2, height * 4),
            Marker::Sextant => (width * 2, height * 3),
            Marker::Quadrant => (width * 2, height * 2),
            Marker::HalfBlock => (width, height * 2),
            Marker::Dot | Marker::Block => (width, height),
        }
    }

    pub(crate) fn mark(&mut self, marker: Marker, x: i64, y: i64, series: usize) {
        let (ink, cell_x, cell_y, mask) = match marker {
            Marker::Braille => (
                Ink::Braille,
                x / 2,
                y / 4,
                BRAILLE_BITS[usize::try_from(x % 2).unwrap_or(0)]
                    [usize::try_from(y % 4).unwrap_or(0)],
            ),
            Marker::Octant => (Ink::Octant, x / 2, y / 4, 1 << (x % 2 + 2 * (y % 4))),
            Marker::Sextant => (Ink::Sextant, x / 2, y / 3, 1 << (x % 2 + 2 * (y % 3))),
            Marker::Quadrant => (Ink::Quadrant, x / 2, y / 2, 1 << (x % 2 + 2 * (y % 2))),
            Marker::HalfBlock => (Ink::Half, x, y / 2, 1 << (y % 2)),
            Marker::Dot => (Ink::Dot, x, y, 1),
            Marker::Block => (Ink::Block, x, y, 1),
        };
        let Some(spot) = self.spot(
            u16::try_from(cell_x).unwrap_or(0),
            u16::try_from(cell_y).unwrap_or(0),
        ) else {
            return;
        };
        if spot.ink == ink {
            spot.mask |= mask;
        } else {
            spot.mask = mask;
        }
        spot.ink = ink;
        spot.series = series;
    }

    pub(crate) fn overlay(&mut self, layer: &Self) {
        for (spot, over) in self.spots.iter_mut().zip(&layer.spots) {
            if over.ink != Ink::Empty {
                *spot = *over;
            }
        }
    }

    #[cfg(feature = "chart")]
    pub(crate) fn eighths(&mut self, column: u16, ticks: i64, series: usize) {
        let height = i64::from(self.rect.height);
        let ticks = ticks.clamp(0, height * 8);
        for row in 0..height {
            let from_bottom = height - 1 - row;
            let filled = (ticks - from_bottom * 8).clamp(0, 8);
            if filled == 0 {
                continue;
            }
            let Some(spot) = self.spot(column, u16::try_from(row).unwrap_or(0)) else {
                continue;
            };
            spot.ink = Ink::Eighths;
            spot.mask = u8::try_from(filled).unwrap_or(8);
            spot.series = series;
        }
    }

    pub(crate) fn point(
        &mut self,
        window: Window,
        marker: Marker,
        (x, y): (f64, f64),
        series: usize,
    ) {
        if !window.contains((x, y)) {
            return;
        }
        let (sub_w, sub_h) = self.resolution(marker);
        self.mark(
            marker,
            window.column(x, sub_w),
            window.row(y, sub_h),
            series,
        );
    }

    #[cfg(feature = "canvas")]
    pub(crate) fn segment_centered(
        &mut self,
        window: Window,
        marker: Marker,
        a: (f64, f64),
        b: (f64, f64),
        series: usize,
    ) {
        let Some((a, b)) = window.clip(a, b) else {
            return;
        };
        let (sub_w, sub_h) = self.resolution(marker);
        let (x0, y0) = (window.column(a.0, sub_w), window.row(a.1, sub_h));
        let (x1, y1) = (window.column(b.0, sub_w), window.row(b.1, sub_h));
        let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
        if dx == 0 && dy == 0 {
            self.mark(marker, x0, y0, series);
            return;
        }
        if dx >= dy {
            let ((x0, y0), (_x1, y1)) = if x0 > x1 {
                ((x1, y1), (x0, y0))
            } else {
                ((x0, y0), (x1, y1))
            };
            let sy = if y0 < y1 { 1 } else { -1 };
            for offset in 0..=dx {
                let x = x0 + offset;
                let minor = round_ratio_to_even(offset * dy, dx);
                self.mark(marker, x, y0 + sy * minor, series);
            }
        } else {
            let ((x0, y0), (x1, _y1)) = if y0 > y1 {
                ((x1, y1), (x0, y0))
            } else {
                ((x0, y0), (x1, y1))
            };
            let sx = if x0 < x1 { 1 } else { -1 };
            for offset in 0..=dy {
                let minor = round_ratio_to_even(offset * dx, dy);
                let x = x0 + sx * minor;
                let y = y0 + offset;
                self.mark(marker, x, y, series);
            }
        }
    }

    #[cfg(any(feature = "canvas", feature = "chart"))]
    pub(crate) fn segment(
        &mut self,
        window: Window,
        marker: Marker,
        a: (f64, f64),
        b: (f64, f64),
        series: usize,
    ) {
        let Some((a, b)) = window.clip(a, b) else {
            return;
        };
        let (sub_w, sub_h) = self.resolution(marker);
        let (x0, y0) = (window.column(a.0, sub_w), window.row(a.1, sub_h));
        let (x1, y1) = (window.column(b.0, sub_w), window.row(b.1, sub_h));
        let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
        if dy < dx {
            let ((x0, y0), (x1, y1)) = if x0 > x1 {
                ((x1, y1), (x0, y0))
            } else {
                ((x0, y0), (x1, y1))
            };
            let step = if y1 < y0 { -1 } else { 1 };
            let (mut y, mut error) = (y0, 2 * dy - dx);
            for x in x0..=x1 {
                self.mark(marker, x, y, series);
                if error > 0 {
                    y += step;
                    error -= 2 * dx;
                }
                error += 2 * dy;
            }
        } else {
            let ((x0, y0), (x1, y1)) = if y0 > y1 {
                ((x1, y1), (x0, y0))
            } else {
                ((x0, y0), (x1, y1))
            };
            let step = if x1 < x0 { -1 } else { 1 };
            let (mut x, mut error) = (x0, 2 * dx - dy);
            for y in y0..=y1 {
                self.mark(marker, x, y, series);
                if error > 0 {
                    x += step;
                    error -= 2 * dy;
                }
                error += 2 * dx;
            }
        }
    }
}

#[cfg(feature = "canvas")]
fn round_ratio_to_even(numerator: i64, denominator: i64) -> i64 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    match (remainder * 2).cmp(&denominator) {
        core::cmp::Ordering::Less => quotient,
        core::cmp::Ordering::Greater => quotient + 1,
        core::cmp::Ordering::Equal => quotient + (quotient % 2),
    }
}

#[cfg(any(feature = "chart", feature = "canvas"))]
#[derive(Clone, Copy)]
pub(crate) struct Window {
    pub(crate) x: [f64; 2],
    pub(crate) y: [f64; 2],
}

#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) const LEFT: u8 = 1;
#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) const RIGHT: u8 = 2;
#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) const BOTTOM: u8 = 4;
#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) const TOP: u8 = 8;

pub(crate) fn round_to_tick(value: f64) -> i64 {
    let bits = value.round().to_bits();
    let exponent = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0) - 1023;
    if exponent < 0 {
        return 0;
    }
    let mantissa = (bits & ((1_u64 << 52) - 1)) + (1_u64 << 52);
    let shift = u32::try_from(52 - exponent.min(52)).unwrap_or(0);
    i64::try_from(mantissa >> shift).unwrap_or(i64::MAX)
}

#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) fn span(cells: i64) -> f64 {
    f64::from(i32::try_from(cells).unwrap_or(i32::MAX))
}

#[cfg(any(feature = "chart", feature = "canvas"))]
impl Window {
    pub(crate) fn width(self) -> f64 {
        self.x[1] - self.x[0]
    }

    pub(crate) fn height(self) -> f64 {
        self.y[1] - self.y[0]
    }

    pub(crate) fn usable(self) -> bool {
        self.width().is_finite()
            && self.height().is_finite()
            && self.width() != 0.0
            && self.height() != 0.0
    }

    pub(crate) fn region(self, (x, y): (f64, f64)) -> u8 {
        let (left, right) = (self.x[0].min(self.x[1]), self.x[0].max(self.x[1]));
        let (bottom, top) = (self.y[0].min(self.y[1]), self.y[0].max(self.y[1]));
        let mut region = 0;
        if x < left {
            region |= LEFT;
        } else if x > right {
            region |= RIGHT;
        }
        if y < bottom {
            region |= BOTTOM;
        } else if y > top {
            region |= TOP;
        }
        region
    }

    pub(crate) fn contains(self, point: (f64, f64)) -> bool {
        self.region(point) == 0
    }

    pub(crate) fn cut(self, from: (f64, f64), toward: (f64, f64), region: u8) -> (f64, f64) {
        let (left, right) = (self.x[0].min(self.x[1]), self.x[0].max(self.x[1]));
        let (bottom, top) = (self.y[0].min(self.y[1]), self.y[0].max(self.y[1]));
        let (dx, dy) = (toward.0 - from.0, toward.1 - from.1);
        if region & LEFT != 0 {
            (left, from.1 + (left - from.0) * dy / dx)
        } else if region & RIGHT != 0 {
            (right, from.1 + (right - from.0) * dy / dx)
        } else if region & BOTTOM != 0 {
            (from.0 + (bottom - from.1) * dx / dy, bottom)
        } else {
            (from.0 + (top - from.1) * dx / dy, top)
        }
    }

    pub(crate) fn clip(
        self,
        mut a: (f64, f64),
        mut b: (f64, f64),
    ) -> Option<((f64, f64), (f64, f64))> {
        for _ in 0..8 {
            let (ra, rb) = (self.region(a), self.region(b));
            if ra & rb != 0 {
                return None;
            }
            if ra != 0 {
                a = self.cut(a, b, ra);
            } else if rb != 0 {
                b = self.cut(b, a, rb);
            } else {
                return Some((a, b));
            }
        }
        None
    }

    pub(crate) fn column(self, x: f64, cells: i64) -> i64 {
        round_to_tick((x - self.x[0]) * span(cells - 1) / self.width())
    }

    pub(crate) fn row(self, y: f64, cells: i64) -> i64 {
        round_to_tick((self.y[1] - y) * span(cells - 1) / self.height())
    }

    #[cfg(feature = "chart")]
    pub(crate) fn ticks(self, y: f64, eighths: i64) -> i64 {
        round_to_tick((y - self.y[0]) * span(eighths) / self.height())
    }
}

#[cfg(any(feature = "chart", feature = "canvas"))]
pub(crate) fn glyph(spot: Spot) -> char {
    match spot.ink {
        Ink::Empty => ' ',
        Ink::Dot => '•',
        Ink::Block => '█',
        Ink::Half => match spot.mask {
            1 => '▀',
            2 => '▄',
            _ => '█',
        },
        Ink::Quadrant => QUADRANT[usize::from(spot.mask & 15)],
        Ink::Sextant => sextant(spot.mask),
        Ink::Octant => GlyphSet::Octants.glyph(spot.mask),
        Ink::Braille => char::from_u32(0x2800 + u32::from(spot.mask)).unwrap_or(' '),
        #[cfg(feature = "chart")]
        Ink::Eighths => EIGHTHS_UP[usize::from(spot.mask.min(8))],
    }
}
