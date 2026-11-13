// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt::{self, Write};

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, cluster_display_width, width};
use lunatui_theme::Token;
use unicode_segmentation::UnicodeSegmentation;

use crate::{RenderCx, Widget, WidgetState};

const HEADER_ROWS: u16 = 3;
const FOOTER_ROWS: u16 = 1;
const SIZE_WIDTH: u16 = 9;
const MODIFIED_WIDTH: u16 = 13;
const MARK_WIDTH: u16 = 2;
const ICON_WIDTH: u16 = 2;
const SHOW_SIZE_AT: u16 = 30;
const SHOW_MODIFIED_AT: u16 = 50;
const ELLIPSIS: &str = "…";
const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// What a [`FileEntry`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// A folder the picker can open.
    Directory,
    /// A file the picker can choose.
    File,
}

/// One row the app lists for the folder being browsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileEntry<'a> {
    name: &'a str,
    kind: FileKind,
    size: Option<u64>,
    modified: &'a str,
}

impl<'a> FileEntry<'a> {
    /// A folder named `name`.
    #[must_use]
    pub const fn directory(name: &'a str) -> Self {
        Self {
            name,
            kind: FileKind::Directory,
            size: None,
            modified: "",
        }
    }

    /// A file named `name` of `size` bytes.
    #[must_use]
    pub const fn file(name: &'a str, size: u64) -> Self {
        Self {
            name,
            kind: FileKind::File,
            size: Some(size),
            modified: "",
        }
    }

    /// The modified time as the app wants it shown, such as `Oct 31 14:02`.
    #[must_use]
    pub const fn modified(mut self, modified: &'a str) -> Self {
        self.modified = modified;
        self
    }

    /// The modified time text the app gave the entry, empty if none.
    #[must_use]
    pub const fn modified_text(&self) -> &'a str {
        self.modified
    }

    /// The entry's name.
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Whether the entry is a folder or a file.
    #[must_use]
    pub const fn kind(&self) -> FileKind {
        self.kind
    }
}

/// A file browser for a listing the app provides.
///
/// The widget does no filesystem access. The app lists the folder it is in, sorts and filters
/// that listing, and owns the [`active`](FilePicker::active) row, the
/// [`marked`](FilePicker::marked) rows and the filter text. The picker draws the path, the
/// filter, a header, one row per entry with its size and modified time, and a count line. The
/// list scrolls to keep the active row in view and shows a scroll thumb when it overflows.
/// Narrow areas drop the modified column, then the size column. [`step`](FilePicker::step),
/// [`toggle`](FilePicker::toggle), [`parent`](FilePicker::parent) and
/// [`matches`](FilePicker::matches) are the pure helpers for the app's key handling,
/// [`FilePickerState`](crate::FilePickerState) handles the keys, type-ahead included, and [`list_area`](FilePicker::list_area) and
/// [`entry_at`](FilePicker::entry_at) are for hit testing. Drawing allocates nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct FilePicker<'a> {
    path: &'a str,
    entries: &'a [FileEntry<'a>],
    active: Option<usize>,
    marked: &'a [usize],
    filter: &'a str,
    state: WidgetState,
}

impl<'a> FilePicker<'a> {
    /// A picker showing `entries` of the folder at `path`.
    #[must_use]
    pub const fn new(path: &'a str, entries: &'a [FileEntry<'a>]) -> Self {
        Self {
            path,
            entries,
            active: None,
            marked: &[],
            filter: "",
            state: WidgetState::Normal,
        }
    }

    /// The highlighted row, as an index into the entries.
    #[must_use]
    pub const fn active(mut self, active: Option<usize>) -> Self {
        self.active = active;
        self
    }

    /// The rows chosen so far, as indices into the entries, drawn with a check.
    #[must_use]
    pub const fn marked(mut self, marked: &'a [usize]) -> Self {
        self.marked = marked;
        self
    }

    /// The text of the filter the app applied to the listing.
    #[must_use]
    pub const fn filter(mut self, filter: &'a str) -> Self {
        self.filter = filter;
        self
    }

    /// Interaction state: focused draws a cursor in the filter and the active row in the accent
    /// color, disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The row `delta` steps from `current` out of `total`, stopping at the first and last row.
    /// From `None`, any step lands on the first row. `None` when there are no rows.
    #[must_use]
    pub fn step(current: Option<usize>, total: usize, delta: isize) -> Option<usize> {
        let from = current.filter(|index| *index < total);
        if from.is_none() {
            return (total > 0).then_some(0);
        }
        lunatui_runtime::RovingFocus::step_by(from, total, delta, false, |_| true)
    }

    /// Adds `index` to the sorted `marked` rows, or removes it when it is already there.
    pub fn toggle(marked: &mut Vec<usize>, index: usize) {
        match marked.binary_search(&index) {
            Ok(at) => {
                marked.remove(at);
            }
            Err(at) => marked.insert(at, index),
        }
    }

    /// The folder that holds `path`: everything before its last `/`, `/` for a top-level
    /// folder, and `path` itself at the root or when it has no `/`.
    #[must_use]
    pub fn parent(path: &str) -> &str {
        let trimmed = path.trim_end_matches('/');
        match trimmed.rfind('/') {
            Some(0) => "/",
            Some(at) => &trimmed[..at],
            None => path,
        }
    }

    /// Whether `name` contains `query`, ignoring case. Everything matches an empty query.
    #[must_use]
    pub fn matches(name: &str, query: &str) -> bool {
        query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
    }

    /// The cells the entry rows take in `area`.
    #[must_use]
    pub fn list_area(&self, area: Rect) -> Rect {
        let top = HEADER_ROWS.min(area.height);
        let rows = area
            .height
            .saturating_sub(HEADER_ROWS + FOOTER_ROWS)
            .min(area.height);
        Rect::new(area.x, area.y + top, area.width, rows)
    }

    /// The index of the entry on the row at (`x`, `y`), if any.
    #[must_use]
    pub fn entry_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let list = self.list_area(area);
        let inside =
            x >= list.x && x - list.x < list.width && y >= list.y && y - list.y < list.height;
        let index = self.first_row(list.height) + usize::from(y.saturating_sub(list.y));
        (inside && index < self.entries.len()).then_some(index)
    }

    fn active_index(&self) -> Option<usize> {
        self.active.filter(|index| *index < self.entries.len())
    }

    fn first_row(&self, rows: u16) -> usize {
        let active = self.active_index().unwrap_or(0);
        (active + 1).saturating_sub(usize::from(rows))
    }

    fn render_path(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let disabled = self.state == WidgetState::Disabled;
        let bar = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1))
            .with_attrs(Attrs::new().set_dim(disabled));
        let row = Rect::new(area.x, area.y, area.width, 1);
        cx.fill(row, bar, ' ');
        let accent = bar.fg(cx.token(Token::Accent));
        cx.put_str(area.x + 1.min(area.width), area.y, "▸", accent);
        let room = area.width.saturating_sub(3);
        let (shown, clipped) = clip_display_suffix(self.path, room);
        let mut x = area.x + 3.min(area.width);
        if clipped {
            cx.put_str(x, area.y, ELLIPSIS, bar);
            x += 1;
        }
        cx.put_str(x, area.y, shown, bar);
    }

    fn render_filter(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let disabled = self.state == WidgetState::Disabled;
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0))
            .with_attrs(Attrs::new().set_dim(disabled));
        let dim = base.with_attrs(Attrs::new().set_dim(true));
        let y = area.y + 1;
        cx.fill(Rect::new(area.x, y, area.width, 1), base, ' ');
        cx.put_str(area.x + 1.min(area.width), y, "Filter", dim);
        let text_x = area.x + 8.min(area.width);
        let room = area.width.saturating_sub(9);
        if self.filter.is_empty() {
            let gap = u16::from(self.state == WidgetState::Focused);
            let hint = clip_display_prefix("type to narrow the list", room.saturating_sub(gap));
            cx.put_str(text_x + gap, y, hint, dim);
        } else {
            let shown = clip_display_prefix(self.filter, room);
            cx.put_str(text_x, y, shown, base);
        }
        if self.state == WidgetState::Focused {
            let used = u16::try_from(width(clip_display_prefix(self.filter, room))).unwrap_or(0);
            if used < room {
                let accent = base.fg(cx.token(Token::Accent));
                cx.put_str(text_x + used, y, "▏", accent);
            }
        }
    }

    fn columns(area_width: u16) -> (u16, u16) {
        let modified = if area_width >= SHOW_MODIFIED_AT {
            MODIFIED_WIDTH
        } else {
            0
        };
        let size = if area_width >= SHOW_SIZE_AT {
            SIZE_WIDTH
        } else {
            0
        };
        (size, modified)
    }

    const fn size_end(right: u16, modified: u16) -> u16 {
        right - modified - if modified > 0 { 2 } else { 1 }
    }

    fn render_header(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let y = area.y + 2;
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0))
            .with_attrs(Attrs::new().set_dim(true).set_bold(true));
        cx.fill(Rect::new(area.x, y, area.width, 1), style, ' ');
        cx.put_str(
            area.x + (MARK_WIDTH + ICON_WIDTH).min(area.width),
            y,
            "Name",
            style,
        );
        let (size, modified) = Self::columns(area.width);
        let thumb = u16::from(self.overflows(self.list_area(area).height));
        let right = area.x + area.width - thumb;
        if size > 0 {
            cx.put_str(Self::size_end(right, modified) - 4, y, "Size", style);
        }
        if modified > 0 {
            cx.put_str(right - modified, y, "Modified", style);
        }
    }

    fn render_row(
        &self,
        list: Rect,
        row: u16,
        index: usize,
        entry: &FileEntry<'_>,
        cx: &mut RenderCx<'_>,
    ) {
        let disabled = self.state == WidgetState::Disabled;
        let surface = cx.token(Token::Surface0);
        let accent = cx.token(Token::Accent);
        let text = cx.token(Token::TextPrimary);
        let active = self.active_index() == Some(index);
        let marked = self.marked.binary_search(&index).is_ok();
        let y = list.y + row;
        let base = if active {
            Style::new().fg(surface).bg(accent)
        } else {
            Style::new().fg(text).bg(surface)
        }
        .with_attrs(Attrs::new().set_dim(disabled).set_bold(marked));
        cx.fill(Rect::new(list.x, y, list.width, 1), base, ' ');
        if marked && list.width >= 2 {
            let check = if active { base } else { base.fg(accent) };
            cx.put_str(list.x + 1, y, "✓", check);
        }
        let is_dir = entry.kind == FileKind::Directory;
        let name_style = if is_dir && !active {
            base.fg(accent)
        } else {
            base
        };
        let icon_x = list.x + MARK_WIDTH.min(list.width);
        if is_dir {
            cx.put_str(icon_x, y, "▸", name_style);
        }
        let (size, modified) = Self::columns(list.width);
        let thumb = u16::from(self.overflows(list.height));
        let right = list.x + list.width - thumb;
        let reserved = size + modified + thumb;
        let name_x = list.x + (MARK_WIDTH + ICON_WIDTH).min(list.width);
        let room = list
            .width
            .saturating_sub(MARK_WIDTH + ICON_WIDTH + reserved);
        let shown = clip_display_prefix(entry.name, room);
        cx.put_str(name_x, y, shown, name_style);
        if is_dir && usize::from(room) > width(shown) {
            let after = u16::try_from(width(shown)).unwrap_or(0);
            cx.put_str(name_x + after, y, "/", name_style);
        }
        if let (true, Some(bytes)) = (size > 0, entry.size) {
            let mut text = Stack::<12>::new();
            let _ = write_size(&mut text, bytes);
            cx.put_str(
                Self::size_end(right, modified) - text.width(),
                y,
                text.as_str(),
                base,
            );
        }
        if modified > 0 {
            let shown = clip_display_prefix(entry.modified, modified);
            cx.put_str(
                right - modified,
                y,
                shown,
                base.with_attrs(base.attrs.set_dim(true)),
            );
        }
    }

    fn overflows(&self, rows: u16) -> bool {
        self.entries.len() > usize::from(rows)
    }

    fn render_list(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let list = self.list_area(area);
        if list.height == 0 {
            return;
        }
        let surface = cx.token(Token::Surface0);
        let base = Style::new().fg(cx.token(Token::TextPrimary)).bg(surface);
        if self.entries.is_empty() {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            for row in 0..list.height {
                cx.fill(Rect::new(list.x, list.y + row, list.width, 1), base, ' ');
            }
            let message = clip_display_prefix("This folder is empty", list.width.saturating_sub(4));
            cx.put_str(list.x + 4.min(list.width), list.y, message, dim);
            return;
        }
        let first = self.first_row(list.height);
        for row in 0..list.height {
            match self.entries.get(first + usize::from(row)) {
                Some(entry) => self.render_row(list, row, first + usize::from(row), entry, cx),
                None => cx.fill(Rect::new(list.x, list.y + row, list.width, 1), base, ' '),
            }
        }
        if self.overflows(list.height) {
            self.render_thumb(list, first, cx);
        }
    }

    fn render_thumb(&self, list: Rect, first: usize, cx: &mut RenderCx<'_>) {
        let rows = usize::from(list.height);
        let total = self.entries.len();
        let length = (rows * rows / total).max(1);
        let travel = rows - length;
        let position = first * travel / (total - rows);
        let accent = cx.token(Token::Accent);
        let surface = cx.token(Token::Surface0);
        let x = list.x + list.width - 1;
        for row in 0..rows {
            let on = row >= position && row < position + length;
            let (glyph, style) = if on {
                ("┃", Style::new().fg(accent).bg(surface))
            } else {
                (
                    "│",
                    Style::new()
                        .fg(cx.token(Token::TextPrimary))
                        .bg(surface)
                        .with_attrs(Attrs::new().set_dim(true)),
                )
            };
            cx.put_str(x, list.y + u16::try_from(row).unwrap_or(0), glyph, style);
        }
    }

    fn render_footer(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let y = area.y + area.height - 1;
        let style = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1))
            .with_attrs(Attrs::new().set_dim(true));
        cx.fill(Rect::new(area.x, y, area.width, 1), style, ' ');
        let mut text = Stack::<48>::new();
        let items = self.entries.len();
        let _ = write!(
            text,
            "{items} {}",
            if items == 1 { "item" } else { "items" }
        );
        if !self.marked.is_empty() {
            let _ = write!(text, "  ·  {} selected", self.marked.len());
        }
        let shown = clip_display_prefix(text.as_str(), area.width.saturating_sub(2));
        cx.put_str(area.x + 1.min(area.width), y, shown, style);
    }
}

fn clip_display_suffix(text: &str, max_cols: u16) -> (&str, bool) {
    if width(text) <= usize::from(max_cols) {
        return (text, false);
    }
    let budget = usize::from(max_cols.saturating_sub(1));
    let mut used = 0;
    let mut start = text.len();
    for (index, grapheme) in text.grapheme_indices(true).rev() {
        let cells = cluster_display_width(grapheme);
        if used + cells > budget {
            break;
        }
        used += cells;
        start = index;
    }
    (&text[start..], true)
}

fn write_size(out: &mut impl Write, bytes: u64) -> fmt::Result {
    let mut unit = 0;
    let mut tenths = bytes.saturating_mul(10);
    while tenths >= 10_240 && unit + 1 < UNITS.len() {
        tenths = (tenths + 512) / 1024;
        unit += 1;
    }
    let whole = if unit == 0 { bytes } else { tenths / 10 };
    if unit == 0 || whole >= 10 {
        let whole = if unit == 0 { whole } else { (tenths + 5) / 10 };
        write!(out, "{whole} {}", UNITS[unit])
    } else {
        write!(out, "{whole}.{} {}", tenths % 10, UNITS[unit])
    }
}

struct Stack<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Stack<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    fn width(&self) -> u16 {
        u16::try_from(width(self.as_str())).unwrap_or(u16::MAX)
    }
}

impl<const N: usize> Write for Stack<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        let slot = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        slot.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl Widget for FilePicker<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height < HEADER_ROWS + FOOTER_ROWS {
            return;
        }
        self.render_path(area, cx);
        self.render_filter(area, cx);
        self.render_header(area, cx);
        self.render_list(area, cx);
        self.render_footer(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = u16::try_from(self.entries.len().max(1)).unwrap_or(u16::MAX);
        Size::new(
            avail.width,
            (HEADER_ROWS + FOOTER_ROWS).saturating_add(rows),
        )
        .clamp(avail)
    }
}
