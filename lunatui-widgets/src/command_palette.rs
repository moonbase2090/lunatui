// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{BorderType, Popup, RenderCx, SearchBox, SearchMatch, Widget};

const DEFAULT_WIDTH: u16 = 60;
const DEFAULT_ROWS: u16 = 8;
const PROMPT: &str = "› ";
const PROMPT_WIDTH: u16 = 2;

/// One command in a [`CommandPalette`]: a title, an optional muted detail and shortcut hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteEntry<'a> {
    title: &'a str,
    detail: &'a str,
    shortcut: &'a str,
}

impl<'a> PaletteEntry<'a> {
    /// An entry with this title, matched against the query.
    #[must_use]
    pub const fn new(title: &'a str) -> Self {
        Self {
            title,
            detail: "",
            shortcut: "",
        }
    }

    /// Muted text after the title, not matched against the query.
    #[must_use]
    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    /// The key hint drawn at the right edge, such as `Ctrl+S`.
    #[must_use]
    pub const fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = shortcut;
        self
    }

    /// The title.
    #[must_use]
    pub const fn title(&self) -> &'a str {
        self.title
    }
}

impl AsRef<str> for PaletteEntry<'_> {
    fn as_ref(&self) -> &str {
        self.title
    }
}

/// A centred command palette: a query field over the commands that match it.
///
/// The panel is a [`Popup`] with a `›` prompt row on top and one row per matching entry below,
/// scrolled to keep the selected row in view. Entries match the query as in [`SearchBox::filter`],
/// with the matched text highlighted. The app keeps the query, its caret and the selected position
/// among [`matches`](Self::matches) (move it with [`SearchBox::step`]) and runs
/// `matches()[selected].index` on Enter. [`entry_at`](Self::entry_at) maps a click to a position.
#[derive(Clone, Debug)]
pub struct CommandPalette<'a> {
    popup: Popup<Body<'a>>,
}

#[derive(Clone, Debug)]
struct Body<'a> {
    query: Cow<'a, str>,
    caret: usize,
    placeholder: Cow<'a, str>,
    entries: &'a [PaletteEntry<'a>],
    selected: Option<usize>,
    max_rows: u16,
    width: u16,
}

impl<'a> CommandPalette<'a> {
    /// A palette over `entries` showing `query` with its caret at byte `caret`.
    #[must_use]
    pub fn new(
        query: impl Into<Cow<'a, str>>,
        caret: usize,
        entries: &'a [PaletteEntry<'a>],
    ) -> Self {
        let body = Body {
            query: query.into(),
            caret,
            placeholder: Cow::Borrowed("Type a command"),
            entries,
            selected: None,
            max_rows: DEFAULT_ROWS,
            width: DEFAULT_WIDTH,
        };
        Self {
            popup: Popup::new(body).backdrop(0.55),
        }
    }

    /// Dimmed text shown while the query is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.popup.content_mut().placeholder = placeholder.into();
        self
    }

    /// The highlighted position among [`matches`](Self::matches).
    #[must_use]
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.popup.content_mut().selected = selected;
        self
    }

    /// The most entry rows shown at once, eight by default.
    #[must_use]
    pub fn max_rows(mut self, rows: u16) -> Self {
        self.popup.content_mut().max_rows = rows.max(1);
        self
    }

    /// Panel width in columns, including the border; 62 by default.
    #[must_use]
    pub fn width(mut self, columns: u16) -> Self {
        self.popup.content_mut().width = columns.saturating_sub(2);
        self
    }

    /// Title on the top border.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.popup = self.popup.title(title);
        self
    }

    /// Border line style.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.popup = self.popup.border_type(border_type);
        self
    }

    /// How far the screen behind fades, from `0.0` to `1.0`; `0.55` by default.
    #[must_use]
    pub fn backdrop(mut self, alpha: f32) -> Self {
        self.popup = self.popup.backdrop(alpha);
        self
    }

    /// The entries that match the query, in order, with the ranges to highlight in each title.
    #[must_use]
    pub fn matches(&self) -> Vec<SearchMatch> {
        self.popup.content().matches()
    }

    /// The cells the panel covers inside `area`.
    #[must_use]
    pub fn area(&self, area: Rect) -> Rect {
        self.popup.area(area)
    }

    /// The position among [`matches`](Self::matches) of the row drawn at `(column, row)` inside
    /// `area`.
    #[must_use]
    pub fn entry_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        let body = self.popup.content();
        let list = body.list(inside(self.area(area)), body.matches().len());
        if column < list.x
            || column - list.x >= list.width
            || row < list.y
            || row - list.y >= list.height
        {
            return None;
        }
        let at = body.first(list.height, body.matches().len()) + usize::from(row - list.y);
        (at < body.matches().len()).then_some(at)
    }
}

impl Widget for CommandPalette<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.popup.render(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

impl Body<'_> {
    fn matches(&self) -> Vec<SearchMatch> {
        SearchBox::filter(self.entries, &self.query)
    }

    fn rows(&self, found: usize) -> u16 {
        u16::try_from(found.max(1))
            .unwrap_or(u16::MAX)
            .min(self.max_rows)
    }

    fn list(&self, inner: Rect, found: usize) -> Rect {
        let rows = self.rows(found).min(inner.height.saturating_sub(1));
        Rect::new(inner.x, inner.y.saturating_add(1), inner.width, rows)
    }

    fn first(&self, rows: u16, found: usize) -> usize {
        let selected = self.selected.filter(|at| *at < found).unwrap_or(0);
        (selected + 1).saturating_sub(usize::from(rows))
    }

    fn render_query(&self, row: Rect, found: usize, cx: &mut RenderCx<'_>) {
        let surface = cx.token(Token::Surface1);
        let line = Style::new().fg(cx.token(Token::TextPrimary)).bg(surface);
        cx.fill(row, line, ' ');
        let accent = line
            .fg(cx.token(Token::Accent))
            .with_attrs(Attrs::new().set_bold(true));
        cx.put_str(row.x, row.y, clip_display_prefix(PROMPT, row.width), accent);
        let count = if found == 1 {
            "1 command".to_owned()
        } else {
            format!("{found} commands")
        };
        let count_width = u16::try_from(width(&count) + 1).unwrap_or(u16::MAX);
        let right = if row.width > PROMPT_WIDTH + count_width + 4 {
            count_width
        } else {
            0
        };
        let text = Rect::new(
            row.x.saturating_add(PROMPT_WIDTH.min(row.width)),
            row.y,
            row.width.saturating_sub(PROMPT_WIDTH + right),
            1,
        );
        if right > 0 {
            let dim = line.with_attrs(Attrs::new().set_dim(true));
            cx.put_str(row.x + row.width - right, row.y, &count, dim);
        }
        if self.query.is_empty() {
            let dim = line.with_attrs(Attrs::new().set_dim(true));
            cx.put_str(
                text.x,
                text.y,
                clip_display_prefix(&self.placeholder, text.width),
                dim,
            );
        } else {
            let start = scroll_start(&self.query, self.caret_index(), text.width);
            cx.put_str(
                text.x,
                text.y,
                clip_display_prefix(&self.query[start..], text.width),
                line,
            );
        }
        let caret = self.caret_index();
        let start = scroll_start(&self.query, caret, text.width);
        let offset = u16::try_from(width(&self.query[start..caret])).unwrap_or(u16::MAX);
        let at = text.x.saturating_add(offset);
        if at < text.x.saturating_add(text.width) {
            let shown = if self.query.is_empty() {
                self.placeholder.as_ref()
            } else {
                &self.query[caret..]
            };
            let under = shown.chars().next().unwrap_or(' ');
            let inverted = Style::new().fg(surface).bg(cx.token(Token::FocusRing));
            cx.put_str(at, text.y, under.encode_utf8(&mut [0; 4]), inverted);
        }
    }

    fn caret_index(&self) -> usize {
        let mut index = self.caret.min(self.query.len());
        while !self.query.is_char_boundary(index) {
            index -= 1;
        }
        index
    }

    fn render_list(&self, list: Rect, found: &[SearchMatch], cx: &mut RenderCx<'_>) {
        let panel = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1));
        if found.is_empty() {
            cx.fill(list, panel, ' ');
            let dim = panel.with_attrs(Attrs::new().set_dim(true));
            let shown = clip_display_prefix("No matching commands", list.width.saturating_sub(2));
            cx.put_str(list.x.saturating_add(1), list.y, shown, dim);
            return;
        }
        let first = self.first(list.height, found.len());
        let accent = cx.token(Token::Accent);
        let surface = cx.token(Token::Surface1);
        for row in 0..list.height {
            let y = list.y.saturating_add(row);
            let Some(hit) = found.get(first + usize::from(row)) else {
                cx.fill(Rect::new(list.x, y, list.width, 1), panel, ' ');
                continue;
            };
            let entry = &self.entries[hit.index];
            let current = self.selected == Some(first + usize::from(row));
            let (base, mark, muted) = if current {
                let on = Style::new()
                    .fg(surface)
                    .bg(accent)
                    .with_attrs(Attrs::new().set_bold(true));
                (on, Style::new().fg(accent).bg(surface), on)
            } else {
                (
                    panel,
                    panel.fg(accent).with_attrs(Attrs::new().set_bold(true)),
                    panel.with_attrs(Attrs::new().set_dim(true)),
                )
            };
            cx.fill(Rect::new(list.x, y, list.width, 1), base, ' ');
            let hint = u16::try_from(width(entry.shortcut)).unwrap_or(u16::MAX);
            let room = list.width.saturating_sub(2);
            let hint_cells = if hint > 0 && room > hint + 4 { hint } else { 0 };
            let main = room.saturating_sub(if hint_cells > 0 { hint_cells + 1 } else { 0 });
            let title_width = u16::try_from(width(entry.title))
                .unwrap_or(u16::MAX)
                .min(main);
            SearchBox::put_highlighted(
                cx,
                (list.x.saturating_add(1), y),
                entry.title,
                &hit.ranges,
                (base, mark),
                main,
            );
            let after = main.saturating_sub(title_width);
            if !entry.detail.is_empty() && after > 2 {
                cx.put_str(
                    list.x.saturating_add(1 + title_width + 2),
                    y,
                    clip_display_prefix(entry.detail, after - 2),
                    muted,
                );
            }
            if hint_cells > 0 {
                cx.put_str(
                    list.x
                        .saturating_add(list.width)
                        .saturating_sub(1 + hint_cells),
                    y,
                    entry.shortcut,
                    muted,
                );
            }
        }
    }
}

impl Widget for Body<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let found = self.matches();
        let query = Rect::new(area.x, area.y, area.width, 1);
        self.render_query(query, found.len(), cx);
        let list = self.list(area, found.len());
        if list.height > 0 {
            self.render_list(list, &found, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let rows = self.rows(self.matches().len());
        Size::new(
            self.width.min(avail.width),
            rows.saturating_add(1).min(avail.height),
        )
    }
}

fn inside(panel: Rect) -> Rect {
    Rect::new(
        panel.x.saturating_add(1),
        panel.y.saturating_add(1),
        panel.width.saturating_sub(2),
        panel.height.saturating_sub(2),
    )
}

fn scroll_start(text: &str, caret: usize, columns: u16) -> usize {
    let room = usize::from(columns.max(1)) - 1;
    let mut start = 0;
    let mut chars = text[..caret].char_indices();
    while width(&text[start..caret]) > room {
        match chars.next() {
            Some((_, ch)) => start += ch.len_utf8(),
            None => break,
        }
    }
    start
}
