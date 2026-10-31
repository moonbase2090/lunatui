// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::ops::Range;

use lunatui_core::{Attrs, Color, Rect, Size, Style, cluster_display_width};
use lunatui_theme::Token;
use unicode_segmentation::UnicodeSegmentation;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Scrollbar, Widget, WidgetState};

const DEFAULT_TAB: u16 = 4;

/// A sign drawn in the gutter beside a line of a [`CodeView`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeMark {
    /// The line was added: `▎` in the ok color.
    Added,
    /// The line was changed: `▎` in the warn color.
    Changed,
    /// Lines were removed after this one: `▁` in the fail color.
    Removed,
    /// An error on the line: `●` in the fail color.
    Error,
    /// A warning on the line: `●` in the warn color.
    Warning,
    /// A note on the line: `●` in the accent color.
    Info,
    /// A breakpoint on the line: `◆` in the fail color.
    Breakpoint,
}

impl CodeMark {
    fn glyph(self) -> &'static str {
        match self {
            Self::Added | Self::Changed => "▎",
            Self::Removed => "▁",
            Self::Error | Self::Warning | Self::Info => "●",
            Self::Breakpoint => "◆",
        }
    }

    fn token(self) -> Token {
        match self {
            Self::Added => Token::Ok,
            Self::Changed | Self::Warning => Token::Warn,
            Self::Removed | Self::Error | Self::Breakpoint => Token::Fail,
            Self::Info => Token::Accent,
        }
    }
}

/// Styles parts of a line in a [`CodeView`], such as a syntax highlighter or search hits.
///
/// [`highlight`](Self::highlight) is called for each visible line with its index and text, and
/// calls `paint` with byte ranges of that text and the style to lay over them. A color left as
/// [`Color::Default`] keeps the line's color, and empty attributes keep the line's. Closures
/// with the same signature implement it.
pub trait CodeHighlight {
    /// Paints byte ranges of line `line`, whose text is `text`.
    fn highlight(&self, line: usize, text: &str, paint: &mut dyn FnMut(Range<usize>, Style));
}

impl<F> CodeHighlight for F
where
    F: Fn(usize, &str, &mut dyn FnMut(Range<usize>, Style)),
{
    fn highlight(&self, line: usize, text: &str, paint: &mut dyn FnMut(Range<usize>, Style)) {
        self(line, text, paint);
    }
}

/// Where a [`CodeView`] is scrolled and which line has the cursor, kept by the app between
/// frames.
///
/// [`CodeView::sync`] records how many lines there are and how many rows show, so the moves keep
/// the cursor on screen and the top line in range. Without a cursor, the moves scroll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CodeViewState {
    top: usize,
    left: u16,
    cursor: Option<usize>,
    lines: usize,
    rows: usize,
}

impl CodeViewState {
    /// Scrolled to the top left, without a cursor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            top: 0,
            left: 0,
            cursor: None,
            lines: 0,
            rows: 0,
        }
    }

    /// The first line shown.
    #[must_use]
    pub const fn top(&self) -> usize {
        self.top
    }

    /// Columns scrolled off the left of the text.
    #[must_use]
    pub const fn left(&self) -> u16 {
        self.left
    }

    /// The line with the cursor.
    #[must_use]
    pub const fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// Puts the cursor on `line`, or removes it, and scrolls so it shows.
    pub fn set_cursor(&mut self, line: Option<usize>) {
        self.cursor = line.map(|line| line.min(self.lines.saturating_sub(1)));
        self.reveal();
    }

    /// Moves the cursor down by `lines`, or scrolls down without a cursor.
    pub fn down(&mut self, lines: usize) {
        match self.cursor {
            Some(cursor) => self.set_cursor(Some(cursor.saturating_add(lines))),
            None => self.scroll_to(self.top.saturating_add(lines)),
        }
    }

    /// Moves the cursor up by `lines`, or scrolls up without a cursor.
    pub fn up(&mut self, lines: usize) {
        match self.cursor {
            Some(cursor) => self.set_cursor(Some(cursor.saturating_sub(lines))),
            None => self.scroll_to(self.top.saturating_sub(lines)),
        }
    }

    /// Moves down by one screen of rows.
    pub fn page_down(&mut self) {
        self.down(self.rows.max(1));
    }

    /// Moves up by one screen of rows.
    pub fn page_up(&mut self) {
        self.up(self.rows.max(1));
    }

    /// Makes `line` the first line shown, as far as the last screen allows.
    pub fn scroll_to(&mut self, line: usize) {
        self.top = line.min(self.max_top());
    }

    /// Scrolls the text left by `columns`.
    pub fn scroll_left(&mut self, columns: u16) {
        self.left = self.left.saturating_sub(columns);
    }

    /// Scrolls the text right by `columns`.
    pub fn scroll_right(&mut self, columns: u16) {
        self.left = self.left.saturating_add(columns);
    }

    fn max_top(&self) -> usize {
        self.lines.saturating_sub(self.rows.max(1))
    }

    fn reveal(&mut self) {
        if let Some(cursor) = self.cursor {
            if cursor < self.top {
                self.top = cursor;
            } else if self.rows > 0 && cursor >= self.top + self.rows {
                self.top = cursor + 1 - self.rows;
            }
        }
        self.top = self.top.min(self.max_top());
    }
}

/// Read-only lines of code or text with a line-number gutter.
///
/// Each row shows one line from the top line on, cut at the right edge; there is no wrapping.
/// Tabs move to the next multiple of [`tab_width`](Self::tab_width) columns. The gutter holds a
/// column of [`CodeMark`] signs when any are given, then the line numbers right aligned and
/// dimmed, then a blank column. The line with the cursor is drawn on `surface.1` with its number
/// bold in `accent` (`focus.ring` when focused). A vertical [`Scrollbar`] takes the right column
/// when the lines do not fit. A [`CodeHighlight`] styles parts of each visible line.
///
/// The lines are borrowed, so a built view draws without allocating. [`sync`](Self::sync) gives
/// [`CodeViewState`] the size it scrolls in, and [`line_at`](Self::line_at) finds the line under
/// a click.
pub struct CodeView<'a, S = &'a str> {
    lines: &'a [S],
    first_number: usize,
    top: usize,
    left: u16,
    cursor: Option<usize>,
    marks: &'a [(usize, CodeMark)],
    highlighter: Option<&'a dyn CodeHighlight>,
    tab_width: u16,
    numbers: bool,
    scrollbar: bool,
    state: WidgetState,
}

impl<'a, S: AsRef<str>> CodeView<'a, S> {
    /// A view of `lines`, numbered from one, scrolled to the top.
    #[must_use]
    pub const fn new(lines: &'a [S]) -> Self {
        Self {
            lines,
            first_number: 1,
            top: 0,
            left: 0,
            cursor: None,
            marks: &[],
            highlighter: None,
            tab_width: DEFAULT_TAB,
            numbers: true,
            scrollbar: true,
            state: WidgetState::Normal,
        }
    }

    /// The number shown beside the first line, one by default.
    #[must_use]
    pub const fn first_line_number(mut self, number: usize) -> Self {
        self.first_number = number;
        self
    }

    /// Scrolls and places the cursor as `state` says.
    #[must_use]
    pub const fn view(mut self, state: &CodeViewState) -> Self {
        self.top = state.top;
        self.left = state.left;
        self.cursor = state.cursor;
        self
    }

    /// Gutter signs as `(line index, mark)` pairs; the first pair for a line is drawn.
    #[must_use]
    pub const fn marks(mut self, marks: &'a [(usize, CodeMark)]) -> Self {
        self.marks = marks;
        self
    }

    /// Styles parts of each visible line.
    #[must_use]
    pub fn highlight(mut self, highlighter: &'a dyn CodeHighlight) -> Self {
        self.highlighter = Some(highlighter);
        self
    }

    /// Columns between tab stops, four by default and at least one.
    #[must_use]
    pub fn tab_width(mut self, columns: u16) -> Self {
        self.tab_width = columns.max(1);
        self
    }

    /// Whether to draw line numbers, on by default.
    #[must_use]
    pub const fn line_numbers(mut self, show: bool) -> Self {
        self.numbers = show;
        self
    }

    /// Whether to draw a scrollbar when the lines do not fit, on by default.
    #[must_use]
    pub const fn scrollbar(mut self, show: bool) -> Self {
        self.scrollbar = show;
        self
    }

    /// Interaction state; a focused view draws the cursor line number in `focus.ring`.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Columns the gutter takes: the mark column, the widest line number and a blank column.
    #[must_use]
    pub fn gutter_width(&self) -> u16 {
        let marks = u16::from(!self.marks.is_empty());
        if !self.numbers {
            return marks;
        }
        let last = self
            .first_number
            .saturating_add(self.lines.len().saturating_sub(1));
        marks + digits(last) + 1
    }

    /// The cells the text is drawn in inside `area`, right of the gutter and left of the
    /// scrollbar.
    #[must_use]
    pub fn text_area(&self, area: Rect) -> Rect {
        let area = clip_to_coordinates(area);
        let gutter = self.gutter_width().min(area.width);
        let bar = u16::from(self.has_scrollbar(area));
        Rect::new(
            area.x + gutter,
            area.y,
            area.width.saturating_sub(gutter).saturating_sub(bar),
            area.height,
        )
    }

    /// The line drawn on `row` inside `area`, if there is one.
    #[must_use]
    pub fn line_at(&self, area: Rect, row: u16) -> Option<usize> {
        let area = clip_to_coordinates(area);
        if row < area.y || row - area.y >= area.height {
            return None;
        }
        let line = self.top.checked_add(usize::from(row - area.y))?;
        (line < self.lines.len()).then_some(line)
    }

    /// Records the line count and visible rows in `state`, and keeps its top line and cursor
    /// in range.
    pub fn sync(&self, area: Rect, state: &mut CodeViewState) {
        state.lines = self.lines.len();
        state.rows = usize::from(clip_to_coordinates(area).height);
        state.cursor = state
            .cursor
            .filter(|_| !self.lines.is_empty())
            .map(|cursor| cursor.min(self.lines.len() - 1));
        state.reveal();
    }

    fn has_scrollbar(&self, area: Rect) -> bool {
        self.scrollbar && area.width > 1 && self.lines.len() > usize::from(area.height)
    }

    fn mark_for(&self, line: usize) -> Option<CodeMark> {
        self.marks
            .iter()
            .find(|(at, _)| *at == line)
            .map(|(_, mark)| *mark)
    }

    fn draw_line(&self, line: usize, text: &str, row: Rect, base: Style, cx: &mut RenderCx<'_>) {
        let end = u32::from(self.left) + u32::from(row.width);
        let mut column = 0_u32;
        for grapheme in text.graphemes(true) {
            let cells = self.cells(grapheme, column);
            let next = column + cells;
            if next > end {
                break;
            }
            if grapheme != "\t" && column >= u32::from(self.left) {
                let x = row.x + to_u16(column - u32::from(self.left));
                cx.put_str(x, row.y, grapheme, base);
            }
            column = next;
        }
        if let Some(highlighter) = self.highlighter {
            highlighter.highlight(line, text, &mut |bytes: Range<usize>, style: Style| {
                let start = self.column_of(text, bytes.start);
                let stop = self.column_of(text, bytes.end.max(bytes.start));
                let from = start.max(u32::from(self.left)).min(end);
                let to = stop.max(u32::from(self.left)).min(end);
                if to > from {
                    let x = row.x + to_u16(from - u32::from(self.left));
                    cx.set_style(
                        Rect::new(x, row.y, to_u16(to - from), 1),
                        overlay(base, style),
                    );
                }
            });
        }
    }

    fn cells(&self, grapheme: &str, column: u32) -> u32 {
        if grapheme == "\t" {
            let tab = u32::from(self.tab_width);
            tab - column % tab
        } else {
            u32::try_from(cluster_display_width(grapheme)).unwrap_or(u32::MAX)
        }
    }

    fn column_of(&self, text: &str, byte: usize) -> u32 {
        let mut column = 0_u32;
        for (at, grapheme) in text.grapheme_indices(true) {
            if at >= byte {
                break;
            }
            column += self.cells(grapheme, column);
        }
        column
    }
}

impl<S: AsRef<str>> Widget for CodeView<'_, S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let text_color: Color = cx.token(Token::TextPrimary);
        let plain = Style::new().fg(text_color);
        let current = plain.bg(cx.token(Token::Surface1));
        let number = plain.with_attrs(Attrs::new().set_dim(true));
        let tone = if matches!(self.state, WidgetState::Focused) {
            cx.token(Token::FocusRing)
        } else {
            cx.token(Token::Accent)
        };
        let gutter = self.gutter_width().min(area.width);
        let text = self.text_area(area);
        let marks = u16::from(!self.marks.is_empty()).min(gutter);
        let digits_width = gutter.saturating_sub(marks).saturating_sub(1);
        for offset in 0..area.height {
            let y = area.y + offset;
            let line = self.top.saturating_add(usize::from(offset));
            let Some(source) = self.lines.get(line) else {
                cx.fill(Rect::new(area.x, y, gutter + text.width, 1), plain, ' ');
                continue;
            };
            let on_cursor = self.cursor == Some(line);
            let base = if on_cursor { current } else { plain };
            cx.fill(Rect::new(area.x, y, gutter + text.width, 1), base, ' ');
            if marks > 0
                && let Some(mark) = self.mark_for(line)
            {
                cx.put_str(area.x, y, mark.glyph(), base.fg(cx.token(mark.token())));
            }
            if digits_width > 0 {
                let style = if on_cursor {
                    base.fg(tone).with_attrs(Attrs::new().set_bold(true))
                } else {
                    number.bg(base.bg)
                };
                let mut buffer = [0_u8; 20];
                let shown = write_number(self.first_number.saturating_add(line), &mut buffer);
                let skip = shown.len().saturating_sub(usize::from(digits_width));
                let shown = &shown[skip..];
                let x =
                    area.x + marks + digits_width - to_u16(u32::try_from(shown.len()).unwrap_or(0));
                cx.put_str(x, y, shown, style);
            }
            self.draw_line(
                line,
                source.as_ref(),
                Rect::new(text.x, y, text.width, 1),
                base,
                cx,
            );
        }
        if self.has_scrollbar(area) {
            let bar = Rect::new(area.x + area.width - 1, area.y, 1, area.height);
            Scrollbar::vertical(self.lines.len(), usize::from(area.height), self.top)
                .render(bar, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn overlay(base: Style, over: Style) -> Style {
    Style::new()
        .fg(if over.fg == Color::Default {
            base.fg
        } else {
            over.fg
        })
        .bg(if over.bg == Color::Default {
            base.bg
        } else {
            over.bg
        })
        .with_attrs(if over.attrs == Attrs::new() {
            base.attrs
        } else {
            over.attrs
        })
}

fn digits(mut number: usize) -> u16 {
    let mut count = 1;
    while number >= 10 {
        number /= 10;
        count += 1;
    }
    count
}

fn write_number(mut number: usize, buffer: &mut [u8; 20]) -> &str {
    let mut at = buffer.len();
    loop {
        at -= 1;
        buffer[at] = b'0' + u8::try_from(number % 10).unwrap_or(0);
        number /= 10;
        if number == 0 {
            break;
        }
    }
    std::str::from_utf8(&buffer[at..]).unwrap_or("")
}

fn to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
