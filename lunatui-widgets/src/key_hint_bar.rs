// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_runtime::keymap::Keymap;
use lunatui_theme::Token;

use crate::{RenderCx, Widget};

/// A one-line summary of the commands available in the active keymap.
///
/// Build this widget from the keymap for the current screen or mode. The first
/// chord registered for each command is shown, in registration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyHintBar<'a> {
    keymap: &'a Keymap,
    gap: u16,
}

impl<'a> KeyHintBar<'a> {
    /// Hints from the keymap active for this screen or mode.
    #[must_use]
    pub fn new(keymap: &'a Keymap) -> Self {
        Self { keymap, gap: 2 }
    }

    /// Number of blank cells between command hints.
    #[must_use]
    pub const fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }
}

impl Widget for KeyHintBar<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let key_style = Style::new().fg(cx.token(Token::Accent));
        let title_style = Style::new().fg(cx.token(Token::TextPrimary));
        let end = area.x.saturating_add(area.width);
        let mut x = area.x;

        for command in self.keymap.commands() {
            let Some(chord) = command.chords().first() else {
                continue;
            };
            if x >= end {
                break;
            }

            let key = format!("[{chord}]");
            let key_draw = clip_display_prefix(&key, end - x);
            cx.put_str(x, area.y, key_draw, key_style);
            x = x.saturating_add(u16::try_from(width(key_draw)).unwrap_or(u16::MAX));
            if x >= end {
                break;
            }

            cx.put_str(x, area.y, " ", title_style);
            x = x.saturating_add(1);
            if x >= end {
                break;
            }

            let title_draw = clip_display_prefix(command.title(), end - x);
            cx.put_str(x, area.y, title_draw, title_style);
            x = x.saturating_add(u16::try_from(width(title_draw)).unwrap_or(u16::MAX));
            if x < end {
                let gap = self.gap.min(end - x);
                for _ in 0..gap {
                    cx.put_str(x, area.y, " ", title_style);
                    x = x.saturating_add(1);
                }
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, u16::from(avail.width > 0 && avail.height > 0)).clamp(avail)
    }
}
