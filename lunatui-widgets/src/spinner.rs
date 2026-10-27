// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, width};

use crate::{RenderCx, Widget};

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const HALF_MOON: [&str; 4] = ["◐", "◓", "◑", "◒"];
const QUADRANT: [&str; 4] = ["▖", "▗", "▝", "▘"];
const BRAILLE_RING: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
const ARC: [&str; 4] = ["◜", "◝", "◞", "◟"];
const BARS: [&str; 1] = ["▁▃▅"];

/// Frame family used by a spinner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerPattern {
    /// Rotating Braille dots.
    #[default]
    BrailleDots,
    /// A rotating half moon.
    HalfMoon,
    /// A four-corner quadrant cycle.
    Quadrant,
    /// A Braille ring cycle.
    BrailleRing,
    /// A four-corner arc cycle.
    Arc,
    /// A rising and falling bar cycle.
    Bars,
}

impl SpinnerPattern {
    const fn frames(self) -> &'static [&'static str] {
        match self {
            Self::BrailleDots => &FRAMES,
            Self::HalfMoon => &HALF_MOON,
            Self::Quadrant => &QUADRANT,
            Self::BrailleRing => &BRAILLE_RING,
            Self::Arc => &ARC,
            Self::Bars => &BARS,
        }
    }
}

/// Braille animation frame (ratatui dashboard spinner parity).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spinner {
    frame: u32,
    style: Style,
    pattern: SpinnerPattern,
}

impl Spinner {
    /// Spinner at animation frame `frame`.
    #[must_use]
    pub fn new(frame: u32) -> Self {
        Self {
            frame,
            style: Style::new(),
            pattern: SpinnerPattern::BrailleDots,
        }
    }

    /// Selects the glyph cycle. The default is [`SpinnerPattern::BrailleDots`].
    #[must_use]
    pub fn pattern(mut self, pattern: SpinnerPattern) -> Self {
        self.pattern = pattern;
        self
    }

    /// Glyph color.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Spinner {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let frames = self.pattern.frames();
        let index =
            usize::try_from(self.frame % u32::try_from(frames.len()).unwrap_or(1)).unwrap_or(0);
        cx.put_str(area.x, area.y, frames[index], self.style);
    }

    fn measure(&self, avail: Size) -> Size {
        let frames = self.pattern.frames();
        let columns = frames
            .first()
            .map_or(1, |frame| u16::try_from(width(frame)).unwrap_or(u16::MAX));
        Size::new(columns, 1).clamp(avail)
    }
}
