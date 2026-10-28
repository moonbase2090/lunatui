// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const FULL: &str = "━";
const LEFT_HALF: &str = "╸";
const RIGHT_HALF: &str = "╺";
const EMPTY: &str = "─";
const KNOB: &str = "●";

/// A one-row slider for a single value or a range: `━━━━╸──────` with half-cell precision.
///
/// The filled part of the track runs from the minimum to the value, or between the two ends of a
/// range, and each end can sit on a half cell (`╸` or `╺`). [`Slider::knob`] adds a `●` on the
/// cell of each end. [`Slider::value_at`] turns a click or drag column into a value (snapped to
/// the step), [`Slider::nudge`] moves a value by whole steps for the keyboard, and
/// [`Slider::track_area`] and [`Slider::thumb_areas`] give the cells for hit testing.
#[derive(Clone, Debug, PartialEq)]
pub struct Slider<'a> {
    min: f64,
    max: f64,
    low: f64,
    high: Option<f64>,
    step: Option<f64>,
    knob: bool,
    label: Option<Cow<'a, str>>,
    show_value: bool,
    precision: usize,
    state: WidgetState,
    style: Option<Style>,
}

impl<'a> Slider<'a> {
    /// A slider over `min..=max` with its value at the minimum. Bounds given the wrong way round
    /// are swapped, and a NaN bound counts as zero.
    #[must_use]
    pub fn new(min: f64, max: f64) -> Self {
        let (min, max) = (clean(min), clean(max));
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        Self {
            min,
            max,
            low: min,
            high: None,
            step: None,
            knob: false,
            label: None,
            show_value: false,
            precision: 0,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// A single value, clamped into the bounds. A NaN value counts as the minimum.
    #[must_use]
    pub fn value(mut self, value: f64) -> Self {
        self.low = self.clamp(value);
        self.high = None;
        self
    }

    /// A range between two values, in either order, clamped into the bounds.
    #[must_use]
    pub fn range(mut self, first: f64, second: f64) -> Self {
        let (first, second) = (self.clamp(first), self.clamp(second));
        self.low = first.min(second);
        self.high = Some(first.max(second));
        self
    }

    /// Values snap to multiples of `step` above the minimum. Zero, negative, and NaN steps are
    /// ignored.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = (step > 0.0 && step.is_finite()).then_some(step);
        self
    }

    /// Draws a `●` on the cell of each end of the filled part.
    #[must_use]
    pub const fn knob(mut self, knob: bool) -> Self {
        self.knob = knob;
        self
    }

    /// Text before the track, one cell away from it.
    #[must_use]
    pub fn label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows the value (or `low–high` for a range) after the track, with `precision` decimals.
    #[must_use]
    pub const fn show_value(mut self, show: bool, precision: usize) -> Self {
        self.show_value = show;
        self.precision = precision;
        self
    }

    /// Interaction state: focused uses the focus ring color and a bold label, disabled dims.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws everything in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    fn clamp(&self, value: f64) -> f64 {
        if value.is_nan() {
            self.min
        } else {
            value.clamp(self.min, self.max)
        }
    }

    fn snap(&self, value: f64) -> f64 {
        let value = self.clamp(value);
        match self.step {
            Some(step) => self.clamp(self.min + ((value - self.min) / step).round() * step),
            None => value,
        }
    }

    /// The value `steps` whole steps from `value` (negative goes down), snapped and clamped. The
    /// default step is a hundredth of the span.
    #[must_use]
    pub fn nudge(&self, value: f64, steps: i32) -> f64 {
        let step = self.step.unwrap_or((self.max - self.min) / 100.0);
        self.snap(self.clamp(value) + f64::from(steps) * step)
    }

    /// The value at `column` of a track at `track`: the first cell is the minimum and the last
    /// the maximum, snapped to the step. Columns outside the track give the nearest end.
    #[must_use]
    pub fn value_at(&self, track: Rect, column: u16) -> f64 {
        if track.width <= 1 {
            return self.min;
        }
        let last = track.width - 1;
        let cell = column.saturating_sub(track.x).min(last);
        let fraction = f64::from(cell) / f64::from(last);
        self.snap(self.min + fraction * (self.max - self.min))
    }

    fn texts(&self) -> (String, String) {
        let label = self
            .label
            .as_ref()
            .map_or_else(String::new, |label| format!("{label} "));
        let value = if self.show_value {
            let precision = self.precision;
            let (low, high) = (self.low, self.high);
            match high {
                Some(high) => format!(" {low:.precision$}–{high:.precision$}"),
                None => format!(" {low:.precision$}"),
            }
        } else {
            String::new()
        };
        (label, value)
    }

    fn widest_value(&self) -> usize {
        if !self.show_value {
            return 0;
        }
        let precision = self.precision;
        let (min, max) = (self.min, self.max);
        let one = format!(" {min:.precision$}")
            .len()
            .max(format!(" {max:.precision$}").len());
        if self.high.is_some() {
            one * 2 + width("–") - 1
        } else {
            one
        }
    }

    /// The cells the track covers when the slider is rendered into `area`. Empty when the label
    /// and value text leave no room.
    #[must_use]
    pub fn track_area(&self, area: Rect) -> Rect {
        if area.width == 0 || area.height == 0 {
            return Rect::new(area.x, area.y, 0, 0);
        }
        let (label, _) = self.texts();
        let before = u16::try_from(width(&label)).unwrap_or(u16::MAX);
        let after = u16::try_from(self.widest_value()).unwrap_or(u16::MAX);
        let room = area.width.saturating_sub(before).saturating_sub(after);
        Rect::new(area.x.saturating_add(before), area.y, room, 1)
    }

    fn half_position(&self, value: f64, total: u16) -> u16 {
        let span = self.max - self.min;
        let fraction = ((value - self.min) / span.max(f64::EPSILON)).clamp(0.0, 1.0);
        let distance = |half: u16| (f64::from(half) / f64::from(total) - fraction).abs();
        (0..=total)
            .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
            .unwrap_or(0)
    }

    fn ends(&self, track: Rect) -> (u16, u16) {
        let total = track.width.saturating_mul(2);
        let low = self.half_position(self.low, total);
        match self.high {
            Some(high) => (low, self.half_position(high, total)),
            None => (0, low),
        }
    }

    /// The cells of each end of the filled part as `(index, rect)`: index 0 is the single value
    /// or the low end, index 1 the high end of a range. Each is one cell, for hit testing.
    #[must_use]
    pub fn thumb_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        let track = self.track_area(area);
        if track.width == 0 {
            return Vec::new();
        }
        let (low, high) = self.ends(track);
        let cell = |half: u16| track.x + (half / 2).min(track.width - 1);
        let mut thumbs = Vec::new();
        if self.high.is_some() {
            let last = if high > low { high - 1 } else { low };
            thumbs.push((0, Rect::new(cell(low), track.y, 1, 1)));
            thumbs.push((1, Rect::new(cell(last), track.y, 1, 1)));
        } else {
            thumbs.push((0, Rect::new(cell(high.saturating_sub(1)), track.y, 1, 1)));
        }
        thumbs
    }

    fn styles(&self, cx: &RenderCx<'_>) -> (Style, Style, Style, Style) {
        if let Some(style) = self.style {
            return (style, style, style, style);
        }
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let dim = Attrs::new().set_dim(disabled);
        let text = cx.token(Token::TextPrimary);
        (
            Style::new().fg(cx.token(Token::Accent)).with_attrs(dim),
            Style::new().fg(text).with_attrs(Attrs::new().set_dim(true)),
            Style::new()
                .fg(if focused {
                    cx.token(Token::FocusRing)
                } else {
                    cx.token(Token::Accent)
                })
                .with_attrs(dim.set_bold(focused)),
            Style::new().fg(text).with_attrs(dim.set_bold(focused)),
        )
    }
}

fn clean(value: f64) -> f64 {
    if value.is_nan() { 0.0 } else { value }
}

impl Widget for Slider<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let (filled, empty, knob, text) = self.styles(cx);
        let (label, value) = self.texts();
        cx.put_str(
            area.x,
            area.y,
            clip_display_prefix(&label, area.width),
            text,
        );
        let track = self.track_area(area);
        let (low, high) = self.ends(track);
        for cell in 0..track.width {
            let (left, right) = (cell * 2, cell * 2 + 1);
            let in_fill = |half: u16| half >= low && half < high;
            let (glyph, style) = match (in_fill(left), in_fill(right)) {
                (true, true) => (FULL, filled),
                (true, false) => (LEFT_HALF, filled),
                (false, true) => (RIGHT_HALF, filled),
                (false, false) => (EMPTY, empty),
            };
            cx.put_str(track.x + cell, track.y, glyph, style);
        }
        if self.knob {
            for (_, rect) in self.thumb_areas(area) {
                cx.put_str(rect.x, rect.y, KNOB, knob);
            }
        }
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
