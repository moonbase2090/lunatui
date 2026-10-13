// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

use lunatui_core::{Color, Rect};

use super::ease::Ease;

/// Time-based interpolation between two values (PRD §2.10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween<T> {
    from: T,
    to: T,
    duration: Duration,
    ease: Ease,
}

impl<T> Tween<T> {
    /// Builds a tween from `from` to `to` over `duration`.
    #[must_use]
    pub const fn new(from: T, to: T, duration: Duration, ease: Ease) -> Self {
        Self {
            from,
            to,
            duration,
            ease,
        }
    }

    /// Normalized duration used by the tween.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }
}

impl Tween<f32> {
    /// Samples the tween at `elapsed` since the start.
    #[must_use]
    pub fn sample(self, elapsed: Duration) -> f32 {
        let progress = normalized(elapsed, self.duration);
        let eased = self.ease.apply(progress);
        self.from + (self.to - self.from) * eased
    }
}

impl Tween<Rect> {
    /// Samples each edge with the layout lerp rule (fractional cells floor).
    #[must_use]
    pub fn sample(self, elapsed: Duration) -> Rect {
        let eased = self.ease.apply(normalized(elapsed, self.duration));
        Rect::new(
            lerp_u16(self.from.x, self.to.x, eased),
            lerp_u16(self.from.y, self.to.y, eased),
            lerp_u16(self.from.width, self.to.width, eased),
            lerp_u16(self.from.height, self.to.height, eased),
        )
    }
}

impl Tween<Color> {
    /// Blends [`Color::Rgb`] values in `OKLab`; other kinds step at the end.
    #[must_use]
    pub fn sample(self, elapsed: Duration) -> Color {
        let eased = self.ease.apply(normalized(elapsed, self.duration));
        match (self.from, self.to) {
            (Color::Rgb(r0, g0, b0), Color::Rgb(r1, g1, b1)) => {
                let blended = oklab_lerp_rgb([r0, g0, b0], [r1, g1, b1], eased);
                Color::Rgb(blended[0], blended[1], blended[2])
            }
            _ if eased >= 1.0 => self.to,
            _ => self.from,
        }
    }
}

fn normalized(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    let progress = elapsed.as_secs_f32() / duration.as_secs_f32();
    progress.clamp(0.0, 1.0)
}

fn lerp_u16(from: u16, to: u16, progress: f32) -> u16 {
    if progress <= 0.0 {
        return from;
    }
    if progress >= 1.0 {
        return to;
    }
    if from <= to {
        from + fraction_floor(to - from, progress)
    } else {
        from - fraction_floor(from - to, progress)
    }
}

fn fraction_floor(span: u16, progress: f32) -> u16 {
    if span == 0 || progress <= 0.0 {
        return 0;
    }
    if progress >= 1.0 {
        return span;
    }
    let permille = f32_permille(progress);
    let step = u32::from(span) * permille / 1000;
    u16::try_from(step).unwrap_or(span)
}

fn f32_permille(progress: f32) -> u32 {
    if progress <= 0.0 {
        0
    } else if progress >= 1.0 {
        1000
    } else {
        let threshold = progress * 1000.0;
        let mut units = 0_u32;
        while f32::from(u16::try_from(units).unwrap_or(1000)) < threshold && units < 1000 {
            units += 1;
        }
        units
    }
}

#[derive(Clone, Copy)]
struct Oklab {
    lightness: f64,
    a: f64,
    b: f64,
}

fn oklab_lerp_rgb(from: [u8; 3], to: [u8; 3], progress: f32) -> [u8; 3] {
    let start = rgb_to_oklab(from);
    let end = rgb_to_oklab(to);
    let mix = f64::from(progress);
    let blended = Oklab {
        lightness: start.lightness + (end.lightness - start.lightness) * mix,
        a: start.a + (end.a - start.a) * mix,
        b: start.b + (end.b - start.b) * mix,
    };
    oklab_to_rgb(blended)
}

fn rgb_to_oklab(rgb: [u8; 3]) -> Oklab {
    let red = srgb_to_linear(rgb[0]);
    let green = srgb_to_linear(rgb[1]);
    let blue = srgb_to_linear(rgb[2]);
    let long = 0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue;
    let medium = 0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue;
    let short = 0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue;
    let long = long.cbrt();
    let medium = medium.cbrt();
    let short = short.cbrt();
    Oklab {
        lightness: 0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short,
        a: 1.977_998_495_1 * long - 2.357_996_504_2 * medium + 0.061_581_897_7 * short,
        b: 0.943_152_168_7 * long + 0.116_596_280_4 * medium - 0.059_801_647_9 * short,
    }
}

fn oklab_to_rgb(oklab: Oklab) -> [u8; 3] {
    let long = oklab.lightness + 0.396_337_777_4 * oklab.a + 0.215_803_757_3 * oklab.b;
    let medium = oklab.lightness - 0.105_561_345_8 * oklab.a - 0.063_854_172_8 * oklab.b;
    let short = oklab.lightness - 0.089_484_177_5 * oklab.a - 1.291_485_548_0 * oklab.b;
    let long = long.powi(3);
    let medium = medium.powi(3);
    let short = short.powi(3);
    let red = 4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short;
    let green = -1.268_438_004_6 * long + 2.609_757_401_5 * medium - 0.341_319_396_5 * short;
    let blue = -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701_0 * short;
    [
        linear_to_srgb(red),
        linear_to_srgb(green),
        linear_to_srgb(blue),
    ]
}

fn srgb_to_linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(channel: f64) -> u8 {
    let clamped = channel.clamp(0.0, 1.0);
    let encoded = if clamped <= 0.003_130_8 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    round_unit_to_u8(encoded)
}

fn round_unit_to_u8(unit: f64) -> u8 {
    let scaled = unit.clamp(0.0, 1.0) * 255.0;
    if scaled >= 254.5 {
        255
    } else if scaled <= 0.5 {
        0
    } else {
        let mut byte = 1u8;
        while byte < 255 && scaled >= f64::from(byte) + 0.5 {
            byte += 1;
        }
        byte
    }
}
