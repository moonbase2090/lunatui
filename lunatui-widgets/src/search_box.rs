// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;
use std::ops::Range;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const PROMPT: &str = "/";
const CLEAR: &str = " × ";
const CLEAR_WIDTH: u16 = 3;
const MIN_WITH_CLEAR: u16 = 12;
const TEXT_AT: u16 = 3;

/// One item that matched a [`SearchBox::filter`] query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    /// Index of the item in the filtered slice.
    pub index: usize,
    /// Byte ranges of the item's text the query terms matched, sorted and not overlapping, for
    /// highlighting with [`SearchBox::put_highlighted`]. Empty for an empty query.
    pub ranges: Vec<Range<usize>>,
}

/// A one-row search field: a prompt, the query (or a placeholder while it is empty), a match count
/// on the right, and a clear button.
///
/// The app owns the query text and the caret. [`SearchBox::filter`] applies the query to a list of
/// items (smart case, every space-separated term must match) and returns the matches with the
/// ranges to highlight; [`SearchBox::step`] moves through them. The count shows "No matches" in the
/// fail color, a one-based position when [`matches`](SearchBox::matches) names the active match, or
/// the total otherwise. [`clear_area`](SearchBox::clear_area) is the cell to hit test for the
/// clear button, which appears once the query is not empty and the row is wide enough.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchBox<'a> {
    query: Cow<'a, str>,
    caret: usize,
    placeholder: Cow<'a, str>,
    matches: Option<(Option<usize>, usize)>,
    state: WidgetState,
    style: Option<Style>,
}

impl<'a> SearchBox<'a> {
    /// A field showing `query` with the caret at byte index `caret` (moved back to a character
    /// boundary, and to the end when past it).
    #[must_use]
    pub fn new(query: impl Into<Cow<'a, str>>, caret: usize) -> Self {
        Self {
            query: query.into(),
            caret,
            placeholder: Cow::Borrowed(""),
            matches: None,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// Dim text shown while the query is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// The number of matches, and which one is active (a zero-based index).
    #[must_use]
    pub const fn matches(mut self, active: Option<usize>, total: usize) -> Self {
        self.matches = Some((active, total));
        self
    }

    /// Interaction state: focused draws the prompt in the accent color and shows the caret,
    /// disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws the query in `style`'s foreground instead of the theme's text color.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// The items that match `query`, in order, with the ranges to highlight.
    ///
    /// The query is split on whitespace and an item matches when it contains every term. A term
    /// ignores ASCII case unless it has an uppercase letter. An empty query matches every item.
    #[must_use]
    pub fn filter<S: AsRef<str>>(items: &[S], query: &str) -> Vec<SearchMatch> {
        let terms: Vec<&str> = query.split_whitespace().collect();
        items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let ranges = term_ranges(item.as_ref(), &terms)?;
                Some(SearchMatch { index, ranges })
            })
            .collect()
    }

    /// The match after (or before) `current` out of `total`, wrapping around. From `None`, forward
    /// gives the first match and backward the last; `None` when there are no matches.
    #[must_use]
    pub fn step(current: Option<usize>, total: usize, forward: bool) -> Option<usize> {
        if total == 0 {
            return None;
        }
        Some(match (current.filter(|index| *index < total), forward) {
            (Some(index), true) => (index + 1) % total,
            (Some(index), false) => (index + total - 1) % total,
            (None, true) => 0,
            (None, false) => total - 1,
        })
    }

    /// Draws `text` at (`x`, `y`) in `base` with the byte `ranges` in `hit`, cut to `max_width`
    /// cells. The ranges are those of a [`SearchMatch`].
    pub fn put_highlighted(
        cx: &mut RenderCx<'_>,
        (x, y): (u16, u16),
        text: &str,
        ranges: &[Range<usize>],
        (base, hit): (Style, Style),
        max_width: u16,
    ) {
        let mut at = x;
        let mut left = max_width;
        let mut cursor = 0;
        let mut segments = Vec::new();
        for range in ranges {
            let (start, end) = (range.start.min(text.len()), range.end.min(text.len()));
            if start < cursor || start >= end {
                continue;
            }
            segments.push((&text[cursor..start], base));
            segments.push((&text[start..end], hit));
            cursor = end;
        }
        segments.push((&text[cursor..], base));
        for (segment, style) in segments {
            let clipped = clip_display_prefix(segment, left);
            cx.put_str(at, y, clipped, style);
            let used = u16::try_from(width(clipped)).unwrap_or(left);
            at = at.saturating_add(used);
            left = left.saturating_sub(used);
        }
    }

    fn has_clear(&self, area: Rect) -> bool {
        !self.query.is_empty() && area.width >= MIN_WITH_CLEAR
    }

    /// The cells of the clear button in `area` (empty while the query is empty or the row is
    /// narrower than twelve columns).
    #[must_use]
    pub fn clear_area(&self, area: Rect) -> Rect {
        if self.has_clear(area) {
            Rect::new(
                area.x + area.width - CLEAR_WIDTH,
                area.y,
                CLEAR_WIDTH,
                area.height.min(1),
            )
        } else {
            Rect::new(area.x + area.width, area.y, 0, area.height.min(1))
        }
    }

    /// The cells the query is drawn in, between the prompt and the count.
    #[must_use]
    pub fn query_area(&self, area: Rect) -> Rect {
        let start = TEXT_AT.min(area.width);
        let clear = self.clear_area(area).width;
        let text = self.count_text();
        let count = if text.is_empty() {
            0
        } else {
            u16::try_from(width(&text) + 2).unwrap_or(u16::MAX)
        };
        let right = if area.width > start + clear + count {
            clear + count
        } else {
            clear
        };
        Rect::new(
            area.x + start,
            area.y,
            area.width.saturating_sub(start).saturating_sub(right),
            area.height.min(1),
        )
    }

    fn count_text(&self) -> String {
        match self.matches {
            None => String::new(),
            Some((_, 0)) => "No matches".to_owned(),
            Some((Some(active), total)) => format!("{} of {total}", active + 1),
            Some((None, 1)) => "1 match".to_owned(),
            Some((None, total)) => format!("{total} matches"),
        }
    }

    fn caret_index(&self) -> usize {
        let mut index = self.caret.min(self.query.len());
        while !self.query.is_char_boundary(index) {
            index -= 1;
        }
        index
    }
}

fn term_ranges(text: &str, terms: &[&str]) -> Option<Vec<Range<usize>>> {
    let mut ranges = Vec::new();
    for term in terms {
        let exact = term.chars().any(char::is_uppercase);
        let before = ranges.len();
        let needle = term.as_bytes();
        let hay = text.as_bytes();
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
        if ranges.len() == before {
            return None;
        }
    }
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    Some(merged)
}

impl Widget for SearchBox<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let surface = cx.token(Token::Surface1);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), base);
        let prompt = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled || !focused).set_bold(focused));
        cx.put_str(area.x + 1.min(area.width - 1), area.y, PROMPT, prompt);
        let field = self.query_area(area);
        let text = match self.style {
            Some(style) => Style::new().fg(style.fg).bg(surface),
            None => base,
        };
        if self.query.is_empty() {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            cx.put_str(
                field.x,
                field.y,
                clip_display_prefix(&self.placeholder, field.width),
                dim,
            );
        } else {
            cx.put_str(
                field.x,
                field.y,
                clip_display_prefix(&self.query, field.width),
                text,
            );
        }
        if self.has_clear(area) {
            let clear = self.clear_area(area);
            cx.put_str(clear.x, clear.y, CLEAR, prompt);
        }
        let count = self.count_text();
        let clear = self.clear_area(area).width;
        let used = u16::try_from(width(&count)).unwrap_or(u16::MAX);
        let at = (area.x + area.width).saturating_sub(clear + used + 1);
        if at >= field.x + field.width {
            let fail = matches!(self.matches, Some((_, 0)));
            let style = if fail && !disabled {
                base.fg(cx.token(Token::Fail))
            } else {
                base.with_attrs(Attrs::new().set_dim(true))
            };
            cx.put_str(at, area.y, &count, style);
        }
        if focused {
            let caret = self.caret_index();
            let before = &self.query[..caret];
            let offset = u16::try_from(width(before)).unwrap_or(u16::MAX);
            let at = field.x.saturating_add(offset);
            if at < field.x + field.width.max(1) {
                let under = self.query[caret..].chars().next().unwrap_or(' ');
                let inverted = Style::new().fg(surface).bg(cx.token(Token::Accent));
                cx.put_str(at, field.y, &under.to_string(), inverted);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1)
    }
}
