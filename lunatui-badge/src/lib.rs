// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, clip_display_prefix, width};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{RenderCx, Widget};

/// How a [`Badge`] is filled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    /// Tone color as the background, bold text on top.
    #[default]
    Filled,
    /// Raised surface with the tone color as text.
    Subtle,
}

/// One-line label with a space of padding on each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Badge<'a> {
    text: &'a str,
    tone: Tone,
    look: Look,
}

impl<'a> Badge<'a> {
    /// Neutral filled badge.
    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self {
            text,
            tone: Tone::Neutral,
            look: Look::Filled,
        }
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

impl Widget for Badge<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let neutral = self.tone == Tone::Neutral;
        let style = match self.look {
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
        };
        let label = format!(" {} ", self.text);
        cx.put_str(
            area.x,
            area.y,
            clip_display_prefix(&label, area.width),
            style,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        let w = u16::try_from(width(self.text)).unwrap_or(u16::MAX);
        Size::new(w.saturating_add(2), 1).clamp(avail)
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

impl Widget for Kbd<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.height == 0 {
            return;
        }
        let chip = surface(cx, Role::Card).with_attrs(Attrs::new().set_bold(true));
        let end = area.x.saturating_add(area.width);
        let mut x = area.x;
        for (index, key) in self.keys.iter().enumerate() {
            if index > 0 {
                if x >= end {
                    return;
                }
                x += 1;
            }
            let label = format!(" {key} ");
            let room = end.saturating_sub(x);
            if room == 0 {
                return;
            }
            let shown = clip_display_prefix(&label, room);
            cx.put_str(x, area.y, shown, chip);
            x = x.saturating_add(u16::try_from(width(shown)).unwrap_or(u16::MAX));
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let chips: usize = self.keys.iter().map(|k| width(k) + 2).sum();
        let gaps = self.keys.len().saturating_sub(1);
        Size::new(u16::try_from(chips + gaps).unwrap_or(u16::MAX), 1).clamp(avail)
    }
}
