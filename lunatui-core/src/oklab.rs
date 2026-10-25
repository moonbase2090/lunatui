// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// A finite `OKLab` color constructed from an sRGB triplet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklab {
    lightness: f64,
    a: f64,
    b: f64,
}

/// Interpolates sRGB in `OKLab`; progress is clamped to 0..1 and NaN selects the start.
#[must_use]
pub fn lerp_rgb(from: [u8; 3], to: [u8; 3], progress: f32) -> [u8; 3] {
    if progress.is_nan() || progress <= 0.0 {
        return from;
    }
    if progress >= 1.0 {
        return to;
    }
    let start = from_rgb(from);
    let end = from_rgb(to);
    let mix = f64::from(progress);
    let blended = Oklab {
        lightness: start.lightness + (end.lightness - start.lightness) * mix,
        a: start.a + (end.a - start.a) * mix,
        b: start.b + (end.b - start.b) * mix,
    };
    to_rgb(blended)
}

/// Converts an 8-bit sRGB triplet into `OKLab` coordinates.
#[must_use]
pub fn from_rgb(rgb: [u8; 3]) -> Oklab {
    let red = srgb_to_linear(rgb[0]);
    let green = srgb_to_linear(rgb[1]);
    let blue = srgb_to_linear(rgb[2]);
    let long = 0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue;
    let medium = 0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue;
    let short = 0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue;
    let long = libm::cbrt(long);
    let medium = libm::cbrt(medium);
    let short = libm::cbrt(short);
    Oklab {
        lightness: 0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short,
        a: 1.977_998_495_1 * long - 2.428_592_205_0 * medium + 0.450_593_709_9 * short,
        b: 0.025_904_037_1 * long + 0.782_771_766_2 * medium - 0.808_675_766_0 * short,
    }
}

/// Converts `OKLab` to sRGB, clipping out-of-gamut channels and rounding to the nearest byte.
#[must_use]
pub fn to_rgb(oklab: Oklab) -> [u8; 3] {
    let long = oklab.lightness + 0.396_337_777_4 * oklab.a + 0.215_803_757_3 * oklab.b;
    let medium = oklab.lightness - 0.105_561_345_8 * oklab.a - 0.063_854_172_8 * oklab.b;
    let short = oklab.lightness - 0.089_484_177_5 * oklab.a - 1.291_485_548_0 * oklab.b;
    let long = long * long * long;
    let medium = medium * medium * medium;
    let short = short * short * short;
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
        libm::pow((value + 0.055) / 1.055, 2.4)
    }
}

fn linear_to_srgb(channel: f64) -> u8 {
    let clamped = channel.clamp(0.0, 1.0);
    let encoded = if clamped <= 0.003_130_8 {
        clamped * 12.92
    } else {
        1.055 * libm::pow(clamped, 1.0 / 2.4) - 0.055
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
        let (mut low, mut high) = (1_u8, 255_u8);
        while low < high {
            let mid = low + (high - low) / 2;
            if scaled >= f64::from(mid) + 0.5 {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }
}

impl Oklab {
    /// Lightness, green-red and blue-yellow coordinates, in that order.
    #[must_use]
    pub const fn components(self) -> [f64; 3] {
        [self.lightness, self.a, self.b]
    }
}
