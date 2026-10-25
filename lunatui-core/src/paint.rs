// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use core::fmt;

use crate::{Color, oklab};

/// A bounded fixed-point foreground weight, from zero through 256.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opacity(u16);

impl Opacity {
    /// A fully transparent foreground.
    pub const TRANSPARENT: Self = Self(0);
    /// A fully opaque foreground.
    pub const OPAQUE: Self = Self(256);

    /// Clamps `weight` to 0..=256; 128 is half opacity.
    #[must_use]
    pub const fn new(weight: u16) -> Self {
        Self(if weight > 256 { 256 } else { weight })
    }

    /// The foreground weight, from zero through 256.
    #[must_use]
    pub const fn weight(self) -> u16 {
        self.0
    }
}

/// Composites a foreground over an opaque sRGB background using Graphite's fixed-point rule.
#[must_use]
pub fn blend_rgb(background: [u8; 3], foreground: [u8; 3], opacity: Opacity) -> [u8; 3] {
    let foreground_weight = u32::from(opacity.weight());
    let background_weight = 256 - foreground_weight;
    core::array::from_fn(|channel| {
        let mixed = u32::from(background[channel]) * background_weight
            + u32::from(foreground[channel]) * foreground_weight;
        u8::try_from(mixed / 256).unwrap_or(255)
    })
}

/// The output policy resolved once when preparing a color ramp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampOutput {
    /// Prepare a perceptual truecolor gradient.
    TrueColor,
    /// Snap every sample to this semantic or palette color on reduced-color terminals.
    Solid(Color),
}

/// An unsupported number of prepared samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RampError;

impl fmt::Display for RampError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("a prepared ramp needs between 2 and 65536 samples")
    }
}

impl core::error::Error for RampError {}

/// An immutable, fixed-size color table prepared at startup or when its theme changes.
///
/// Preparation performs `OKLab` conversion; sampling only reads the table. No sample allocates,
/// converts color spaces, or searches a terminal palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedRamp<const N: usize> {
    colors: [Color; N],
}

impl<const N: usize> PreparedRamp<N> {
    /// Prepares `N` samples between resolved sRGB theme colors, including both endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`RampError`] unless `N` is between 2 and 65536 inclusive.
    pub fn new(from: [u8; 3], to: [u8; 3], output: RampOutput) -> Result<Self, RampError> {
        if !(2..=65_536).contains(&N) {
            return Err(RampError);
        }
        let colors = match output {
            RampOutput::Solid(color) => [color; N],
            RampOutput::TrueColor => core::array::from_fn(|index| {
                let step = u16::try_from(index).unwrap_or(0);
                let steps = u16::try_from(N - 1).unwrap_or(1);
                let rgb = oklab::lerp_rgb(from, to, f32::from(step) / f32::from(steps));
                Color::Rgb(rgb[0], rgb[1], rgb[2])
            }),
        };
        Ok(Self { colors })
    }

    /// Samples the nearest prepared color at a phase from zero through `u16::MAX`.
    #[must_use]
    pub fn sample(&self, phase: u16) -> Color {
        let steps = u32::try_from(N - 1).unwrap_or(0);
        let index = (u32::from(phase) * steps + 32_767) / u32::from(u16::MAX);
        self.colors[usize::try_from(index).unwrap_or(0)]
    }

    /// Reads a prepared color by column or row index, clamping beyond the end.
    #[must_use]
    pub fn color_at(&self, index: usize) -> Color {
        self.colors[index.min(N - 1)]
    }

    /// Borrows the complete prepared table.
    #[must_use]
    pub fn colors(&self) -> &[Color] {
        &self.colors
    }
}
