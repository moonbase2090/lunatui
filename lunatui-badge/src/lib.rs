// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_ui_tokens::{Role, Tone, surface};
#[cfg(feature = "graphite-text")]
use lunatui_widgets::graphite::Face;
use lunatui_widgets::{RenderCx, Widget};

#[cfg(feature = "graphite-text")]
mod graphite;

/// How a [`Badge`] is filled.
///
/// Downstream code must keep a wildcard arm, because new values can be added:
///
/// ```compile_fail,E0004
/// use lunatui_badge::Look;
///
/// fn covers_every_value(value: Look) -> u8 {
///     match value {
///         Look::Filled => 0,
///         Look::Subtle => 0,
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Look {
    /// Tone color as the background, bold text on top.
    #[default]
    Filled,
    /// Raised surface with the tone color as text.
    Subtle,
}

/// A state mark drawn before a [`Badge`]'s text.
///
/// In cells it is a glyph; in Graphite mode it is drawn as a shape.
///
/// Downstream code must keep a wildcard arm, because new values can be added:
///
/// ```compile_fail,E0004
/// use lunatui_badge::Marker;
///
/// fn covers_every_value(value: Marker) -> u8 {
///     match value {
///         Marker::Dot | Marker::Ring | Marker::Diamond => 0,
///         Marker::Cross | Marker::Square | Marker::Bang => 0,
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Marker {
    /// A filled circle, `●` in cells.
    Dot,
    /// An open circle, `○` in cells.
    Ring,
    /// A filled diamond, `◆` in cells.
    Diamond,
    /// A cross, `×` in cells.
    Cross,
    /// A filled square, `■` in cells.
    Square,
    /// An exclamation mark, `!` in cells.
    Bang,
}

impl Marker {
    /// The glyph drawn for this marker in cells.
    #[must_use]
    pub const fn glyph(self) -> char {
        match self {
            Self::Dot => '●',
            Self::Ring => '○',
            Self::Diamond => '◆',
            Self::Cross => '×',
            Self::Square => '■',
            Self::Bang => '!',
        }
    }
}

/// One-line label with a space of padding on each side.
///
/// In Graphite mode the badge is drawn as a rounded pill over the cells it covers, with its
/// marker and proportional text centered. A subtle badge's pill is a tint of its tone. The badge
/// covers the same cells in both modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Badge<'a> {
    text: &'a str,
    tone: Tone,
    look: Look,
    marker: Option<Marker>,
}

impl<'a> Badge<'a> {
    /// Neutral filled badge.
    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self {
            text,
            tone: Tone::Neutral,
            look: Look::Filled,
            marker: None,
        }
    }

    /// Draws `marker` before the text.
    #[must_use]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = Some(marker);
        self
    }

    /// Sets the tone.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets how the badge is filled.
    #[must_use]
    pub const fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }
}

impl Badge<'_> {
    fn style(&self, cx: &RenderCx<'_>) -> Style {
        let neutral = self.tone == Tone::Neutral;
        match self.look {
            Look::Filled if neutral => {
                surface(cx, Role::Background).with_attrs(Attrs::new().set_bold(true))
            }
            Look::Filled => self.tone.style(cx),
            Look::Subtle => {
                let fg = if neutral {
                    Role::Foreground.token()
                } else {
                    self.tone.fill()
                };
                surface(cx, Role::Card)
                    .fg(cx.token(fg))
                    .with_attrs(Attrs::new().set_bold(true))
            }
        }
    }

    fn label(&self) -> String {
        match self.marker {
            Some(marker) => format!(" {} {} ", marker.glyph(), self.text),
            None => format!(" {} ", self.text),
        }
    }

    #[cfg(feature = "graphite-text")]
    fn paint(&self, area: Rect, shown: &str, style: Style, cx: &mut RenderCx<'_>) -> bool {
        let cells = u16::try_from(width(shown)).unwrap_or(u16::MAX);
        let area = Rect::new(area.x, area.y, cells, 1);
        let tinted = self.look == Look::Subtle && self.tone != Tone::Neutral;
        let (fill, face) = match self.look {
            Look::Subtle if tinted => (style.fg, Face::Regular),
            Look::Subtle => (style.bg, Face::Regular),
            _ => (style.bg, Face::SemiBold),
        };
        cx.replace_with_pixels(area, |px| {
            graphite::chip(px, self.text, self.marker, style.fg, fill, tinted, face);
        })
    }
}

impl Widget for Badge<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let style = self.style(cx);
        let label = self.label();
        let shown = clip_display_prefix(&label, area.width);
        #[cfg(feature = "graphite-text")]
        if self.paint(area, shown, style, cx) {
            return;
        }
        cx.put_str(area.x, area.y, shown, style);
    }

    fn measure(&self, avail: Size) -> Size {
        let w = u16::try_from(width(&self.label())).unwrap_or(u16::MAX);
        Size::new(w, 1).clamp(avail)
    }
}

/// A key or chord drawn as raised chips, one per key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kbd<'a> {
    keys: &'a [&'a str],
}

impl<'a> Kbd<'a> {
    /// Chips for `keys`, in order, such as `["Ctrl", "K"]`.
    #[must_use]
    pub const fn new(keys: &'a [&'a str]) -> Self {
        Self { keys }
    }
}

impl Kbd<'_> {
    fn slots(&self, area: Rect) -> Vec<(&str, u16, u16)> {
        let mut slots = Vec::new();
        let mut x = 0_u16;
        for (index, key) in self.keys.iter().enumerate() {
            if index > 0 {
                if x >= area.width {
                    break;
                }
                x += 1;
            }
            let room = area.width.saturating_sub(x);
            if room == 0 {
                break;
            }
            let label = format!(" {key} ");
            let cells = u16::try_from(width(clip_display_prefix(&label, room))).unwrap_or(u16::MAX);
            slots.push((*key, x, cells));
            x = x.saturating_add(cells);
        }
        slots
    }

    #[cfg(feature = "graphite-text")]
    fn paint(&self, area: Rect, chip: Style, cx: &mut RenderCx<'_>) -> bool {
        let slots = self.slots(area);
        let Some(&(_, last, cells)) = slots.last() else {
            return false;
        };
        let edge = cx.token(Role::Border.token());
        let run = Rect::new(area.x, area.y, last.saturating_add(cells), 1);
        cx.replace_with_pixels(run, |px| {
            for &(key, column, columns) in &slots {
                graphite::keycap(px, key, (column, columns), chip, edge);
            }
        })
    }
}

impl Widget for Kbd<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.height == 0 {
            return;
        }
        let chip = surface(cx, Role::Card).with_attrs(Attrs::new().set_bold(true));
        #[cfg(feature = "graphite-text")]
        if self.paint(area, chip, cx) {
            return;
        }
        for (key, column, cells) in self.slots(area) {
            let label = format!(" {key} ");
            let shown = clip_display_prefix(&label, cells);
            cx.put_str(area.x.saturating_add(column), area.y, shown, chip);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let chips: usize = self.keys.iter().map(|k| width(k) + 2).sum();
        let gaps = self.keys.len().saturating_sub(1);
        Size::new(u16::try_from(chips + gaps).unwrap_or(u16::MAX), 1).clamp(avail)
    }
}
