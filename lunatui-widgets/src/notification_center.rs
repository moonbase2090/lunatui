// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, ToastKind, Widget, WidgetState};

const TITLE: &str = "Notifications";
const ALL_READ: &str = "All read";
const UNREAD: &str = " unread";
const EMPTY: &str = "No notifications";
const DISMISS: &str = "×";
const READ_MARK: &str = "○";
const UNREAD_MARK: &str = "●";
const MIN_WITH_DISMISS: u16 = 6;
const DIGITS: usize = 20;

/// Identifies one entry of a [`NotificationLog`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NotificationId(
    /// Counter value, unique within a [`NotificationLog`].
    pub u32,
);

/// One entry of a [`NotificationLog`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    id: NotificationId,
    kind: ToastKind,
    text: String,
    stamp: String,
    read: bool,
}

impl Notification {
    /// The entry's id.
    #[must_use]
    pub fn id(&self) -> NotificationId {
        self.id
    }

    /// What the entry is about.
    #[must_use]
    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    /// The message.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The time label the app gave it. Empty for none.
    #[must_use]
    pub fn stamp(&self) -> &str {
        &self.stamp
    }

    /// Whether it has been marked read.
    #[must_use]
    pub fn read(&self) -> bool {
        self.read
    }
}

/// The history behind a [`NotificationCenter`]: once it holds `capacity` entries, each new one
/// pushes out the oldest.
///
/// Entries start unread. The app pushes one next to each [`Toast`](crate::Toast) it shows, so the
/// message is still there after the toast goes away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationLog {
    items: VecDeque<Notification>,
    capacity: usize,
    next: u32,
}

impl NotificationLog {
    /// An empty log that keeps at most `capacity` entries (at least one).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
            next: 0,
        }
    }

    /// Adds an unread entry as the newest, dropping the oldest when the log is full. `stamp` is a
    /// time label the app formats, such as `12:04`; empty for none.
    pub fn push(
        &mut self,
        kind: ToastKind,
        text: impl Into<String>,
        stamp: impl Into<String>,
    ) -> NotificationId {
        let id = NotificationId(self.next);
        self.next = self.next.wrapping_add(1);
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(Notification {
            id,
            kind,
            text: text.into(),
            stamp: stamp.into(),
            read: false,
        });
        id
    }

    /// Removes one entry. Returns whether it was there.
    pub fn dismiss(&mut self, id: NotificationId) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        self.items.len() != before
    }

    /// Removes every entry.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Marks one entry read. Returns whether it was there.
    pub fn mark_read(&mut self, id: NotificationId) -> bool {
        self.items
            .iter_mut()
            .find(|item| item.id == id)
            .map(|item| item.read = true)
            .is_some()
    }

    /// Marks every entry read.
    pub fn mark_all_read(&mut self) {
        for item in &mut self.items {
            item.read = true;
        }
    }

    /// How many entries are unread.
    #[must_use]
    pub fn unread(&self) -> usize {
        self.items.iter().filter(|item| !item.read).count()
    }

    /// The number of entries held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the log holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The entry with this id.
    #[must_use]
    pub fn get(&self, id: NotificationId) -> Option<&Notification> {
        self.items.iter().find(|item| item.id == id)
    }

    /// The entries from newest to oldest, the order a [`NotificationCenter`] lists them.
    pub fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.items.iter().rev()
    }
}

/// A list of past notifications, newest first, under a header that counts the unread ones.
///
/// Each row shows a mark (filled while unread), the time label, the message colored by its
/// [`ToastKind`] mark, and a `×` to dismiss it. Unread messages are bold, read ones dim. The app
/// owns the [`NotificationLog`] and which row is [`selected`](NotificationCenter::selected), as a
/// position counted from the newest; the list scrolls to keep the selected row in view.
/// [`row_at`](NotificationCenter::row_at) and [`dismiss_at`](NotificationCenter::dismiss_at) are
/// for hit testing, and map to [`NotificationLog::iter`] positions.
#[derive(Clone, Debug)]
pub struct NotificationCenter<'a> {
    log: &'a NotificationLog,
    selected: Option<usize>,
    state: WidgetState,
}

impl<'a> NotificationCenter<'a> {
    /// A center listing `log`.
    #[must_use]
    pub const fn new(log: &'a NotificationLog) -> Self {
        Self {
            log,
            selected: None,
            state: WidgetState::Normal,
        }
    }

    /// The highlighted row, counted from the newest.
    #[must_use]
    pub const fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Interaction state: disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The cells of the entry rows in `area`, below the header.
    #[must_use]
    pub fn list_area(&self, area: Rect) -> Rect {
        Rect::new(
            area.x,
            area.y + area.height.min(1),
            area.width,
            area.height.saturating_sub(1),
        )
    }

    fn first_row(&self, rows: u16) -> usize {
        let selected = self
            .selected
            .filter(|selected| *selected < self.log.len())
            .unwrap_or(0);
        (selected + 1).saturating_sub(usize::from(rows))
    }

    /// The position of the row at (`x`, `y`), counted from the newest, if any.
    #[must_use]
    pub fn row_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let list = self.list_area(area);
        let inside =
            x >= list.x && x - list.x < list.width && y >= list.y && y - list.y < list.height;
        let index = self.first_row(list.height) + usize::from(y.saturating_sub(list.y));
        (inside && index < self.log.len()).then_some(index)
    }

    /// The position of the row whose `×` is at (`x`, `y`), if any.
    #[must_use]
    pub fn dismiss_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let at = Self::dismiss_x(area)?;
        self.row_at(area, x, y).filter(|_| x == at)
    }

    fn dismiss_x(area: Rect) -> Option<u16> {
        (area.width >= MIN_WITH_DISMISS).then(|| area.x + area.width - 2)
    }

    fn render_header(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let bar = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface1))
            .with_attrs(Attrs::new().set_bold(true).set_dim(disabled));
        cx.fill(Rect::new(area.x, area.y, area.width, 1), bar, ' ');
        let room = area.width.saturating_sub(2);
        let title = clip_display_prefix(TITLE, room);
        cx.put_str(area.x + 1, area.y, title, bar);
        let unread = self.log.unread();
        let mut buffer = [0; DIGITS];
        let count = digits(unread, &mut buffer);
        let right = if unread == 0 {
            width(ALL_READ)
        } else {
            width(count) + width(UNREAD)
        };
        let used = width(title) + 1 + right;
        if usize::from(room) < used {
            return;
        }
        let dim = bar.with_attrs(Attrs::new().set_dim(true));
        let start = area.x + area.width - 1 - u16::try_from(right).unwrap_or(0);
        if unread == 0 {
            cx.put_str(start, area.y, ALL_READ, dim);
        } else {
            let accent = bar.fg(cx.token(Token::Accent));
            cx.put_str(start, area.y, count, accent);
            let after = start + u16::try_from(width(count)).unwrap_or(0);
            cx.put_str(after, area.y, UNREAD, dim);
        }
    }

    fn render_list(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let list = self.list_area(area);
        let disabled = matches!(self.state, WidgetState::Disabled);
        let surface = cx.token(Token::Surface0);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        for row in 0..list.height {
            cx.fill(Rect::new(list.x, list.y + row, list.width, 1), base, ' ');
        }
        if self.log.is_empty() {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            let message = clip_display_prefix(EMPTY, list.width.saturating_sub(2));
            cx.put_str(list.x + 1, list.y, message, dim);
            return;
        }
        let first = self.first_row(list.height);
        let rows = usize::from(list.height);
        for (offset, item) in self.log.iter().skip(first).take(rows).enumerate() {
            let y = list.y + u16::try_from(offset).unwrap_or(0);
            let current = self.selected == Some(first + offset);
            self.render_row(item, Rect::new(list.x, y, list.width, 1), current, base, cx);
        }
    }

    fn render_row(
        &self,
        item: &Notification,
        row: Rect,
        current: bool,
        base: Style,
        cx: &mut RenderCx<'_>,
    ) {
        let surface = cx.token(Token::Surface0);
        let accent = cx.token(Token::Accent);
        let (text, mark) = if current {
            let selected = Style::new().fg(surface).bg(accent);
            cx.fill(row, selected, ' ');
            (selected, selected)
        } else {
            let kind = base.fg(cx.token(item.kind.token()));
            (base, kind)
        };
        let weight = if item.read {
            Attrs::new().set_dim(true)
        } else {
            Attrs::new().set_bold(true)
        };
        let weight = if matches!(self.state, WidgetState::Disabled) {
            weight.set_dim(true)
        } else {
            weight
        };
        let glyph = if item.read { READ_MARK } else { UNREAD_MARK };
        cx.put_str(row.x + 1, row.y, glyph, mark);
        let right = Self::dismiss_x(row).unwrap_or(row.x + row.width);
        let mut at = row.x + 3;
        if !item.stamp.is_empty() {
            let stamp = clip_display_prefix(&item.stamp, right.saturating_sub(at));
            cx.put_str(
                at,
                row.y,
                stamp,
                text.with_attrs(Attrs::new().set_dim(true)),
            );
            at += u16::try_from(width(stamp)).unwrap_or(0) + 1;
        }
        let room = right.saturating_sub(at).saturating_sub(1);
        cx.put_str(
            at,
            row.y,
            clip_display_prefix(&item.text, room),
            text.with_attrs(weight),
        );
        if let Some(x) = Self::dismiss_x(row) {
            cx.put_str(
                x,
                row.y,
                DISMISS,
                text.with_attrs(Attrs::new().set_dim(true)),
            );
        }
    }
}

fn digits(mut value: usize, buffer: &mut [u8; DIGITS]) -> &str {
    let mut start = DIGITS;
    loop {
        start -= 1;
        buffer[start] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
        if value == 0 {
            break;
        }
    }
    std::str::from_utf8(&buffer[start..]).unwrap_or("")
}

impl Widget for NotificationCenter<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.render_header(area, cx);
        if area.height > 1 {
            self.render_list(area, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = u16::try_from(self.log.len().max(1)).unwrap_or(u16::MAX);
        Size::new(avail.width, rows.saturating_add(1).min(avail.height))
    }
}
