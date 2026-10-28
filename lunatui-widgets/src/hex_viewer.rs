// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::ops::Range;

use lunatui_core::{Attrs, Rect, Size, Style};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Scrollbar, Widget, WidgetState};

const DEFAULT_ROW: usize = 16;
const DEFAULT_GROUP: usize = 8;
const MIN_DIGITS: usize = 8;
const GAP: usize = 2;
const MAX_ROW: usize = 4096;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Cursor, selection, and scroll position of a [`HexViewer`], kept by the app between frames.
///
/// [`HexViewer::sync`] records the data length, the bytes per row, and the visible rows, so the
/// moves know where rows and pages end. Moving the cursor scrolls to keep it on screen; scrolling
/// with [`scroll_up`](Self::scroll_up) or [`scroll_down`](Self::scroll_down) leaves the cursor
/// where it is until the next move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HexViewerState {
    cursor: usize,
    anchor: Option<usize>,
    top: usize,
    follow: bool,
    len: Option<usize>,
    per_row: Option<usize>,
    page: Option<usize>,
}

impl Default for HexViewerState {
    fn default() -> Self {
        Self::new()
    }
}

impl HexViewerState {
    /// The cursor on the first byte, nothing selected, scrolled to the top.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cursor: 0,
            anchor: None,
            top: 0,
            follow: true,
            len: None,
            per_row: None,
            page: None,
        }
    }

    /// Index of the byte under the cursor.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// The first visible row.
    #[must_use]
    pub const fn top_row(&self) -> usize {
        self.top
    }

    /// The selected bytes, from the anchor to the cursor inclusive. `None` when nothing is
    /// selected.
    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        Some(anchor.min(self.cursor)..anchor.max(self.cursor).saturating_add(1))
    }

    /// Moves the cursor to byte `at`. With `extend`, the selection grows from where the cursor
    /// was; without it, the selection is cleared.
    pub fn move_to(&mut self, at: usize, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = at;
        self.follow = true;
        self.clamp();
        self.reveal();
    }

    /// Moves the cursor one byte back.
    pub fn move_left(&mut self, extend: bool) {
        self.move_to(self.cursor.saturating_sub(1), extend);
    }

    /// Moves the cursor one byte on.
    pub fn move_right(&mut self, extend: bool) {
        self.move_to(self.cursor.saturating_add(1), extend);
    }

    /// Moves the cursor one row up, staying put on the first row.
    pub fn move_up(&mut self, extend: bool) {
        self.move_rows_up(1, extend);
    }

    /// Moves the cursor one row down. On the row above a shorter last row it stops on the last
    /// byte, and on the last row it stays put.
    pub fn move_down(&mut self, extend: bool) {
        self.move_rows_down(1, extend);
    }

    /// Moves the cursor up by one page of visible rows, or to the first row when it is closer.
    pub fn page_up(&mut self, extend: bool) {
        self.move_rows_up(self.page.unwrap_or(1).max(1), extend);
    }

    /// Moves the cursor down by one page of visible rows.
    pub fn page_down(&mut self, extend: bool) {
        self.move_rows_down(self.page.unwrap_or(1).max(1), extend);
    }

    /// Moves the cursor to the first byte of its row.
    pub fn row_start(&mut self, extend: bool) {
        let per_row = self.per_row();
        self.move_to(self.cursor - self.cursor % per_row, extend);
    }

    /// Moves the cursor to the last byte of its row.
    pub fn row_end(&mut self, extend: bool) {
        let per_row = self.per_row();
        let start = self.cursor - self.cursor % per_row;
        self.move_to(start + (per_row - 1), extend);
    }

    /// Moves the cursor to the first byte.
    pub fn to_start(&mut self, extend: bool) {
        self.move_to(0, extend);
    }

    /// Moves the cursor to the last byte. Before the first sync it goes as far as it can, and the
    /// next sync pulls it back to the last byte.
    pub fn to_end(&mut self, extend: bool) {
        self.move_to(usize::MAX, extend);
    }

    /// Selects every byte, with the cursor on the last one.
    pub fn select_all(&mut self) {
        self.move_to(0, false);
        self.to_end(true);
    }

    /// Clears the selection and keeps the cursor.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Scrolls the view up by `rows` without moving the cursor.
    pub fn scroll_up(&mut self, rows: usize) {
        self.top = self.top.saturating_sub(rows);
        self.follow = false;
    }

    /// Scrolls the view down by `rows` without moving the cursor.
    pub fn scroll_down(&mut self, rows: usize) {
        self.top = self.top.saturating_add(rows);
        self.follow = false;
        self.clamp();
    }

    fn per_row(&self) -> usize {
        self.per_row.unwrap_or(DEFAULT_ROW).max(1)
    }

    fn move_rows_up(&mut self, rows: usize, extend: bool) {
        let step = rows.saturating_mul(self.per_row());
        let at = self
            .cursor
            .checked_sub(step)
            .unwrap_or(self.cursor % self.per_row());
        self.move_to(at, extend);
    }

    fn move_rows_down(&mut self, rows: usize, extend: bool) {
        let per_row = self.per_row();
        let mut at = self.cursor.saturating_add(rows.saturating_mul(per_row));
        if let Some(len) = self.len {
            let last = len.saturating_sub(1);
            if at / per_row > last / per_row {
                at = if last / per_row > self.cursor / per_row {
                    last
                } else {
                    self.cursor
                };
            }
        }
        self.move_to(at, extend);
    }

    fn clamp(&mut self) {
        let Some(len) = self.len else {
            return;
        };
        let last = len.saturating_sub(1);
        self.cursor = self.cursor.min(last);
        if len == 0 {
            self.anchor = None;
        }
        self.anchor = self.anchor.map(|anchor| anchor.min(last));
        if let Some(page) = self.page {
            let rows = len.div_ceil(self.per_row());
            self.top = self.top.min(rows.saturating_sub(page));
        }
    }

    fn reveal(&mut self) {
        let (Some(per_row), Some(page)) = (self.per_row, self.page) else {
            return;
        };
        if page == 0 {
            return;
        }
        let row = self.cursor / per_row.max(1);
        self.top = self.top.min(row);
        if row >= self.top + page {
            self.top = row + 1 - page;
        }
    }
}

/// A hex dump of a byte slice: an offset column, the bytes in hex, and the same bytes as ASCII,
/// with a cursor and a selection.
///
/// Only the visible rows are drawn, so the slice can be as long as the app likes. Rows hold
/// [`bytes_per_row`](Self::bytes_per_row) bytes (16 by default) with an extra space every
/// [`group`](Self::group) bytes (8 by default), and
/// [`fitting_bytes_per_row`](Self::fitting_bytes_per_row) picks the widest row that fits a width.
/// [`base_offset`](Self::base_offset) sets the address of the first byte, for showing a window
/// into a larger file.
///
/// Colors come from theme tokens and the [`WidgetState`]. Offsets and zero bytes are dimmed
/// `text.primary`, printable ASCII (space included) is `text.primary`, other control bytes such as
/// tabs and newlines are `warn`, and bytes from `0x80` up are `accent`. Selected bytes sit on `surface.1`, and the cursor
/// is `surface.0` on `accent`, or on `focus.ring` when focused. A disabled viewer dims every byte
/// and shows no cursor or selection. Cells keep the background under the viewer except where the
/// selection or cursor sets one. A vertical scrollbar takes the right column when the rows do not
/// fit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HexViewer<'a> {
    data: &'a [u8],
    base: u64,
    per_row: usize,
    group: usize,
    top: usize,
    cursor: Option<usize>,
    selection: Option<Range<usize>>,
    header: bool,
    scrollbar: bool,
    state: WidgetState,
    style: Option<Style>,
}

#[derive(Clone, Copy)]
struct Columns {
    digits: usize,
    hex: usize,
    ascii: usize,
    width: usize,
}

impl<'a> HexViewer<'a> {
    /// A viewer of `data`, starting at offset zero.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            base: 0,
            per_row: DEFAULT_ROW,
            group: DEFAULT_GROUP,
            top: 0,
            cursor: None,
            selection: None,
            header: false,
            scrollbar: true,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// The offset shown for the first byte.
    #[must_use]
    pub const fn base_offset(mut self, offset: u64) -> Self {
        self.base = offset;
        self
    }

    /// Bytes on each row, at least one.
    #[must_use]
    pub fn bytes_per_row(mut self, bytes: usize) -> Self {
        self.per_row = bytes.clamp(1, MAX_ROW);
        self
    }

    /// An extra space after every `bytes` bytes in the hex column. Zero turns grouping off.
    #[must_use]
    pub const fn group(mut self, bytes: usize) -> Self {
        self.group = bytes;
        self
    }

    /// Takes the cursor, selection, and scroll position from `state`.
    #[must_use]
    pub fn view(mut self, state: &HexViewerState) -> Self {
        self.top = state.top_row();
        self.cursor = Some(state.cursor());
        self.selection = state.selection();
        self
    }

    /// The first visible row.
    #[must_use]
    pub const fn top_row(mut self, row: usize) -> Self {
        self.top = row;
        self
    }

    /// The byte under the cursor, or `None` for no cursor.
    #[must_use]
    pub const fn cursor(mut self, at: Option<usize>) -> Self {
        self.cursor = at;
        self
    }

    /// The selected bytes, or `None` for no selection.
    #[must_use]
    pub fn selection(mut self, bytes: Option<Range<usize>>) -> Self {
        self.selection = bytes;
        self
    }

    /// A first row labelling each hex and ASCII column with its position in the row.
    #[must_use]
    pub const fn header(mut self, header: bool) -> Self {
        self.header = header;
        self
    }

    /// Whether the scrollbar is drawn when the rows do not fit. On by default.
    #[must_use]
    pub const fn scrollbar(mut self, scrollbar: bool) -> Self {
        self.scrollbar = scrollbar;
        self
    }

    /// Interaction state for the cursor and scrollbar colors.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws offsets and bytes in `style` instead of theme tokens. The cursor and selection keep
    /// theirs.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Number of rows the data takes.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.data.len().div_ceil(self.per_row)
    }

    /// Cells one row takes, from the offset to the last ASCII column, without the scrollbar.
    #[must_use]
    pub fn row_width(&self) -> usize {
        self.columns(self.per_row).width
    }

    /// The most bytes per row whose rows fit in `width` cells, in whole groups when grouping is on.
    /// At least one.
    #[must_use]
    pub fn fitting_bytes_per_row(&self, width: usize) -> usize {
        let step = if self.group == 0 { 1 } else { self.group };
        let mut best = 1;
        let mut bytes = step;
        while bytes <= MAX_ROW && self.columns(bytes).width <= width {
            best = bytes;
            bytes += step;
        }
        best
    }

    /// Records the data length, row length, and visible rows for `area` in `state`, keeping the
    /// cursor on a byte and on screen.
    pub fn sync(&self, area: Rect, state: &mut HexViewerState) {
        let area = clip_to_coordinates(area);
        state.len = Some(self.data.len());
        state.per_row = Some(self.per_row);
        state.page = Some(usize::from(self.body(area).height));
        state.clamp();
        if state.follow {
            state.reveal();
        }
    }

    /// The byte drawn at `(x, y)` in either the hex or the ASCII column, for mouse selection.
    #[must_use]
    pub fn byte_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let area = clip_to_coordinates(area);
        let body = self.body(area);
        if y < body.y || y >= body.y + body.height || x >= self.text_end(area) {
            return None;
        }
        let row = self.first_row(body) + usize::from(y - body.y);
        let columns = self.columns(self.per_row);
        let dx = usize::from(x.checked_sub(area.x)?);
        let column = if dx >= columns.ascii {
            Some(dx - columns.ascii).filter(|column| *column < self.per_row)
        } else {
            (0..self.per_row).find(|&column| {
                let at = self.hex_x(&columns, column);
                dx == at || dx == at + 1
            })
        }?;
        let index = row.checked_mul(self.per_row)?.checked_add(column)?;
        (index < self.data.len()).then_some(index)
    }

    fn columns(&self, per_row: usize) -> Columns {
        let last = self
            .base
            .saturating_add(u64::try_from(self.data.len().saturating_sub(1)).unwrap_or(u64::MAX));
        let bits = u64::BITS - last.leading_zeros();
        let digits = usize::try_from(bits.div_ceil(4))
            .unwrap_or(16)
            .max(MIN_DIGITS);
        let groups = (per_row - 1).checked_div(self.group).unwrap_or(0);
        let hex = digits + GAP;
        let ascii = hex + 3 * per_row - 1 + groups + GAP;
        Columns {
            digits,
            hex,
            ascii,
            width: ascii + per_row,
        }
    }

    fn hex_x(&self, columns: &Columns, column: usize) -> usize {
        let groups = column.checked_div(self.group).unwrap_or(0);
        columns.hex + 3 * column + groups
    }

    fn body(&self, area: Rect) -> Rect {
        let header = u16::from(self.header).min(area.height);
        Rect::new(area.x, area.y + header, area.width, area.height - header)
    }

    fn has_bar(&self, body: Rect) -> bool {
        self.scrollbar && self.row_count() > usize::from(body.height)
    }

    fn text_end(&self, area: Rect) -> u16 {
        let bar = u16::from(self.has_bar(self.body(area)));
        area.x + area.width.saturating_sub(bar)
    }

    fn first_row(&self, body: Rect) -> usize {
        self.top
            .min(self.row_count().saturating_sub(usize::from(body.height)))
    }

    fn byte_style(&self, cx: &RenderCx<'_>, byte: u8) -> Style {
        if let Some(style) = self.style {
            return style;
        }
        let text = Style::new().fg(cx.token(Token::TextPrimary));
        if self.state == WidgetState::Disabled {
            return text.with_attrs(Attrs::new().set_dim(true));
        }
        match byte {
            0 => text.with_attrs(Attrs::new().set_dim(true)),
            0x20..=0x7e => text,
            0x80..=0xff => Style::new().fg(cx.token(Token::Accent)),
            _ => Style::new().fg(cx.token(Token::Warn)),
        }
    }

    fn dim_style(&self, cx: &RenderCx<'_>) -> Style {
        self.style.unwrap_or_else(|| {
            Style::new()
                .fg(cx.token(Token::TextPrimary))
                .with_attrs(Attrs::new().set_dim(true))
        })
    }

    fn mark(&self, cx: &RenderCx<'_>, index: usize, style: Style) -> Style {
        if self.state == WidgetState::Disabled {
            return style;
        }
        if self.cursor == Some(index) {
            let bg = if self.state == WidgetState::Focused {
                Token::FocusRing
            } else {
                Token::Accent
            };
            return Style::new()
                .fg(cx.token(Token::Surface0))
                .bg(cx.token(bg))
                .with_attrs(Attrs::new().set_bold(true));
        }
        if self.selected(index) {
            return style.bg(cx.token(Token::Surface1));
        }
        style
    }

    fn selected(&self, index: usize) -> bool {
        self.state != WidgetState::Disabled
            && self
                .selection
                .as_ref()
                .is_some_and(|range| range.contains(&index))
    }

    fn draw_header(&self, cx: &mut RenderCx<'_>, area: Rect, columns: &Columns, end: u16) {
        let style = self.dim_style(cx);
        for column in 0..self.per_row {
            let label = [HEX[(column >> 4) & 0xf], HEX[column & 0xf]];
            put(
                cx,
                at(area.x, self.hex_x(columns, column)),
                area.y,
                end,
                &label,
                style,
            );
            put(
                cx,
                at(area.x, columns.ascii + column),
                area.y,
                end,
                &label[1..],
                style,
            );
        }
    }

    fn draw_row(&self, cx: &mut RenderCx<'_>, x: u16, y: u16, end: u16, row: usize) {
        let columns = self.columns(self.per_row);
        let start = row * self.per_row;
        let offset = self
            .base
            .saturating_add(u64::try_from(start).unwrap_or(u64::MAX));
        let mut digits = [b'0'; 16];
        for (place, digit) in digits[..columns.digits].iter_mut().rev().enumerate() {
            *digit = HEX[usize::try_from((offset >> (4 * place)) & 0xf).unwrap_or(0)];
        }
        let dim = self.dim_style(cx);
        put(cx, x, y, end, &digits[..columns.digits], dim);
        let bytes = &self.data[start..(start + self.per_row).min(self.data.len())];
        for (column, &byte) in bytes.iter().enumerate() {
            let index = start + column;
            let style = self.mark(cx, index, self.byte_style(cx, byte));
            let hex_x = self.hex_x(&columns, column);
            let pair = [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0xf)]];
            put(cx, at(x, hex_x), y, end, &pair, style);
            let shown = if (0x20..=0x7e).contains(&byte) {
                byte
            } else {
                b'.'
            };
            put(cx, at(x, columns.ascii + column), y, end, &[shown], style);
            if column + 1 < bytes.len() && self.selected(index) && self.selected(index + 1) {
                let next = self.hex_x(&columns, column + 1);
                let gap = Style::new().bg(cx.token(Token::Surface1));
                for between in hex_x + 2..next {
                    put(cx, at(x, between), y, end, b" ", gap);
                }
            }
        }
    }

    fn bar_styles(&self, cx: &RenderCx<'_>) -> (Style, Style) {
        let track = Style::new().fg(cx.token(Token::Surface1));
        let thumb = match self.state {
            WidgetState::Focused => Style::new().fg(cx.token(Token::FocusRing)),
            WidgetState::Disabled => Style::new()
                .fg(cx.token(Token::TextPrimary))
                .with_attrs(Attrs::new().set_dim(true)),
            _ => Style::new().fg(cx.token(Token::Accent)),
        };
        (track, thumb)
    }
}

fn at(x: u16, offset: usize) -> u16 {
    x.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX))
}

fn put(cx: &mut RenderCx<'_>, x: u16, y: u16, end: u16, text: &[u8], style: Style) {
    for (offset, &byte) in text.iter().enumerate() {
        let at = at(x, offset);
        if at >= end {
            break;
        }
        cx.put_char_patched(at, y, char::from(byte), style);
    }
}

impl Widget for HexViewer<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let columns = self.columns(self.per_row);
        let body = self.body(area);
        let end = self.text_end(area);
        if self.header {
            self.draw_header(cx, Rect::new(area.x, area.y, area.width, 1), &columns, end);
        }
        let top = self.first_row(body);
        let rows = self.row_count();
        for line in 0..body.height {
            let row = top + usize::from(line);
            if row >= rows {
                break;
            }
            self.draw_row(cx, area.x, body.y + line, end, row);
        }
        if self.has_bar(body) {
            let (track, thumb) = self.bar_styles(cx);
            Scrollbar::vertical(rows, usize::from(body.height), top)
                .track_style(track)
                .thumb_style(thumb)
                .render(Rect::new(end, body.y, 1, body.height), cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let header = usize::from(self.header);
        let rows = self.row_count();
        let bar = self.scrollbar && rows + header > usize::from(avail.height);
        let width = self.row_width() + usize::from(bar);
        Size::new(
            u16::try_from(width).unwrap_or(u16::MAX),
            u16::try_from(rows + header).unwrap_or(u16::MAX),
        )
        .clamp(avail)
    }
}
