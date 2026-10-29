// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const MIN_TRACK: u16 = 2;
const DEFAULT_TRACK: u16 = 4;

/// An on/off switch with an optional label: `━━━●` when on, `●───` when off.
///
/// The track is the accent color when on and dimmed text when off; the knob is the ok color when
/// on. A focused switch draws its knob in the focus ring color and its label bold, and a disabled
/// one is dimmed. [`Toggle::hit_area`] gives the cells a click should reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toggle {
    on: bool,
    label: String,
    track: u16,
    state: WidgetState,
    style: Option<Style>,
}

impl Toggle {
    /// A switch that is `on` or off.
    #[must_use]
    pub fn new(on: bool) -> Self {
        Self {
            on,
            label: String::new(),
            track: DEFAULT_TRACK,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// Text drawn after the switch, one cell away.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Width of the track in cells, at least two. The default is four.
    #[must_use]
    pub fn track_width(mut self, cells: u16) -> Self {
        self.track = cells.max(MIN_TRACK);
        self
    }

    /// Interaction state the switch paints itself for.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws the whole switch and its label in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// The same switch turned the other way.
    #[must_use]
    pub fn toggled(mut self) -> Self {
        self.on = !self.on;
        self
    }

    /// Whether the switch is on.
    #[must_use]
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The cells the switch and its label cover when rendered at the top left of `area`, for
    /// hit testing. Empty when `area` is.
    #[must_use]
    pub fn hit_area(&self, area: Rect) -> Rect {
        if area.width == 0 || area.height == 0 {
            return Rect::new(area.x, area.y, 0, 0);
        }
        let label = u16::try_from(width(&self.label)).unwrap_or(u16::MAX);
        let wanted = if self.label.is_empty() {
            self.track
        } else {
            self.track.saturating_add(1).saturating_add(label)
        };
        Rect::new(area.x, area.y, wanted.min(area.width), 1)
    }

    fn styles(&self, cx: &RenderCx<'_>) -> (Style, Style, Style) {
        if let Some(style) = self.style {
            return (style, style, style);
        }
        let dim = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let attrs = Attrs::new().set_dim(dim);
        let track = if self.on {
            cx.token(Token::Accent)
        } else {
            cx.token(Token::TextPrimary)
        };
        let track_attrs = if self.on { attrs } else { attrs.set_dim(true) };
        let knob = if focused {
            cx.token(Token::FocusRing)
        } else if self.on {
            cx.token(Token::Ok)
        } else {
            cx.token(Token::TextPrimary)
        };
        let label_attrs = attrs.set_bold(focused);
        (
            Style::new().fg(track).with_attrs(track_attrs),
            Style::new().fg(knob).with_attrs(attrs.set_bold(focused)),
            Style::new()
                .fg(cx.token(Token::TextPrimary))
                .with_attrs(label_attrs),
        )
    }
}

impl Widget for Toggle {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let (track_style, knob_style, label_style) = self.styles(cx);
        let knob_at = if self.on { self.track - 1 } else { 0 };
        let rail = if self.on { "━" } else { "─" };
        let end = area.x.saturating_add(area.width);
        let mut x = area.x;
        for cell in 0..self.track {
            if x >= end {
                return;
            }
            if cell == knob_at {
                cx.put_str(x, area.y, "●", knob_style);
            } else {
                cx.put_str(x, area.y, rail, track_style);
            }
            x = x.saturating_add(1);
        }
        if self.label.is_empty() {
            return;
        }
        let start = x.saturating_add(1);
        if start >= end {
            return;
        }
        cx.put_str(
            start,
            area.y,
            clip_display_prefix(&self.label, end - start),
            label_style,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let area = self.hit_area(Rect::new(0, 0, avail.width, avail.height));
        Size::new(area.width, 1)
    }
}
