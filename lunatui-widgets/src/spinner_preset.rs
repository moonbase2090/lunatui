// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

/// One named spinner cycle: the frames it draws and the time between them.
///
/// The spinner does not sleep. Advance [`Spinner::new`](crate::Spinner::new) once
/// per [`interval`](Self::interval).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpinnerPreset {
    name: &'static str,
    frames: &'static [&'static str],
    interval_ms: u64,
}

impl SpinnerPreset {
    /// Preset `name` drawing `frames` and waiting `interval_ms` between frames.
    ///
    /// An empty frame list draws nothing. A zero interval is stored as given;
    /// callers that schedule from [`interval`](Self::interval) should use a
    /// non-zero value. The built-in presets do.
    #[must_use]
    pub const fn new(
        name: &'static str,
        frames: &'static [&'static str],
        interval_ms: u64,
    ) -> Self {
        Self {
            name,
            frames,
            interval_ms,
        }
    }

    /// Braille dots, 80 ms.
    pub const DOTS: Self = Self::new(
        "dots",
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        80,
    );

    /// ASCII line, 120 ms.
    pub const LINE: Self = Self::new("line", &["|", "/", "-", "\\"], 120);

    /// Four-corner arc, 100 ms.
    pub const ARC: Self = Self::new("arc", &["◜", "◝", "◞", "◟"], 100);

    /// A ball crossing a six-column lane and coming back, 80 ms.
    pub const BOUNCE: Self = Self::new(
        "bounce",
        &[
            "●", " ●", "  ●", "   ●", "    ●", "     ●", "    ●", "   ●", "  ●", " ●",
        ],
        80,
    );

    /// Growing and shrinking dot, 160 ms.
    pub const PULSE: Self = Self::new("pulse", &["∙", "•", "●", "◉", "●", "•"], 160);

    /// Single-column moon phases, 200 ms.
    pub const MOON: Self = Self::new("moon", &["○", "◔", "◑", "◕", "●", "◕", "◑", "◔"], 200);

    /// Braille ring, 90 ms.
    pub const RING: Self = Self::new("ring", &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], 90);

    /// Rising and falling bars, 110 ms.
    pub const BARS: Self = Self::new("bars", &["▁▃▅", "▃▅▇", "▅▇▅", "▇▅▃", "▅▃▁", "▃▁▃"], 110);

    /// Four-corner quadrant, 140 ms.
    pub const QUADRANT: Self = Self::new("quadrant", &["▖", "▗", "▝", "▘"], 140);

    /// Rotating half moon, 180 ms.
    pub const HALF_MOON: Self = Self::new("half-moon", &["◐", "◓", "◑", "◒"], 180);

    /// Built-in presets, in catalog order.
    pub const ALL: &'static [Self] = &[
        Self::DOTS,
        Self::LINE,
        Self::ARC,
        Self::BOUNCE,
        Self::PULSE,
        Self::MOON,
        Self::RING,
        Self::BARS,
        Self::QUADRANT,
        Self::HALF_MOON,
    ];

    /// Preset name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Frames in cycle order.
    #[must_use]
    pub const fn frames(self) -> &'static [&'static str] {
        self.frames
    }

    /// Time between frames.
    #[must_use]
    pub const fn interval(self) -> Duration {
        Duration::from_millis(self.interval_ms)
    }
}
