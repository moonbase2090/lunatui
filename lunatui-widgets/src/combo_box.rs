// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;
use std::ops::Range;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, SearchBox, SearchMatch, Widget, WidgetState};

const CLOSED: &str = " ▾ ";
const OPEN: &str = " ▴ ";
const INDICATOR_WIDTH: u16 = 3;
const MIN_WITH_INDICATOR: u16 = 8;
const DEFAULT_ROWS: u16 = 6;

/// A one-row text field with a drop-down list of the options that match what was typed.
///
/// The app owns the text, the caret, whether the list is [`open`](ComboBox::open), and which row is
/// [`active`](ComboBox::active). The options are filtered with [`SearchBox::filter`] (smart case,
/// every space-separated term must match) and the matched part of each is drawn highlighted.
/// [`matches`](ComboBox::matches) returns the filtered options, so the app can map the active row
/// back to an option, and [`SearchBox::step`] moves through them. The list shows at most
/// [`max_rows`](ComboBox::max_rows) rows, scrolled to keep the active one in view, and says "No
/// matches" when nothing fits. [`field_area`](ComboBox::field_area) and
/// [`option_at`](ComboBox::option_at) are for hit testing.
///
/// Filtering allocates. An app that draws every frame can run [`SearchBox::filter`] when the text
/// changes and pass the result to [`with_matches`](ComboBox::with_matches); drawing then allocates
/// nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboBox<'a, S = &'a str> {
    text: Cow<'a, str>,
    caret: usize,
    options: &'a [S],
    found: Option<&'a [SearchMatch]>,
    placeholder: Cow<'a, str>,
    open: bool,
    active: Option<usize>,
    max_rows: u16,
    state: WidgetState,
}

impl<'a, S: AsRef<str>> ComboBox<'a, S> {
    /// A field showing `text` with the caret at byte index `caret` (moved back to a character
    /// boundary, and to the end when past it), offering `options`.
    #[must_use]
    pub fn new(text: impl Into<Cow<'a, str>>, caret: usize, options: &'a [S]) -> Self {
        Self {
            text: text.into(),
            caret,
            options,
            found: None,
            placeholder: Cow::Borrowed(""),
            open: false,
            active: None,
            max_rows: DEFAULT_ROWS,
            state: WidgetState::Normal,
        }
    }

    /// Dim text shown while the field is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Whether the list of matching options is drawn below the field.
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The highlighted row of the list, as a position in [`matches`](ComboBox::matches).
    #[must_use]
    pub const fn active(mut self, active: Option<usize>) -> Self {
        self.active = active;
        self
    }

    /// The most rows the list may take (at least one). Six by default.
    #[must_use]
    pub fn max_rows(mut self, rows: u16) -> Self {
        self.max_rows = rows.max(1);
        self
    }

    /// Interaction state: focused draws the caret and the active row in the accent color,
    /// disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Uses `found`, the result of [`SearchBox::filter`] for this text and these options, instead
    /// of filtering on every draw and hit test. The app keeps it in step with the text.
    #[must_use]
    pub const fn with_matches(mut self, found: &'a [SearchMatch]) -> Self {
        self.found = Some(found);
        self
    }

    /// The options that match the text, in order, with the ranges to highlight.
    #[must_use]
    pub fn matches(&self) -> Vec<SearchMatch> {
        self.found.map_or_else(
            || SearchBox::filter(self.options, &self.text),
            <[SearchMatch]>::to_vec,
        )
    }

    fn found(&self) -> Cow<'_, [SearchMatch]> {
        self.found.map_or_else(
            || Cow::Owned(SearchBox::filter(self.options, &self.text)),
            Cow::Borrowed,
        )
    }

    fn rows_for(&self, found: usize, height: u16) -> u16 {
        if !self.open {
            return 0;
        }
        let wanted = u16::try_from(found.max(1)).unwrap_or(u16::MAX);
        wanted.min(self.max_rows).min(height.saturating_sub(1))
    }

    /// The row of the text field in `area`.
    #[must_use]
    pub fn field_area(&self, area: Rect) -> Rect {
        Rect::new(area.x, area.y, area.width, area.height.min(1))
    }

    /// The cells of the drop-down list in `area` (empty while it is closed).
    #[must_use]
    pub fn list_area(&self, area: Rect) -> Rect {
        let rows = self.rows_for(self.found().len(), area.height);
        Rect::new(area.x, area.y + 1, area.width, rows)
    }

    /// The position in [`matches`](ComboBox::matches) of the list row at (`x`, `y`), if any.
    #[must_use]
    pub fn option_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let found = self.found().len();
        let list = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            self.rows_for(found, area.height),
        );
        let inside =
            x >= list.x && x - list.x < list.width && y >= list.y && y - list.y < list.height;
        let index = self.first_row(found, list.height) + usize::from(y.saturating_sub(list.y));
        (inside && index < found).then_some(index)
    }

    fn first_row(&self, found: usize, rows: u16) -> usize {
        let active = self.active.filter(|active| *active < found).unwrap_or(0);
        (active + 1).saturating_sub(usize::from(rows))
    }

    fn text_area(area: Rect) -> Rect {
        let right = if area.width >= MIN_WITH_INDICATOR {
            INDICATOR_WIDTH
        } else {
            1
        };
        Rect::new(
            area.x + 1.min(area.width),
            area.y,
            area.width.saturating_sub(1 + right),
            area.height.min(1),
        )
    }

    fn caret_index(&self) -> usize {
        let mut index = self.caret.min(self.text.len());
        while !self.text.is_char_boundary(index) {
            index -= 1;
        }
        index
    }

    fn render_field(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let surface = cx.token(Token::Surface1);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        cx.fill(Rect::new(area.x, area.y, area.width, 1), base, ' ');
        let field = Self::text_area(area);
        if self.text.is_empty() {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            let shown = clip_display_prefix(&self.placeholder, field.width);
            cx.put_str(field.x, field.y, shown, dim);
        } else {
            let shown = clip_display_prefix(&self.text, field.width);
            cx.put_str(field.x, field.y, shown, base);
        }
        if area.width >= MIN_WITH_INDICATOR {
            let chevron = Style::new()
                .fg(cx.token(Token::Accent))
                .bg(surface)
                .with_attrs(Attrs::new().set_dim(disabled || !focused).set_bold(focused));
            let at = area.x + area.width - INDICATOR_WIDTH;
            cx.put_str(at, area.y, if self.open { OPEN } else { CLOSED }, chevron);
        }
        if focused {
            let caret = self.caret_index();
            let offset = u16::try_from(width(&self.text[..caret])).unwrap_or(u16::MAX);
            let at = field.x.saturating_add(offset);
            if at < field.x + field.width {
                let under = self.text[caret..].chars().next().unwrap_or(' ');
                let inverted = Style::new().fg(surface).bg(cx.token(Token::Accent));
                cx.put_str(at, field.y, under.encode_utf8(&mut [0; 4]), inverted);
            }
        }
    }

    fn render_list(&self, area: Rect, found: &[SearchMatch], rows: u16, cx: &mut RenderCx<'_>) {
        let list = Rect::new(area.x, area.y + 1, area.width, rows);
        let disabled = matches!(self.state, WidgetState::Disabled);
        let surface = cx.token(Token::Surface0);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        if found.is_empty() {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            cx.fill(Rect::new(list.x, list.y, list.width, 1), base, ' ');
            let message = clip_display_prefix("No matches", list.width.saturating_sub(2));
            cx.put_str(list.x + 1, list.y, message, dim);
            return;
        }
        let first = self.first_row(found.len(), list.height);
        let accent = cx.token(Token::Accent);
        for row in 0..list.height {
            let position = first + usize::from(row);
            let y = list.y + row;
            let Some(hit) = found.get(position) else {
                cx.fill(Rect::new(list.x, y, list.width, 1), base, ' ');
                continue;
            };
            let current = self.active == Some(position);
            let (row_base, mark) = if current {
                let selected = Style::new().fg(surface).bg(accent);
                (selected, Style::new().fg(accent).bg(surface))
            } else {
                (base, base.fg(surface).bg(accent))
            };
            cx.fill(Rect::new(list.x, y, list.width, 1), row_base, ' ');
            put_marked(
                cx,
                (list.x + 1, y),
                self.options[hit.index].as_ref(),
                &hit.ranges,
                (row_base, mark),
                list.width.saturating_sub(2),
            );
        }
    }
}

fn put_marked(
    cx: &mut RenderCx<'_>,
    (x, y): (u16, u16),
    text: &str,
    ranges: &[Range<usize>],
    (base, hit): (Style, Style),
    max_width: u16,
) {
    let mut at = x;
    let mut left = max_width;
    let mut put = |cx: &mut RenderCx<'_>, shown: &str, style: Style| {
        let clipped = clip_display_prefix(shown, left);
        cx.put_str(at, y, clipped, style);
        let used = u16::try_from(width(clipped)).unwrap_or(left);
        at = at.saturating_add(used);
        left = left.saturating_sub(used);
    };
    let mut cursor = 0;
    for range in ranges {
        let (Some(before), Some(marked)) = (text.get(cursor..range.start), text.get(range.clone()))
        else {
            continue;
        };
        put(cx, before, base);
        put(cx, marked, hit);
        cursor = range.end;
    }
    put(cx, text.get(cursor..).unwrap_or(""), base);
}

impl<S: AsRef<str>> Widget for ComboBox<'_, S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.render_field(area, cx);
        let found = self.found();
        let rows = self.rows_for(found.len(), area.height);
        if rows > 0 {
            self.render_list(area, &found, rows, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = self.rows_for(self.found().len(), avail.height);
        Size::new(avail.width, 1 + rows)
    }
}
