// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use lunatui_core::{Color, Style, UnderlineStyle, width};

const SCROLLBACK_LIMIT: usize = 2000;
const MAX_PARAMS: usize = 16;
const MAX_OSC: usize = 256;
const TAB_STOP: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScreenCell {
    pub(super) ch: char,
    pub(super) style: Style,
    pub(super) tail: bool,
}

impl ScreenCell {
    const fn blank(style: Style) -> Self {
        Self {
            ch: ' ',
            style,
            tail: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Ground,
    Escape,
    Designate,
    Csi,
    Osc,
    OscEscape,
    Ignore,
    IgnoreEscape,
}

#[derive(Clone, Debug)]
struct Saved {
    cells: Vec<ScreenCell>,
    x: usize,
    y: usize,
    pending_wrap: bool,
}

/// A terminal screen that a byte stream from a child process draws into.
///
/// Feed it the child's output with [`feed`](Self::feed). It keeps a cell grid, the cursor, the
/// scroll region, an alternate screen and a bounded scrollback, and understands the common
/// escape sequences: cursor movement, erase, insert and delete, scrolling, SGR colors and
/// attributes (16, 256 and truecolor), the alternate screen and cursor visibility. Zero-width
/// characters are dropped and unknown sequences are ignored.
#[derive(Clone, Debug)]
pub struct TerminalScreen {
    cols: usize,
    rows: usize,
    cells: Vec<ScreenCell>,
    x: usize,
    y: usize,
    pending_wrap: bool,
    pen: Style,
    top: usize,
    bottom: usize,
    saved_cursor: (usize, usize, Style),
    alternate: Option<Saved>,
    cursor_visible: bool,
    autowrap: bool,
    scrollback: VecDeque<Vec<ScreenCell>>,
    title: String,
    mode: Mode,
    params: Vec<u32>,
    private: bool,
    intermediate: bool,
    osc: Vec<u8>,
    utf8: Vec<u8>,
}

impl TerminalScreen {
    /// A blank screen of `cols` by `rows` cells, at least one each.
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = usize::from(cols.max(1));
        let rows = usize::from(rows.max(1));
        Self {
            cols,
            rows,
            cells: vec![ScreenCell::blank(Style::new()); cols * rows],
            x: 0,
            y: 0,
            pending_wrap: false,
            pen: Style::new(),
            top: 0,
            bottom: rows - 1,
            saved_cursor: (0, 0, Style::new()),
            alternate: None,
            cursor_visible: true,
            autowrap: true,
            scrollback: VecDeque::new(),
            title: String::new(),
            mode: Mode::Ground,
            params: Vec::new(),
            private: false,
            intermediate: false,
            osc: Vec::new(),
            utf8: Vec::new(),
        }
    }

    /// Columns.
    #[must_use]
    pub fn cols(&self) -> u16 {
        u16::try_from(self.cols).unwrap_or(u16::MAX)
    }

    /// Rows.
    #[must_use]
    pub fn rows(&self) -> u16 {
        u16::try_from(self.rows).unwrap_or(u16::MAX)
    }

    /// The cursor as `(column, row)`, zero based.
    #[must_use]
    pub fn cursor(&self) -> (u16, u16) {
        (
            u16::try_from(self.x).unwrap_or(u16::MAX),
            u16::try_from(self.y).unwrap_or(u16::MAX),
        )
    }

    /// Whether the child wants the cursor shown.
    #[must_use]
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// The window title the child last set, empty if none.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Whether the alternate screen is showing, as full-screen programs do.
    #[must_use]
    pub fn is_alternate(&self) -> bool {
        self.alternate.is_some()
    }

    /// Lines scrolled off the top of the main screen and still kept.
    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// The text of screen row `row` with trailing blanks trimmed, empty when out of range.
    #[must_use]
    pub fn row_text(&self, row: u16) -> String {
        let row = usize::from(row);
        if row >= self.rows {
            return String::new();
        }
        let line = &self.cells[row * self.cols..(row + 1) * self.cols];
        line_text(line)
    }

    /// The text of the whole screen, one line per row, trailing blanks trimmed.
    #[must_use]
    pub fn text(&self) -> String {
        (0..self.rows)
            .map(|row| self.row_text(u16::try_from(row).unwrap_or(u16::MAX)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(super) fn cell(&self, x: usize, y: usize) -> Option<ScreenCell> {
        if x < self.cols && y < self.rows {
            self.cells.get(y * self.cols + x).copied()
        } else {
            None
        }
    }

    pub(super) fn history_cell(&self, back: usize, x: usize) -> Option<ScreenCell> {
        let len = self.scrollback.len();
        let line = self.scrollback.get(len.checked_sub(back + 1)?)?;
        Some(line.get(x).copied().unwrap_or(ScreenCell::blank(Style::new())))
    }

    /// Resizes to `cols` by `rows`, keeping the top-left content and clamping the cursor.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = usize::from(cols.max(1));
        let rows = usize::from(rows.max(1));
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cells = resized(&self.cells, self.cols, self.rows, cols, rows);
        if let Some(saved) = &mut self.alternate {
            saved.cells = resized(&saved.cells, self.cols, self.rows, cols, rows);
        }
        self.scrollback.iter_mut().for_each(|line| line.truncate(cols));
        self.cols = cols;
        self.rows = rows;
        self.top = 0;
        self.bottom = rows - 1;
        self.x = self.x.min(cols - 1);
        self.y = self.y.min(rows - 1);
        self.pending_wrap = false;
    }

    /// Draws `bytes` from the child onto the screen. A sequence may be split across calls.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.step(byte);
        }
    }

    fn step(&mut self, byte: u8) {
        match self.mode {
            Mode::Ground => self.ground(byte),
            Mode::Escape => self.escape(byte),
            Mode::Designate => self.mode = Mode::Ground,
            Mode::Csi => self.csi_byte(byte),
            Mode::Osc => match byte {
                0x07 => self.finish_osc(),
                0x1b => self.mode = Mode::OscEscape,
                _ if self.osc.len() < MAX_OSC => self.osc.push(byte),
                _ => {}
            },
            Mode::OscEscape => {
                if byte == b'\\' {
                    self.finish_osc();
                } else {
                    self.mode = Mode::Ground;
                    self.escape(byte);
                }
            }
            Mode::Ignore => match byte {
                0x07 => self.mode = Mode::Ground,
                0x1b => self.mode = Mode::IgnoreEscape,
                _ => {}
            },
            Mode::IgnoreEscape => {
                self.mode = Mode::Ground;
                if byte != b'\\' {
                    self.escape(byte);
                }
            }
        }
    }

    fn ground(&mut self, byte: u8) {
        if !self.utf8.is_empty() {
            if byte & 0xC0 == 0x80 {
                self.utf8.push(byte);
                if self.utf8.len() == utf8_len(self.utf8[0]) {
                    let decoded = std::str::from_utf8(&self.utf8)
                        .ok()
                        .and_then(|text| text.chars().next());
                    self.utf8.clear();
                    self.print(decoded.unwrap_or('\u{FFFD}'));
                }
                return;
            }
            self.utf8.clear();
            self.print('\u{FFFD}');
        }
        match byte {
            0x1b => self.mode = Mode::Escape,
            0x07 | 0x00 => {}
            0x08 => {
                self.pending_wrap = false;
                self.x = self.x.saturating_sub(1);
            }
            0x09 => self.tab(),
            0x0a..=0x0c => self.line_feed(),
            0x0d => {
                self.pending_wrap = false;
                self.x = 0;
            }
            0x20..=0x7e => self.print(char::from(byte)),
            0xC2..=0xF4 => self.utf8.push(byte),
            0x80..=0xFF => self.print('\u{FFFD}'),
            _ => {}
        }
    }

    fn escape(&mut self, byte: u8) {
        self.mode = Mode::Ground;
        match byte {
            b'[' => {
                self.mode = Mode::Csi;
                self.params.clear();
                self.private = false;
                self.intermediate = false;
            }
            b']' => {
                self.mode = Mode::Osc;
                self.osc.clear();
            }
            b'P' | b'_' | b'^' | b'X' => self.mode = Mode::Ignore,
            b'(' | b')' | b'*' | b'+' | b'#' | b'%' => self.mode = Mode::Designate,
            b'7' => self.saved_cursor = (self.x, self.y, self.pen),
            b'8' => {
                (self.x, self.y, self.pen) = self.saved_cursor;
                self.x = self.x.min(self.cols - 1);
                self.y = self.y.min(self.rows - 1);
                self.pending_wrap = false;
            }
            b'D' => self.line_feed(),
            b'E' => {
                self.x = 0;
                self.line_feed();
            }
            b'M' => self.reverse_index(),
            b'c' => self.reset(),
            0x1b => self.mode = Mode::Escape,
            _ => {}
        }
    }

    fn csi_byte(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' => {
                if self.params.is_empty() {
                    self.params.push(0);
                }
                if let Some(last) = self.params.last_mut() {
                    *last = last.saturating_mul(10).saturating_add(u32::from(byte - b'0'));
                }
            }
            b';' | b':' => {
                if self.params.is_empty() {
                    self.params.push(0);
                }
                if self.params.len() < MAX_PARAMS {
                    self.params.push(0);
                }
            }
            b'?' | b'>' | b'<' | b'=' => self.private = true,
            0x20..=0x2f => self.intermediate = true,
            0x40..=0x7e => {
                self.mode = Mode::Ground;
                if !self.intermediate {
                    self.dispatch(byte);
                }
            }
            0x1b => self.mode = Mode::Escape,
            0x18 | 0x1a => self.mode = Mode::Ground,
            0x0a..=0x0d | 0x08 | 0x09 => self.ground(byte),
            _ => {}
        }
    }

    fn finish_osc(&mut self) {
        self.mode = Mode::Ground;
        let text = String::from_utf8_lossy(&self.osc).into_owned();
        if let Some((code, rest)) = text.split_once(';') {
            if code == "0" || code == "2" {
                self.title = rest.chars().filter(|ch| !ch.is_control()).collect();
            }
        }
        self.osc.clear();
    }

    fn param(&self, index: usize, default: u32) -> usize {
        match self.params.get(index) {
            Some(&0) | None => default as usize,
            Some(&value) => value as usize,
        }
    }

    fn dispatch(&mut self, last: u8) {
        if self.private {
            self.dispatch_private(last);
            return;
        }
        let n = self.param(0, 1);
        match last {
            b'A' => self.move_to(self.x, self.y.saturating_sub(n).max(self.limit_up())),
            b'B' | b'e' => self.move_to(self.x, (self.y + n).min(self.limit_down())),
            b'C' | b'a' => self.move_to((self.x + n).min(self.cols - 1), self.y),
            b'D' => self.move_to(self.x.saturating_sub(n), self.y),
            b'E' => self.move_to(0, (self.y + n).min(self.rows - 1)),
            b'F' => self.move_to(0, self.y.saturating_sub(n)),
            b'G' | b'`' => self.move_to((n - 1).min(self.cols - 1), self.y),
            b'd' => self.move_to(self.x, (n - 1).min(self.rows - 1)),
            b'H' | b'f' => {
                let row = self.param(0, 1);
                let col = self.param(1, 1);
                self.move_to((col - 1).min(self.cols - 1), (row - 1).min(self.rows - 1));
            }
            b'J' => self.erase_display(self.params.first().copied().unwrap_or(0)),
            b'K' => self.erase_line(self.params.first().copied().unwrap_or(0)),
            b'L' => self.insert_lines(n),
            b'M' => self.delete_lines(n),
            b'@' => self.insert_chars(n),
            b'P' => self.delete_chars(n),
            b'X' => self.erase_chars(n),
            b'S' => self.scroll_up(n),
            b'T' => self.scroll_down(n),
            b'm' => self.sgr(),
            b'r' => self.set_region(),
            b's' => self.saved_cursor = (self.x, self.y, self.pen),
            b'u' => {
                (self.x, self.y, _) = self.saved_cursor;
                self.x = self.x.min(self.cols - 1);
                self.y = self.y.min(self.rows - 1);
                self.pending_wrap = false;
            }
            _ => {}
        }
    }

    fn limit_up(&self) -> usize {
        if self.y >= self.top { self.top } else { 0 }
    }

    fn limit_down(&self) -> usize {
        if self.y <= self.bottom { self.bottom } else { self.rows - 1 }
    }

    fn dispatch_private(&mut self, last: u8) {
        let set = last == b'h';
        if last != b'h' && last != b'l' {
            return;
        }
        let modes: Vec<u32> = self.params.clone();
        for mode in modes {
            match mode {
                7 => self.autowrap = set,
                25 => self.cursor_visible = set,
                47 | 1047 | 1049 => self.set_alternate(set),
                _ => {}
            }
        }
    }

    fn set_alternate(&mut self, enter: bool) {
        if enter == self.alternate.is_some() {
            return;
        }
        if enter {
            let blank = vec![ScreenCell::blank(Style::new()); self.cols * self.rows];
            let main = std::mem::replace(&mut self.cells, blank);
            self.alternate = Some(Saved {
                cells: main,
                x: self.x,
                y: self.y,
                pending_wrap: self.pending_wrap,
            });
            self.x = 0;
            self.y = 0;
            self.pending_wrap = false;
        } else if let Some(saved) = self.alternate.take() {
            self.cells = saved.cells;
            self.x = saved.x.min(self.cols - 1);
            self.y = saved.y.min(self.rows - 1);
            self.pending_wrap = saved.pending_wrap;
        }
        self.top = 0;
        self.bottom = self.rows - 1;
    }

    fn reset(&mut self) {
        let (cols, rows) = (self.cols(), self.rows());
        let title = std::mem::take(&mut self.title);
        let scrollback = std::mem::take(&mut self.scrollback);
        *self = Self::new(cols, rows);
        self.title = title;
        self.scrollback = scrollback;
    }

    fn move_to(&mut self, x: usize, y: usize) {
        self.pending_wrap = false;
        self.x = x;
        self.y = y;
    }

    fn set_region(&mut self) {
        let top = self.param(0, 1) - 1;
        let bottom = self.param(1, u32::try_from(self.rows).unwrap_or(u32::MAX)) - 1;
        if top < bottom && bottom < self.rows {
            self.top = top;
            self.bottom = bottom;
            self.move_to(0, 0);
        }
    }

    fn sgr(&mut self) {
        if self.params.is_empty() {
            self.pen = Style::new();
            return;
        }
        let params = self.params.clone();
        let mut i = 0;
        while i < params.len() {
            let code = params[i];
            let mut attrs = self.pen.attrs;
            match code {
                0 => {
                    self.pen = Style::new();
                    i += 1;
                    continue;
                }
                1 => attrs = attrs.set_bold(true),
                2 => attrs = attrs.set_dim(true),
                3 => attrs = attrs.set_italic(true),
                4 => attrs = attrs.set_underline(UnderlineStyle::Single),
                5 | 6 => attrs = attrs.set_blink(true),
                7 => attrs = attrs.set_reverse(true),
                8 => attrs = attrs.set_hidden(true),
                9 => attrs = attrs.set_strikethrough(true),
                21 | 24 => attrs = attrs.set_underline(UnderlineStyle::None),
                22 => attrs = attrs.set_bold(false).set_dim(false),
                23 => attrs = attrs.set_italic(false),
                25 => attrs = attrs.set_blink(false),
                27 => attrs = attrs.set_reverse(false),
                28 => attrs = attrs.set_hidden(false),
                29 => attrs = attrs.set_strikethrough(false),
                30..=37 => self.pen.fg = Color::Indexed((code - 30) as u8),
                39 => self.pen.fg = Color::Default,
                40..=47 => self.pen.bg = Color::Indexed((code - 40) as u8),
                49 => self.pen.bg = Color::Default,
                90..=97 => self.pen.fg = Color::Indexed((code - 90 + 8) as u8),
                100..=107 => self.pen.bg = Color::Indexed((code - 100 + 8) as u8),
                38 | 48 => {
                    let (color, used) = extended_color(&params[i + 1..]);
                    if let Some(color) = color {
                        if code == 38 {
                            self.pen.fg = color;
                        } else {
                            self.pen.bg = color;
                        }
                    }
                    i += 1 + used;
                    continue;
                }
                _ => {}
            }
            self.pen.attrs = attrs;
            i += 1;
        }
    }

    fn blank(&self) -> ScreenCell {
        ScreenCell::blank(Style::new().bg(self.pen.bg))
    }

    fn tab(&mut self) {
        self.pending_wrap = false;
        let next = (self.x / TAB_STOP + 1) * TAB_STOP;
        self.x = next.min(self.cols - 1);
    }

    fn line_feed(&mut self) {
        if self.y == self.bottom {
            self.scroll_up(1);
        } else if self.y + 1 < self.rows {
            self.y += 1;
        }
    }

    fn reverse_index(&mut self) {
        if self.y == self.top {
            self.scroll_down(1);
        } else {
            self.y = self.y.saturating_sub(1);
        }
    }

    fn print(&mut self, ch: char) {
        let mut buf = [0_u8; 4];
        let cells = width(ch.encode_utf8(&mut buf));
        if cells == 0 {
            return;
        }
        let cells = cells.min(2);
        if self.pending_wrap && self.autowrap {
            self.x = 0;
            self.line_feed();
        }
        self.pending_wrap = false;
        if self.x + cells > self.cols {
            if !self.autowrap || cells > self.cols {
                return;
            }
            self.x = 0;
            self.line_feed();
        }
        let style = self.pen;
        let at = self.y * self.cols + self.x;
        self.break_wide(at);
        self.cells[at] = ScreenCell {
            ch,
            style,
            tail: false,
        };
        if cells == 2 {
            self.break_wide(at + 1);
            self.cells[at + 1] = ScreenCell {
                ch: ' ',
                style,
                tail: true,
            };
        }
        self.x += cells;
        if self.x >= self.cols {
            self.x = self.cols - 1;
            self.pending_wrap = true;
        }
    }

    fn break_wide(&mut self, at: usize) {
        let blank = self.blank();
        if self.cells[at].tail && at % self.cols > 0 {
            self.cells[at - 1] = blank;
        }
        if at + 1 < self.cells.len() && at % self.cols + 1 < self.cols && self.cells[at + 1].tail {
            self.cells[at + 1] = blank;
        }
    }

    fn row_range(&self, row: usize) -> std::ops::Range<usize> {
        row * self.cols..(row + 1) * self.cols
    }

    fn scroll_up(&mut self, n: usize) {
        let keep = self.alternate.is_none() && self.top == 0;
        self.scroll_region_up(n, keep);
    }

    fn scroll_region_up(&mut self, n: usize, keep: bool) {
        let n = n.min(self.bottom - self.top + 1);
        for _ in 0..n {
            let blank = self.blank();
            let range = self.row_range(self.top);
            if keep {
                self.scrollback.push_back(self.cells[range.clone()].to_vec());
                if self.scrollback.len() > SCROLLBACK_LIMIT {
                    self.scrollback.pop_front();
                }
            }
            let start = range.start;
            let end = self.row_range(self.bottom).end;
            self.cells[start..end].rotate_left(self.cols);
            let last = self.row_range(self.bottom);
            self.cells[last].fill(blank);
        }
    }

    fn scroll_down(&mut self, n: usize) {
        let n = n.min(self.bottom - self.top + 1);
        for _ in 0..n {
            let blank = self.blank();
            let start = self.row_range(self.top).start;
            let end = self.row_range(self.bottom).end;
            self.cells[start..end].rotate_right(self.cols);
            let first = self.row_range(self.top);
            self.cells[first].fill(blank);
        }
    }

    fn insert_lines(&mut self, n: usize) {
        if self.y < self.top || self.y > self.bottom {
            return;
        }
        let saved_top = self.top;
        self.top = self.y;
        self.scroll_down(n);
        self.top = saved_top;
        self.move_to(0, self.y);
    }

    fn delete_lines(&mut self, n: usize) {
        if self.y < self.top || self.y > self.bottom {
            return;
        }
        let saved_top = self.top;
        self.top = self.y;
        self.scroll_region_up(n, false);
        self.top = saved_top;
        self.move_to(0, self.y);
    }

    fn insert_chars(&mut self, n: usize) {
        self.pending_wrap = false;
        let blank = self.blank();
        let row = self.row_range(self.y);
        let line = &mut self.cells[row];
        let n = n.min(line.len() - self.x);
        line[self.x..].rotate_right(n);
        line[self.x..self.x + n].fill(blank);
    }

    fn delete_chars(&mut self, n: usize) {
        self.pending_wrap = false;
        let blank = self.blank();
        let row = self.row_range(self.y);
        let line = &mut self.cells[row];
        let n = n.min(line.len() - self.x);
        line[self.x..].rotate_left(n);
        let len = line.len();
        line[len - n..].fill(blank);
    }

    fn erase_chars(&mut self, n: usize) {
        let blank = self.blank();
        let end = (self.x + n).min(self.cols);
        let row = self.row_range(self.y);
        self.cells[row][self.x..end].fill(blank);
    }

    fn erase_line(&mut self, mode: u32) {
        let blank = self.blank();
        let row = self.row_range(self.y);
        let line = &mut self.cells[row];
        match mode {
            0 => line[self.x..].fill(blank),
            1 => line[..=self.x].fill(blank),
            2 => line.fill(blank),
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: u32) {
        let blank = self.blank();
        match mode {
            0 => {
                self.erase_line(0);
                let start = self.row_range(self.y).end;
                self.cells[start..].fill(blank);
            }
            1 => {
                self.erase_line(1);
                let end = self.row_range(self.y).start;
                self.cells[..end].fill(blank);
            }
            2 => self.cells.fill(blank),
            3 => self.scrollback.clear(),
            _ => {}
        }
    }
}

fn extended_color(rest: &[u32]) -> (Option<Color>, usize) {
    match rest.first() {
        Some(5) => {
            let color = rest
                .get(1)
                .map(|&index| Color::Indexed(u8::try_from(index.min(255)).unwrap_or(255)));
            (color, 2.min(rest.len()))
        }
        Some(2) => {
            if rest.len() >= 4 {
                let channel = |value: u32| u8::try_from(value.min(255)).unwrap_or(255);
                (
                    Some(Color::Rgb(channel(rest[1]), channel(rest[2]), channel(rest[3]))),
                    4,
                )
            } else {
                (None, rest.len())
            }
        }
        _ => (None, rest.len().min(1)),
    }
}

fn utf8_len(lead: u8) -> usize {
    match lead {
        0xF0..=0xF4 => 4,
        0xE0..=0xEF => 3,
        _ => 2,
    }
}

fn resized(
    old: &[ScreenCell],
    old_cols: usize,
    old_rows: usize,
    cols: usize,
    rows: usize,
) -> Vec<ScreenCell> {
    let mut cells = vec![ScreenCell::blank(Style::new()); cols * rows];
    for y in 0..rows.min(old_rows) {
        for x in 0..cols.min(old_cols) {
            cells[y * cols + x] = old[y * old_cols + x];
        }
    }
    cells
}

pub(super) fn line_text(line: &[ScreenCell]) -> String {
    let text: String = line
        .iter()
        .filter(|cell| !cell.tail)
        .map(|cell| cell.ch)
        .collect();
    text.trim_end().to_owned()
}
