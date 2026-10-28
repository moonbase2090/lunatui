// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Color, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const EIGHTHS: u16 = 8;
const PARTIAL: [&str; 7] = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"];
const FULL: &str = "█";

/// The color of one [`ProgressBar`] segment, taken from the theme.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressTone {
    /// The accent color.
    Accent,
    /// The ok color.
    Ok,
    /// The warn color.
    Warn,
    /// The fail color.
    Fail,
}

impl ProgressTone {
    fn token(self) -> Token {
        match self {
            Self::Accent => Token::Accent,
            Self::Ok => Token::Ok,
            Self::Warn => Token::Warn,
            Self::Fail => Token::Fail,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Segment {
    amount: f64,
    tone: ProgressTone,
}

/// A progress bar drawn with eighth blocks, so its fill moves in steps of an eighth of a cell.
///
/// Three modes share one widget: [`ProgressBar::ratio`] fills a single segment, repeated
/// [`ProgressBar::segment`] calls stack colored segments (a cell where two segments meet shows
/// both colors), and [`ProgressBar::indeterminate`] draws a block that bounces along the track for
/// work with no known size. The unfilled part is the raised surface color. A label before the bar
/// and the percentage after it are optional, and the bar fills every row of its area.
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressBar<'a> {
    segments: Vec<Segment>,
    sweep: Option<u32>,
    label: Option<Cow<'a, str>>,
    show_percent: bool,
    state: WidgetState,
    style: Option<Style>,
}

impl Default for ProgressBar<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ProgressBar<'a> {
    /// An empty bar.
    #[must_use]
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            sweep: None,
            label: None,
            show_percent: false,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// One accent segment of `ratio` of the bar, replacing any segments. The ratio is clamped
    /// into `0.0..=1.0` and NaN counts as zero.
    #[must_use]
    pub fn ratio(mut self, ratio: f64) -> Self {
        self.segments = vec![Segment {
            amount: ratio,
            tone: ProgressTone::Accent,
        }];
        self.sweep = None;
        self
    }

    /// Adds a segment of `amount` of the bar after the ones already there. Negative and NaN
    /// amounts count as zero, and segments past the end of the bar are cut off.
    #[must_use]
    pub fn segment(mut self, amount: f64, tone: ProgressTone) -> Self {
        self.segments.push(Segment { amount, tone });
        self.sweep = None;
        self
    }

    /// An indeterminate bar: a block a quarter of the track wide that bounces from end to end,
    /// moving one eighth of a cell per tick.
    #[must_use]
    pub fn indeterminate(mut self, tick: u32) -> Self {
        self.sweep = Some(tick);
        self.segments.clear();
        self
    }

    /// Text before the bar, one cell away from it.
    #[must_use]
    pub fn label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows the filled share as a whole percentage after the bar. An indeterminate bar shows
    /// nothing.
    #[must_use]
    pub const fn show_percent(mut self, show: bool) -> Self {
        self.show_percent = show;
        self
    }

    /// Interaction state: focused draws the label bold, disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws the filled part in `style`'s foreground instead of theme colors.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    fn amounts(&self) -> Vec<f64> {
        self.segments
            .iter()
            .map(|segment| {
                if segment.amount.is_nan() {
                    0.0
                } else {
                    segment.amount.clamp(0.0, 1.0)
                }
            })
            .collect()
    }

    /// The filled share of the bar as a whole percentage (`None` for an indeterminate bar).
    #[must_use]
    pub fn percent(&self) -> Option<u8> {
        if self.sweep.is_some() {
            return None;
        }
        let total: f64 = self.amounts().iter().sum();
        let whole = u8::try_from(nearest(100, total)).unwrap_or(100);
        Some(whole)
    }

    fn percent_text(&self) -> String {
        if !self.show_percent {
            return String::new();
        }
        self.percent()
            .map_or_else(String::new, |p| format!(" {p}%"))
    }

    /// The cells the bar covers when rendered into `area`: what is left of the width after the
    /// label and the widest percentage text.
    #[must_use]
    pub fn track_area(&self, area: Rect) -> Rect {
        let label = self.label.as_ref().map_or(0, |label| width(label) + 1);
        let percent = if self.show_percent && self.sweep.is_none() {
            width(" 100%")
        } else {
            0
        };
        let before = u16::try_from(label).unwrap_or(u16::MAX);
        let after = u16::try_from(percent).unwrap_or(u16::MAX);
        let room = area.width.saturating_sub(before).saturating_sub(after);
        Rect::new(area.x.saturating_add(before), area.y, room, area.height)
    }

    fn spans(&self, total: u16) -> Vec<(u16, u16, Option<ProgressTone>)> {
        let mut spans = Vec::new();
        if let Some(tick) = self.sweep {
            let len = (total / 4).max(EIGHTHS.min(total));
            let travel = total.saturating_sub(len);
            let cycle = u32::from(travel) * 2;
            let step = tick.checked_rem(cycle).unwrap_or(0);
            let start = u16::try_from(step.min(cycle - step)).unwrap_or(0);
            spans.push((start, start + len, Some(ProgressTone::Accent)));
            return spans;
        }
        let mut from = 0;
        let mut so_far = 0.0_f64;
        for (segment, amount) in self.segments.iter().zip(self.amounts()) {
            so_far += amount;
            let to = nearest(total, so_far).max(from);
            spans.push((from, to, Some(segment.tone)));
            from = to;
        }
        spans
    }

    fn tone_at(spans: &[(u16, u16, Option<ProgressTone>)], eighth: u16) -> Option<ProgressTone> {
        spans
            .iter()
            .find(|(from, to, _)| eighth >= *from && eighth < *to)
            .and_then(|(_, _, tone)| *tone)
    }

    fn color(&self, cx: &RenderCx<'_>, tone: Option<ProgressTone>) -> Color {
        match (self.style, tone) {
            (Some(style), Some(_)) => style.fg,
            (_, Some(tone)) => cx.token(tone.token()),
            (_, None) => cx.token(Token::Surface1),
        }
    }
}

impl Widget for ProgressBar<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .with_attrs(Attrs::new().set_dim(disabled).set_bold(focused));
        if let Some(label) = &self.label {
            let room = self.track_area(area).x.saturating_sub(area.x);
            cx.put_str(
                area.x,
                area.y,
                clip_display_prefix(&format!("{label} "), room),
                text,
            );
        }
        let track = self.track_area(area);
        let total = track.width.saturating_mul(EIGHTHS);
        let spans = self.spans(total);
        let dim = Attrs::new().set_dim(disabled);
        for cell in 0..track.width {
            let base = cell * EIGHTHS;
            let left = Self::tone_at(&spans, base);
            let change = (1..EIGHTHS).find(|offset| Self::tone_at(&spans, base + offset) != left);
            let (glyph, fg, bg) = match change {
                Some(at) => (
                    PARTIAL[usize::from(at - 1)],
                    self.color(cx, left),
                    self.color(cx, Self::tone_at(&spans, base + at)),
                ),
                None => (FULL, self.color(cx, left), self.color(cx, left)),
            };
            let style = Style::new().fg(fg).bg(bg).with_attrs(dim);
            for row in 0..track.height {
                cx.put_str(track.x + cell, track.y + row, glyph, style);
            }
        }
        let value = self.percent_text();
        let after = track.x.saturating_add(track.width);
        let end = area.x.saturating_add(area.width);
        cx.put_str(
            after,
            area.y,
            clip_display_prefix(&value, end.saturating_sub(after)),
            text,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}

fn nearest(count: u16, fraction: f64) -> u16 {
    let distance = |step: u16| (f64::from(step) / f64::from(count.max(1)) - fraction).abs();
    (0..=count)
        .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
        .unwrap_or(0)
}
