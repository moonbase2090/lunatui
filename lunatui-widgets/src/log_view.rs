// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Range;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const LEVELS: usize = 5;

/// How serious a log entry is, from least to most.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Fine-grained detail.
    Trace,
    /// Information for debugging.
    Debug,
    /// Normal progress.
    Info,
    /// Something unexpected that the program handled.
    Warn,
    /// Something failed.
    Error,
}

impl LogLevel {
    const fn index(self) -> usize {
        match self {
            Self::Trace => 0,
            Self::Debug => 1,
            Self::Info => 2,
            Self::Warn => 3,
            Self::Error => 4,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => " INFO",
            Self::Warn => " WARN",
            Self::Error => "ERROR",
        }
    }
}

/// One line of a [`LogBuffer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// How serious the entry is.
    pub level: LogLevel,
    /// Where it came from, such as a module path.
    pub target: String,
    /// The text; only its first line is drawn.
    pub message: String,
    /// A time label the app formats, drawn before the level. Empty for none.
    pub stamp: String,
}

impl LogEntry {
    /// An entry with no time label.
    #[must_use]
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            target: target.into(),
            message: message.into(),
            stamp: String::new(),
        }
    }

    /// The same entry with a time label such as `12:04:31`.
    #[must_use]
    pub fn stamp(mut self, stamp: impl Into<String>) -> Self {
        self.stamp = stamp.into();
        self
    }
}

/// A ring buffer of [`LogEntry`] values: once it holds `capacity` entries, each new one pushes out
/// the oldest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    capacity: usize,
    dropped: u64,
}

impl LogBuffer {
    /// An empty buffer that keeps at most `capacity` entries (at least one).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            dropped: 0,
        }
    }

    /// Adds an entry as the newest, dropping the oldest when the buffer is full.
    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
            self.dropped += 1;
        }
        self.entries.push_back(entry);
    }

    /// Removes every entry. The count of dropped entries is kept.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The number of entries held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the buffer holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries were pushed out to stay within the capacity.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    /// How many entries there are at each level, from trace to error.
    #[must_use]
    pub fn counts(&self) -> [usize; LEVELS] {
        let mut counts = [0; LEVELS];
        for entry in &self.entries {
            counts[entry.level.index()] += 1;
        }
        counts
    }
}

/// A scrolling list of log entries with a level colored label, an optional target column, and
/// filters by minimum level, target prefix, and text.
///
/// The widget borrows a [`LogBuffer`] the app fills as logs arrive. With [`follow`](LogView::follow)
/// (the default) the newest matching entry sits on the last row, so new entries scroll in from the
/// bottom. Without it, [`scroll`](LogView::scroll) counts the matching entries to leave out below
/// the view, so a value of zero is the same as following. The text filter keeps entries whose
/// message or target contains the query (smart case: ignoring ASCII case unless the query has an
/// uppercase letter) and draws the matched part of the message highlighted.
/// [`matching`](LogView::matching) and [`max_scroll`](LogView::max_scroll) report the numbers an
/// app needs for a scrollbar or a "N more" hint.
#[derive(Clone, Debug)]
pub struct LogView<'a> {
    buffer: &'a LogBuffer,
    min_level: LogLevel,
    target: Cow<'a, str>,
    query: Cow<'a, str>,
    follow: bool,
    scroll: usize,
    target_width: u16,
    state: WidgetState,
}

impl<'a> LogView<'a> {
    /// A view of `buffer` showing every entry, following the newest.
    #[must_use]
    pub const fn new(buffer: &'a LogBuffer) -> Self {
        Self {
            buffer,
            min_level: LogLevel::Trace,
            target: Cow::Borrowed(""),
            query: Cow::Borrowed(""),
            follow: true,
            scroll: 0,
            target_width: 0,
            state: WidgetState::Normal,
        }
    }

    /// Hides entries below `level`.
    #[must_use]
    pub const fn min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Shows only entries whose target starts with `prefix`.
    #[must_use]
    pub fn target(mut self, prefix: impl Into<Cow<'a, str>>) -> Self {
        self.target = prefix.into();
        self
    }

    /// Shows only entries whose message or target contains `query`, and highlights it.
    #[must_use]
    pub fn query(mut self, query: impl Into<Cow<'a, str>>) -> Self {
        self.query = query.into();
        self
    }

    /// Whether the last row always shows the newest matching entry. On by default.
    #[must_use]
    pub const fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    /// With following off, the number of matching entries left out below the view.
    #[must_use]
    pub const fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    /// Draws each entry's target in a column of `columns` cells (clipped, padded). Zero hides it.
    #[must_use]
    pub const fn target_width(mut self, columns: u16) -> Self {
        self.target_width = columns;
        self
    }

    /// Interaction state: disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    fn passes(&self, entry: &LogEntry) -> bool {
        entry.level >= self.min_level
            && entry.target.starts_with(self.target.as_ref())
            && (self.query.is_empty()
                || !text_ranges(&entry.message, &self.query).is_empty()
                || !text_ranges(&entry.target, &self.query).is_empty())
    }

    /// How many entries pass the filters.
    #[must_use]
    pub fn matching(&self) -> usize {
        self.buffer
            .iter()
            .filter(|entry| self.passes(entry))
            .count()
    }

    /// The largest useful [`scroll`](LogView::scroll) for a view `rows` tall.
    #[must_use]
    pub fn max_scroll(&self, rows: u16) -> usize {
        self.matching().saturating_sub(usize::from(rows))
    }

    fn hidden_below(&self, rows: u16) -> usize {
        if self.follow {
            0
        } else {
            self.scroll.min(self.max_scroll(rows))
        }
    }

    fn put_entry(&self, cx: &mut RenderCx<'_>, entry: &LogEntry, (x, y): (u16, u16), room: u16) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0))
            .with_attrs(Attrs::new().set_dim(disabled));
        let dim = text.with_attrs(Attrs::new().set_dim(true));
        let mut at = x;
        let mut left = room;
        let mut put = |cx: &mut RenderCx<'_>, shown: &str, style: Style, gap: bool| {
            let clipped = clip_display_prefix(shown, left);
            cx.put_str(at, y, clipped, style);
            let used = u16::try_from(width(clipped)).unwrap_or(left);
            let used = if gap { (used + 1).min(left) } else { used };
            at = at.saturating_add(used);
            left = left.saturating_sub(used);
        };
        if !entry.stamp.is_empty() {
            put(cx, &entry.stamp, dim, true);
        }
        put(
            cx,
            entry.level.label(),
            self.level_style(cx, entry.level),
            true,
        );
        if self.target_width > 0 {
            let column = clip_display_prefix(&entry.target, self.target_width);
            let pad = usize::from(self.target_width).saturating_sub(width(column));
            put(cx, &format!("{column}{}", " ".repeat(pad)), dim, true);
        }
        let message = entry.message.lines().next().unwrap_or("");
        let hit = text
            .bg(cx.token(Token::Accent))
            .fg(cx.token(Token::Surface0));
        let ranges = text_ranges(message, &self.query);
        let mut cursor = 0;
        for range in ranges {
            put(cx, &message[cursor..range.start], text, false);
            put(cx, &message[range.clone()], hit, false);
            cursor = range.end;
        }
        put(cx, &message[cursor..], text, false);
    }

    fn level_style(&self, cx: &RenderCx<'_>, level: LogLevel) -> Style {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let (token, bold, dim) = match level {
            LogLevel::Trace => (Token::TextPrimary, false, true),
            LogLevel::Debug => (Token::Accent, false, false),
            LogLevel::Info => (Token::Ok, false, false),
            LogLevel::Warn => (Token::Warn, true, false),
            LogLevel::Error => (Token::Fail, true, false),
        };
        Style::new()
            .fg(cx.token(token))
            .bg(cx.token(Token::Surface0))
            .with_attrs(Attrs::new().set_bold(bold).set_dim(dim || disabled))
    }
}

fn text_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let needle = query.as_bytes();
    if needle.is_empty() {
        return Vec::new();
    }
    let exact = query.chars().any(char::is_uppercase);
    let hay = text.as_bytes();
    let mut ranges = Vec::new();
    let mut from = 0;
    while from + needle.len() <= hay.len() {
        let window = &hay[from..from + needle.len()];
        let same = if exact {
            window == needle
        } else {
            window.eq_ignore_ascii_case(needle)
        };
        if same {
            ranges.push(from..from + needle.len());
            from += needle.len();
        } else {
            from += 1;
        }
    }
    ranges
}

impl Widget for LogView<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let background = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let blank = " ".repeat(usize::from(area.width));
        for row in 0..area.height {
            cx.put_str(area.x, area.y + row, &blank, background);
        }
        let total = self.matching();
        let rows = usize::from(area.height);
        let end = total - self.hidden_below(area.height);
        let start = end.saturating_sub(rows);
        let shown = self
            .buffer
            .iter()
            .filter(|entry| self.passes(entry))
            .skip(start)
            .take(end - start);
        let top = area.y + u16::try_from(rows - (end - start)).unwrap_or(0);
        for (offset, entry) in shown.enumerate() {
            let y = top + u16::try_from(offset).unwrap_or(0);
            self.put_entry(cx, entry, (area.x, y), area.width);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let rows = u16::try_from(self.matching()).unwrap_or(u16::MAX);
        Size::new(avail.width, rows.min(avail.height))
    }
}
