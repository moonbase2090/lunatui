// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod input;
mod output;
mod screen;
use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};
use lunatui_core::{CellWidth, Color, Rect, Size, Style};
use lunatui_term::Event;
use lunatui_theme::Token;
pub use screen::{TerminalCell, TerminalScreen};

/// An embedded terminal view for a separately owned VT screen.
///
/// The app feeds output and polls its PTY independently. Default colors use theme
/// tokens; child SGR colors remain intact. Native hosts receive ordinary Grid cells,
/// and the ANSI encoder renders those same cells. Focus shows a local cursor and
/// permits keyboard/paste encoding. Disabled output is dim.
#[derive(Clone, Copy, Debug)]
pub struct PseudoTerminal<'a> {
    screen: &'a TerminalScreen,
    state: WidgetState,
}
impl<'a> PseudoTerminal<'a> {
    /// Borrows a screen without taking process or emulator ownership.
    #[must_use]
    pub const fn new(screen: &'a TerminalScreen) -> Self {
        Self {
            screen,
            state: WidgetState::Normal,
        }
    }
    /// Sets focus or disabled appearance. Only Focused forwards input.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }
    /// Encodes a focused keyboard or paste event for the child's current modes.
    ///
    /// Returns None for unfocused/disabled views, releases, mouse/focus events and
    /// unsupported keys. Paste boundaries require the child's bracketed-paste mode.
    /// These bytes use the child protocol instead of the outer event encoding.
    #[must_use]
    pub fn input(&self, event: &Event) -> Option<Vec<u8>> {
        (self.state == WidgetState::Focused)
            .then(|| input::encode(self.screen, event))
            .flatten()
    }
}
impl Widget for PseudoTerminal<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let foreground = cx.token(Token::TextPrimary);
        let background = cx.token(Token::Surface0);
        let disabled = self.state == WidgetState::Disabled;
        let base = Style::new().fg(foreground).bg(background);
        cx.fill(area, base.with_attrs(base.attrs.set_dim(disabled)), ' ');
        let columns = area.width.min(self.screen.size().cols());
        let rows = area.height.min(self.screen.size().rows());
        let cursor = self.screen.cursor();
        let visible = self.state == WidgetState::Focused
            && self.screen.cursor_visible()
            && self.screen.scrollback() == 0;
        for row in 0..rows {
            for column in 0..columns {
                let Some(cell) = self.screen.cell(column, row) else {
                    continue;
                };
                if cell.width == CellWidth::Continuation {
                    continue;
                }
                let mut style = cell.style;
                if style.fg == Color::Default {
                    style.fg = foreground;
                }
                if style.bg == Color::Default {
                    style.bg = background;
                }
                #[cfg(feature = "color-profiles")]
                if let Some(profile) = cx.color_profile() {
                    style.fg = profile.downsample(style.fg);
                    style.bg = profile.downsample(style.bg);
                }
                style.attrs = style.attrs.set_dim(style.attrs.dim() || disabled);
                let cursor_here = cursor.1 == row
                    && (cursor.0 == column
                        || (cell.width == CellWidth::Two && cursor.0 == column + 1));
                if visible && cursor_here {
                    style = style.fg(background).bg(cx.token(Token::FocusRing));
                    style.attrs = style.attrs.set_reverse(false);
                }
                let text = if cell.text.is_empty()
                    || (cell.width == CellWidth::Two && column + 1 >= columns)
                {
                    " "
                } else {
                    cell.text
                };
                cx.put_str(area.x + column, area.y + row, text, style);
            }
        }
    }
    fn measure(&self, avail: Size) -> Size {
        Size::new(self.screen.size().cols(), self.screen.size().rows()).clamp(avail)
    }
}
