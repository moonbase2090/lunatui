// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::output::OutputFilter;
use lunatui_core::{Attrs, CellWidth, Color, Style, UnderlineStyle};
use lunatui_term::PtySize;
use std::fmt;

const HISTORY_ROWS: usize = 256;
const MAX_REPLIES: usize = 4096;

/// A borrowed emulator cell in Lunatui colors and attributes.
///
/// Downstream code cannot list every field without `..`, because fields can be added:
///
/// ```compile_fail,E0638
/// use lunatui_widgets::TerminalCell;
///
/// fn destructures(value: TerminalCell<'_>) {
///     let TerminalCell { text: _, style: _, width: _ } = value;
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TerminalCell<'a> {
    /// Text stored by the VT parser, including supported combining characters.
    pub text: &'a str,
    /// Child-requested style; default colors resolve through the widget's theme.
    pub style: Style,
    /// One cell, a wide lead, or its continuation.
    pub width: CellWidth,
}

/// A bounded VT screen separate from the child process and outer terminal session.
///
/// The approved parser handles cursor movement, SGR, erasure, alternate screens,
/// scroll regions and UTF-8. History holds at most 256 rows. Control strings over
/// 4096 bytes are discarded as a whole. Clipboard and graphics requests have no
/// host effects. The parser uses conventional terminal character widths.
pub struct TerminalScreen {
    parser: vt100::Parser<Responses>,
    filter: OutputFilter,
    size: PtySize,
}

impl fmt::Debug for TerminalScreen {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("TerminalScreen")
            .field("size", &self.size)
            .field("title", &self.title())
            .finish_non_exhaustive()
    }
}

impl TerminalScreen {
    /// Creates a blank validated screen with bounded scrollback.
    #[must_use]
    pub fn new(size: PtySize) -> Self {
        Self {
            parser: vt100::Parser::new_with_callbacks(
                size.rows(),
                size.cols(),
                HISTORY_ROWS,
                Responses::default(),
            ),
            filter: OutputFilter::default(),
            size,
        }
    }
    /// Feeds child output. UTF-8 and escape sequences may span calls.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.filter.feed(&mut self.parser, bytes);
    }
    /// Current validated dimensions.
    #[must_use]
    pub const fn size(&self) -> PtySize {
        self.size
    }
    /// Resizes the emulator after a successful child PTY resize.
    pub fn resize(&mut self, size: PtySize) {
        self.parser.screen_mut().set_size(size.rows(), size.cols());
        self.size = size;
    }
    /// Cursor column and row, zero based, on the live screen.
    #[must_use]
    pub fn cursor(&self) -> (u16, u16) {
        let (row, col) = self.parser.screen().cursor_position();
        (col.min(self.size.cols() - 1), row.min(self.size.rows() - 1))
    }
    /// Whether the child requests a visible cursor.
    #[must_use]
    pub fn cursor_visible(&self) -> bool {
        !self.parser.screen().hide_cursor()
    }
    /// Whether the alternate screen is active.
    #[must_use]
    pub fn is_alternate(&self) -> bool {
        self.parser.screen().alternate_screen()
    }
    /// The local child window title, limited to 256 printable characters.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.parser.callbacks().title
    }
    /// Scrolls back this many rows, clamped to available history. Zero returns to live output.
    pub fn set_scrollback(&mut self, rows: usize) {
        self.parser.screen_mut().set_scrollback(rows);
    }
    /// Current history offset.
    #[must_use]
    pub fn scrollback(&self) -> usize {
        self.parser.screen().scrollback()
    }
    /// Visible text without trailing blanks, as recorded by the parser.
    #[must_use]
    pub fn contents(&self) -> String {
        self.parser.screen().contents()
    }
    /// Visible row text without trailing blanks, or empty outside the screen.
    #[must_use]
    pub fn row_text(&self, row: u16) -> String {
        self.parser
            .screen()
            .rows(0, self.size.cols())
            .nth(usize::from(row))
            .unwrap_or_default()
            .trim_end()
            .to_owned()
    }
    /// Borrows one visible cell, including history when scrolled back.
    #[must_use]
    pub fn cell(&self, column: u16, row: u16) -> Option<TerminalCell<'_>> {
        let cell = self.parser.screen().cell(row, column)?;
        Some(TerminalCell {
            text: cell.contents(),
            style: cell_style(cell),
            width: if cell.is_wide_continuation() {
                CellWidth::Continuation
            } else if cell.is_wide() {
                CellWidth::Two
            } else {
                CellWidth::One
            },
        })
    }
    /// Takes pending device-status replies to queue back to the child, at most 4096 bytes.
    #[must_use]
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.parser.callbacks_mut().replies)
    }
    pub(super) fn application_cursor(&self) -> bool {
        self.parser.screen().application_cursor()
    }
    pub(super) fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }
}

fn color(value: vt100::Color) -> Color {
    match value {
        vt100::Color::Default => Color::Default,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
fn cell_style(cell: &vt100::Cell) -> Style {
    let attrs = Attrs::new()
        .set_bold(cell.bold())
        .set_dim(cell.dim())
        .set_italic(cell.italic())
        .set_reverse(cell.inverse())
        .set_underline(if cell.underline() {
            UnderlineStyle::Single
        } else {
            UnderlineStyle::None
        });
    Style::new()
        .fg(color(cell.fgcolor()))
        .bg(color(cell.bgcolor()))
        .with_attrs(attrs)
}
#[derive(Default)]
struct Responses {
    title: String,
    replies: Vec<u8>,
}
impl Responses {
    fn reply(&mut self, bytes: &[u8]) {
        if self.replies.len() + bytes.len() <= MAX_REPLIES {
            self.replies.extend_from_slice(bytes);
        }
    }
}
impl vt100::Callbacks for Responses {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = String::from_utf8_lossy(title)
            .chars()
            .filter(|ch| !ch.is_control())
            .take(256)
            .collect();
    }
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        first: Option<u8>,
        second: Option<u8>,
        params: &[&[u16]],
        code: char,
    ) {
        if first.is_some() || second.is_some() {
            return;
        }
        let value = params
            .first()
            .and_then(|values| values.first())
            .copied()
            .unwrap_or(0);
        match (code, value) {
            ('n', 5) => self.reply(b"\x1b[0n"),
            ('n', 6) => {
                let (row, col) = screen.cursor_position();
                let (rows, cols) = screen.size();
                self.reply(
                    format!("\x1b[{};{}R", row.min(rows - 1) + 1, col.min(cols - 1) + 1).as_bytes(),
                );
            }
            ('c', 0) => self.reply(b"\x1b[?1;2c"),
            _ => {}
        }
    }
}
