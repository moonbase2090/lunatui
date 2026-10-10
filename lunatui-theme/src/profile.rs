// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod oklab;
mod palette;

use lunatui_core::Color;

use oklab::nearest;
use palette::rgb;

/// How many colors a frame is allowed to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum Profile {
    /// Keep RGB and indexed colors as written.
    TrueColor,
    /// Nearest of the 256 xterm palette entries.
    Ansi256,
    /// Nearest of the 16 ANSI colors.
    Ansi16,
    /// Nearest of black and white.
    Mono,
}

impl Profile {
    /// Maps `color` into this profile. `Default` and `Token` pass through.
    #[must_use]
    pub fn downsample(self, color: Color) -> Color {
        let Some(sample) = sample(color) else {
            return color;
        };
        match self {
            Self::TrueColor => color,
            Self::Ansi256 => match color {
                Color::Indexed(_) => color,
                _ => Color::Indexed(nearest(sample, 0u8..=255)),
            },
            Self::Ansi16 => match color {
                Color::Indexed(index) if index < 16 => color,
                _ => Color::Indexed(nearest(sample, 0u8..16)),
            },
            Self::Mono => Color::Indexed(nearest(sample, [0, 15])),
        }
    }
}

fn sample(color: Color) -> Option<[u8; 3]> {
    match color {
        Color::Default | Color::Token(_) => None,
        Color::Rgb(red, green, blue) => Some([red, green, blue]),
        Color::Indexed(index) => Some(rgb(index)),
    }
}
