// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};

use crate::spinner_preset::SpinnerPreset;
use crate::{RenderCx, Widget};

const BLANK: &str = "                                                                ";

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const HALF_MOON: [&str; 4] = ["◐", "◓", "◑", "◒"];
const QUADRANT: [&str; 4] = ["▖", "▗", "▝", "▘"];
const BRAILLE_RING: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
const ARC: [&str; 4] = ["◜", "◝", "◞", "◟"];
const BARS: [&str; 6] = ["▁▃▅", "▃▅▇", "▅▇▅", "▇▅▃", "▅▃▁", "▃▁▃"];

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

/// A frame from a spinner pattern.
///
/// The widget is stateless: the caller supplies each frame index with `new` and schedules redraws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spinner {
    frame: u32,
    style: Style,
    pattern: SpinnerPattern,
    preset: Option<SpinnerPreset>,
}

impl Spinner {
    /// Spinner at animation frame `frame`.
    #[must_use]
    pub fn new(frame: u32) -> Self {
        Self {
            frame,
            style: Style::new(),
            pattern: SpinnerPattern::BrailleDots,
            preset: None,
        }
    }

    /// Selects the glyph cycle and clears any [`preset`](Self::preset).
    ///
    /// The default is [`SpinnerPattern::BrailleDots`].
    #[must_use]
    pub fn pattern(mut self, pattern: SpinnerPattern) -> Self {
        self.pattern = pattern;
        self.preset = None;
        self
    }

    /// Draws `preset` instead of the pattern. [`pattern`](Self::pattern) clears it.
    ///
    /// The caller still supplies `frame`. Step it once per [`SpinnerPreset::interval`].
    /// A shorter frame clears the rest of the preset's width, which is the widest frame.
    #[must_use]
    pub fn preset(mut self, preset: SpinnerPreset) -> Self {
        self.preset = Some(preset);
        self
    }

    /// Glyph color.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// The frame string this spinner draws, for use inside text such as a table cell.
    #[must_use]
    pub fn glyph(&self) -> &'static str {
        let frames = match self.preset {
            Some(preset) => preset.frames(),
            None => self.pattern.frames(),
        };
        let Some(len) = u32::try_from(frames.len()).ok().filter(|len| *len > 0) else {
            return "";
        };
        let index = usize::try_from(self.frame % len).unwrap_or(0);
        frames[index]
    }
}

impl Widget for Spinner {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if let Some(preset) = self.preset {
            render_preset(self.frame, self.style, preset, area, cx);
            return;
        }
        cx.put_str(area.x, area.y, self.glyph(), self.style);
    }

    fn measure(&self, avail: Size) -> Size {
        let columns = if let Some(preset) = self.preset {
            preset_width(preset)
        } else {
            let frames = self.pattern.frames();
            frames
                .first()
                .map_or(1, |frame| u16::try_from(width(frame)).unwrap_or(u16::MAX))
        };
        Size::new(columns, 1).clamp(avail)
    }
}

fn preset_width(preset: SpinnerPreset) -> u16 {
    let mut max = 0_u16;
    for frame in preset.frames() {
        let columns = u16::try_from(width(frame)).unwrap_or(u16::MAX);
        if columns > max {
            max = columns;
        }
    }
    max
}

fn render_preset(
    frame_index: u32,
    style: Style,
    preset: SpinnerPreset,
    area: Rect,
    cx: &mut RenderCx<'_>,
) {
    let frames = preset.frames();
    let Some(len) = u32::try_from(frames.len()).ok().filter(|len| *len > 0) else {
        return;
    };
    let columns = preset_width(preset).min(area.width);
    if columns == 0 {
        return;
    }
    clear_columns(cx, area.x, area.y, columns, style);
    let Some(index) = usize::try_from(frame_index % len).ok() else {
        return;
    };
    let Some(frame) = frames.get(index) else {
        return;
    };
    cx.put_str(area.x, area.y, clip_display_prefix(frame, columns), style);
}

fn clear_columns(cx: &mut RenderCx<'_>, x: u16, y: u16, columns: u16, style: Style) {
    let mut written = 0_u16;
    while written < columns {
        let rest = usize::from(columns.saturating_sub(written));
        let take = rest.min(BLANK.len());
        if take == 0 {
            break;
        }
        let Some(step) = u16::try_from(take).ok() else {
            break;
        };
        cx.put_str(x.saturating_add(written), y, &BLANK[..take], style);
        written = written.saturating_add(step);
    }
}
