// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, UnderlineStyle, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};

/// The schedule state of a span or milestone, drawn with a semantic theme token.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineStatus {
    /// Scheduled work. Its progress uses the accent token; without progress it has not
    /// started and draws as track only, so future work recedes behind finished and active work.
    #[default]
    Planned,
    /// Finished work, drawn with the ok token.
    Done,
    /// Work that may slip, drawn with the warning token.
    AtRisk,
    /// Work that cannot proceed, drawn with the failure token.
    Blocked,
}

impl TimelineStatus {
    const fn token(self) -> Token {
        match self {
            Self::Planned => Token::Accent,
            Self::Done => Token::Ok,
            Self::AtRisk => Token::Warn,
            Self::Blocked => Token::Fail,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Span { start: i64, end: i64 },
    Milestone { at: i64 },
}

/// One span or milestone on a lane. Times are in the application's own unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineItem<'a> {
    kind: Kind,
    label: &'a str,
    progress: Option<u8>,
    status: TimelineStatus,
}

impl<'a> TimelineItem<'a> {
    /// A span covering `start..end`. Reversed bounds are sorted.
    #[must_use]
    pub const fn span(start: i64, end: i64) -> Self {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Self {
            kind: Kind::Span { start, end },
            label: "",
            progress: None,
            status: TimelineStatus::Planned,
        }
    }

    /// A milestone at one instant, drawn as `◆` with its label to the right.
    #[must_use]
    pub const fn milestone(at: i64) -> Self {
        Self {
            kind: Kind::Milestone { at },
            label: "",
            progress: None,
            status: TimelineStatus::Planned,
        }
    }

    /// Borrows a label. Span labels clip to the span; milestone labels clip to the view.
    #[must_use]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    /// Fills the first `percent` of a span with its status color and the rest with the
    /// track color. Values above 100 become 100. Without it, a planned span is all track and
    /// any other span is solid. Milestones ignore progress.
    #[must_use]
    pub const fn progress(mut self, percent: u8) -> Self {
        self.progress = Some(if percent > 100 { 100 } else { percent });
        self
    }

    /// Sets the status that picks the item's theme token.
    #[must_use]
    pub const fn status(mut self, status: TimelineStatus) -> Self {
        self.status = status;
        self
    }

    const fn extent(&self) -> (i64, i64) {
        match self.kind {
            Kind::Span { start, end } => (start, end),
            Kind::Milestone { at } => (at, at),
        }
    }
}

/// One labeled row of borrowed items.
#[derive(Clone, Copy, Debug)]
pub struct TimelineLane<'a> {
    label: &'a str,
    items: &'a [TimelineItem<'a>],
}

impl<'a> TimelineLane<'a> {
    /// Borrows a label and its items. An empty label on every lane needs no label gutter.
    #[must_use]
    pub const fn new(label: &'a str, items: &'a [TimelineItem<'a>]) -> Self {
        Self { label, items }
    }
}

/// An axis label placed at a time. Ticks are expected in ascending time order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineTick<'a> {
    /// The time the tick marks.
    pub at: i64,
    /// The label drawn from the tick's column, clipped before the next tick.
    pub label: &'a str,
}

impl<'a> TimelineTick<'a> {
    /// Borrows a label for a time.
    #[must_use]
    pub const fn new(at: i64, label: &'a str) -> Self {
        Self { at, label }
    }
}

/// A logical item position: a lane index and an item index within that lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimelineItemId {
    /// Zero-based lane index.
    pub lane: usize,
    /// Zero-based item index within the lane.
    pub item: usize,
}

/// The visible time window `start..end`, owned by the application.
///
/// The window is never empty: a zero-length window becomes one unit long. Panning and zooming
/// return a new view and saturate at the `i64` range.
///
/// ```
/// use lunatui_widgets::TimelineView;
/// let view = TimelineView::new(0, 100);
/// assert_eq!(view.pan(30), TimelineView::new(30, 130));
/// assert_eq!(view.zoom_in(), TimelineView::new(25, 75));
/// assert_eq!(view.reveal(150, 160), TimelineView::new(60, 160));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimelineView {
    start: i64,
    end: i64,
}

impl TimelineView {
    /// A window over `start..end`. Reversed bounds are sorted.
    #[must_use]
    pub const fn new(start: i64, end: i64) -> Self {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if start == end {
            if end == i64::MAX {
                Self {
                    start: end - 1,
                    end,
                }
            } else {
                Self {
                    start,
                    end: end + 1,
                }
            }
        } else {
            Self { start, end }
        }
    }

    /// The first visible time.
    #[must_use]
    pub const fn start(self) -> i64 {
        self.start
    }

    /// The end of the window, exclusive.
    #[must_use]
    pub const fn end(self) -> i64 {
        self.end
    }

    /// The window length in time units, at least one.
    #[must_use]
    pub const fn len(self) -> u64 {
        self.end.abs_diff(self.start)
    }

    /// Always `false`: a view is at least one unit long.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        false
    }

    /// Moves the window by `delta` units, keeping its length.
    #[must_use]
    pub fn pan(self, delta: i64) -> Self {
        Self::place(i128::from(self.start) + i128::from(delta), self.len_i128())
    }

    /// Halves the window around its center, down to one unit.
    #[must_use]
    pub fn zoom_in(self) -> Self {
        let len = self.len_i128();
        let next = (len / 2).max(1);
        Self::place(self.center() - next / 2, next)
    }

    /// Doubles the window around its center.
    #[must_use]
    pub fn zoom_out(self) -> Self {
        let next = self.len_i128() * 2;
        Self::place(self.center() - next / 2, next)
    }

    /// Pans the least distance that brings `start..end` into view. A range longer than the
    /// window aligns its start with the window's start. An empty range is treated as a
    /// one-unit milestone range. The view stays within the signed time bounds.
    #[must_use]
    pub fn reveal(self, start: i64, end: i64) -> Self {
        let (start, end) = (start.min(end), start.max(end));
        let len = self.len_i128();
        let (start, end) = (i128::from(start), i128::from(end));
        let end = end.max(start + 1);
        if end - start >= len || start < i128::from(self.start) {
            Self::place(start, len)
        } else if end > i128::from(self.end) {
            Self::place(end - len, len)
        } else {
            self
        }
    }

    fn len_i128(self) -> i128 {
        i128::from(self.end) - i128::from(self.start)
    }

    fn center(self) -> i128 {
        i128::from(self.start) + self.len_i128() / 2
    }

    fn place(start: i128, len: i128) -> Self {
        let max = i128::from(i64::MAX);
        let min = i128::from(i64::MIN);
        let len = len.clamp(1, max - min);
        let start = start.clamp(min, max - len);
        let end = start + len;
        Self::new(
            i64::try_from(start).unwrap_or(i64::MIN),
            i64::try_from(end).unwrap_or(i64::MAX),
        )
    }
}

/// A scrollable time axis of labeled lanes holding spans and milestones, as in a Gantt chart.
///
/// The application owns the data, the [`TimelineView`] window, the first visible lane, and the
/// selection; the widget draws them and answers geometry queries with the same layout it
/// renders. Times are plain `i64` values in whatever unit the application uses.
///
/// Spans show their label inside. A span with progress keeps the status color for the finished
/// share and uses the track color for the rest. Without progress, a planned span has not
/// started and is all track; done, at-risk, and blocked spans are solid. Milestones draw `◆` with the label to the right. Items draw in slice order, so a later
/// item covers an earlier one where they overlap. Spans clip at the view's edges.
///
/// With ticks, a two-row header shows each label at its time and an axis rule with `┬` marks.
/// [`now`](Self::now) draws a neutral `▼` on the rule and `┊` through empty lane cells.
/// Selection adds bold and underline to the item; when focused, the selected lane's label uses
/// the focus color. Disabled timelines draw dimmed `░` swatches and no selection.
///
/// ```
/// use lunatui_core::Rect;
/// use lunatui_widgets::{Timeline, TimelineItem, TimelineItemId, TimelineLane, TimelineView};
/// let items = [TimelineItem::span(0, 50).label("build"), TimelineItem::milestone(75)];
/// let lanes = [TimelineLane::new("", &items)];
/// let timeline = Timeline::new(&lanes).view(TimelineView::new(0, 100));
/// let area = Rect::new(0, 0, 20, 1);
/// assert_eq!(timeline.x_at(area, 50), Some(10));
/// assert_eq!(timeline.item_at(area, 3, 0), Some(TimelineItemId { lane: 0, item: 0 }));
/// assert_eq!(timeline.item_at(area, 15, 0), Some(TimelineItemId { lane: 0, item: 1 }));
/// ```
#[derive(Clone, Debug)]
pub struct Timeline<'a> {
    lanes: &'a [TimelineLane<'a>],
    ticks: &'a [TimelineTick<'a>],
    view: TimelineView,
    lane_offset: usize,
    now: Option<i64>,
    selected: Option<TimelineItemId>,
    state: WidgetState,
}

impl<'a> Timeline<'a> {
    /// Borrows lanes, viewing `0..100` with no header.
    #[must_use]
    pub const fn new(lanes: &'a [TimelineLane<'a>]) -> Self {
        Self {
            lanes,
            ticks: &[],
            view: TimelineView::new(0, 100),
            lane_offset: 0,
            now: None,
            selected: None,
            state: WidgetState::Normal,
        }
    }

    /// Reserves the two-row axis header and labels it with `ticks`.
    #[must_use]
    pub const fn ticks(mut self, ticks: &'a [TimelineTick<'a>]) -> Self {
        self.ticks = ticks;
        self
    }

    /// Sets the visible time window.
    #[must_use]
    pub const fn view(mut self, view: TimelineView) -> Self {
        self.view = view;
        self
    }

    /// Starts drawing at lane `offset`, for vertical scrolling. Offsets past the end show no lanes.
    #[must_use]
    pub const fn lane_offset(mut self, offset: usize) -> Self {
        self.lane_offset = offset;
        self
    }

    /// Marks the current time. Times outside the view draw nothing.
    #[must_use]
    pub const fn now(mut self, at: Option<i64>) -> Self {
        self.now = at;
        self
    }

    /// Marks an app-owned item. Positions outside the lanes are ignored.
    #[must_use]
    pub const fn selected(mut self, item: Option<TimelineItemId>) -> Self {
        self.selected = item;
        self
    }

    /// Sets normal, focused, or disabled presentation without changing selection.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Returns the earliest start and latest end over every item, or `None` with no items.
    /// Use it to bound panning.
    #[must_use]
    pub fn bounds(&self) -> Option<(i64, i64)> {
        self.lanes
            .iter()
            .flat_map(|lane| lane.items.iter())
            .map(TimelineItem::extent)
            .reduce(|(low, high), (start, end)| (low.min(start), high.max(end)))
    }

    /// Returns the terminal column that draws time `at`, or `None` outside the view or area.
    #[must_use]
    pub fn x_at(&self, area: Rect, at: i64) -> Option<u16> {
        let layout = self.layout(area);
        let column = layout.floor_column(at);
        (0..i128::from(layout.data.width).min(layout.width_limit()))
            .contains(&column)
            .then(|| layout.data.x + u16::try_from(column).unwrap_or(0))
    }

    /// Returns the time at the left edge of column `x`, rounded down, or `None` outside the
    /// lane area. When the view is zoomed in past one unit per column, neighbouring columns
    /// share a time.
    #[must_use]
    pub fn time_at(&self, area: Rect, x: u16) -> Option<i64> {
        let layout = self.layout(area);
        let column = x.checked_sub(layout.data.x)?;
        if column >= layout.data.width {
            return None;
        }
        let offset = i128::from(column) * layout.len / i128::from(layout.data.width);
        i64::try_from(i128::from(self.view.start) + offset).ok()
    }

    /// Returns the topmost item drawn at a terminal cell. A milestone answers on its `◆` cell
    /// only. Gutters, headers, and empty cells return `None`.
    #[must_use]
    pub fn item_at(&self, area: Rect, x: u16, y: u16) -> Option<TimelineItemId> {
        let layout = self.layout(area);
        let column = x.checked_sub(layout.data.x)?;
        let row = y.checked_sub(layout.data.y)?;
        if column >= layout.data.width || row >= layout.data.height {
            return None;
        }
        let lane = self.lane_offset.checked_add(usize::from(row))?;
        let items = self.lanes.get(lane)?.items;
        items
            .iter()
            .enumerate()
            .rev()
            .find(|(_, item)| {
                layout
                    .columns(item)
                    .is_some_and(|(first, last)| (first..last).contains(&column))
            })
            .map(|(item, _)| TimelineItemId { lane, item })
    }

    fn label_width(&self) -> u16 {
        let widest = self
            .lanes
            .iter()
            .map(|lane| width(lane.label))
            .max()
            .unwrap_or(0);
        u16::try_from(widest.saturating_add(usize::from(widest > 0))).unwrap_or(u16::MAX)
    }

    fn layout(&self, area: Rect) -> Layout {
        let area = clip_to_coordinates(area);
        let gutter = self.label_width().min(area.width);
        let header = if self.ticks.is_empty() { 0 } else { 2 };
        let header = header.min(area.height);
        Layout {
            area,
            data: Rect::new(
                area.x + gutter,
                area.y + header,
                area.width - gutter,
                area.height - header,
            ),
            header,
            start: i128::from(self.view.start),
            len: i128::from(self.view.end) - i128::from(self.view.start),
        }
    }

    fn base_style(&self, cx: &RenderCx<'_>, fg: Token) -> Style {
        let mut style = Style::new().fg(cx.token(fg)).bg(cx.token(Token::Surface0));
        if self.state == WidgetState::Disabled {
            style.attrs = style.attrs.set_dim(true);
        }
        style
    }

    fn is_selected(&self, id: TimelineItemId) -> bool {
        self.state != WidgetState::Disabled && self.selected == Some(id)
    }

    fn render_header(&self, layout: &Layout, cx: &mut RenderCx<'_>) {
        if layout.header == 0 || layout.data.width == 0 {
            return;
        }
        let text = self.base_style(cx, Token::TextPrimary);
        let rule_y = layout.area.y + layout.header - 1;
        if layout.header > 1 {
            cx.fill(
                Rect::new(layout.data.x, rule_y, layout.data.width, 1),
                text,
                '─',
            );
        }
        let visible: Vec<(u16, &str)> = self
            .ticks
            .iter()
            .filter_map(|tick| {
                let column = layout.floor_column(tick.at);
                (0..i128::from(layout.data.width))
                    .contains(&column)
                    .then(|| (u16::try_from(column).unwrap_or(0), tick.label))
            })
            .collect();
        for (index, &(column, label)) in visible.iter().enumerate() {
            let x = layout.data.x + column;
            if layout.header > 1 {
                cx.put_str(x, rule_y, "┬", text);
            }
            let limit = visible
                .get(index + 1)
                .map_or(layout.data.width, |&(next, _)| next.saturating_sub(1))
                .saturating_sub(column);
            cx.put_str(x, layout.area.y, clip_display_prefix(label, limit), text);
        }
        if layout.header > 1
            && let Some(column) = self.now_column(layout)
        {
            let mut marker = text;
            marker.attrs = marker.attrs.set_bold(true);
            cx.put_str(layout.data.x + column, rule_y, "▼", marker);
        }
    }

    fn now_column(&self, layout: &Layout) -> Option<u16> {
        let column = layout.floor_column(self.now?);
        (0..i128::from(layout.data.width))
            .contains(&column)
            .then(|| u16::try_from(column).unwrap_or(0))
    }

    fn render_lane(&self, lane: usize, y: u16, layout: &Layout, cx: &mut RenderCx<'_>) {
        let Some(row) = self.lanes.get(lane) else {
            return;
        };
        let mut label_style = self.base_style(cx, Token::TextPrimary);
        if self.state == WidgetState::Focused && self.selected.is_some_and(|id| id.lane == lane) {
            label_style = label_style.fg(cx.token(Token::FocusRing));
            label_style.attrs = label_style.attrs.set_bold(true);
        }
        let gutter = layout.data.x - layout.area.x;
        cx.put_str(
            layout.area.x,
            y,
            clip_display_prefix(row.label, gutter.saturating_sub(1)),
            label_style,
        );
        if let Some(column) = self.now_column(layout) {
            let now = self.base_style(cx, Token::TextPrimary);
            cx.put_str(layout.data.x + column, y, "┊", now);
        }
        for (index, item) in row.items.iter().enumerate() {
            let selected = self.is_selected(TimelineItemId { lane, item: index });
            match item.kind {
                Kind::Span { .. } => self.render_span(item, selected, y, layout, cx),
                Kind::Milestone { .. } => self.render_milestone(item, selected, y, layout, cx),
            }
        }
    }

    fn render_span(
        &self,
        item: &TimelineItem<'_>,
        selected: bool,
        y: u16,
        layout: &Layout,
        cx: &mut RenderCx<'_>,
    ) {
        let Some((first, last)) = layout.columns(item) else {
            return;
        };
        let split = layout
            .progress_column(item)
            .unwrap_or(last)
            .clamp(first, last);
        let color = cx.token(item.status.token());
        let disabled = self.state == WidgetState::Disabled;
        let (mut done, mut track) = if disabled {
            (
                self.base_style(cx, item.status.token()),
                self.base_style(cx, Token::Surface1),
            )
        } else {
            (
                Style::new().fg(cx.token(Token::Surface0)).bg(color),
                Style::new()
                    .fg(cx.token(Token::TextPrimary))
                    .bg(cx.token(Token::Surface1)),
            )
        };
        if selected {
            for style in [&mut done, &mut track] {
                style.attrs = style
                    .attrs
                    .set_bold(true)
                    .set_underline(UnderlineStyle::Single);
            }
        }
        let fill = if disabled { '░' } else { ' ' };
        let x = layout.data.x + first;
        cx.fill(Rect::new(x, y, split - first, 1), done, fill);
        cx.fill(
            Rect::new(layout.data.x + split, y, last - split, 1),
            track,
            fill,
        );
        let label_style = if disabled {
            self.base_style(cx, Token::TextPrimary)
        } else {
            done
        };
        let label = clip_display_prefix(item.label, last - first);
        cx.put_str(x, y, label, label_style);
        if !disabled {
            let written = u16::try_from(width(label)).unwrap_or(0);
            let tail = first.saturating_add(written).min(last);
            if split < tail {
                cx.set_style(Rect::new(layout.data.x + split, y, tail - split, 1), track);
            }
        }
    }

    fn render_milestone(
        &self,
        item: &TimelineItem<'_>,
        selected: bool,
        y: u16,
        layout: &Layout,
        cx: &mut RenderCx<'_>,
    ) {
        let Some((column, _)) = layout.columns(item) else {
            return;
        };
        let mut glyph = self.base_style(cx, item.status.token());
        let mut text = self.base_style(cx, Token::TextPrimary);
        if selected {
            for style in [&mut glyph, &mut text] {
                style.attrs = style
                    .attrs
                    .set_bold(true)
                    .set_underline(UnderlineStyle::Single);
            }
        }
        let x = layout.data.x + column;
        cx.put_str(x, y, "◆", glyph);
        let room = layout.data.width.saturating_sub(column + 2);
        if !item.label.is_empty() && room > 0 {
            cx.put_str(x + 2, y, clip_display_prefix(item.label, room), text);
        }
    }
}

impl Widget for Timeline<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let layout = self.layout(area);
        let background = self.base_style(cx, Token::TextPrimary);
        cx.fill(layout.area, background, ' ');
        self.render_header(&layout, cx);
        for row in 0..layout.data.height {
            let Some(lane) = self.lane_offset.checked_add(usize::from(row)) else {
                break;
            };
            if lane >= self.lanes.len() {
                break;
            }
            self.render_lane(lane, layout.data.y + row, &layout, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if self.lanes.is_empty() {
            return Size::new(0, 0);
        }
        let lanes = self.lanes.len().saturating_sub(self.lane_offset);
        let header = if self.ticks.is_empty() { 0 } else { 2 };
        let height = lanes.saturating_add(header);
        Size::new(avail.width, u16::try_from(height).unwrap_or(u16::MAX)).clamp(avail)
    }
}

struct Layout {
    area: Rect,
    data: Rect,
    header: u16,
    start: i128,
    len: i128,
}

impl Layout {
    fn width_limit(&self) -> i128 {
        i128::from(self.data.width)
    }

    fn scaled(&self, at: i64) -> i128 {
        (i128::from(at) - self.start) * i128::from(self.data.width)
    }

    fn floor_column(&self, at: i64) -> i128 {
        self.scaled(at).div_euclid(self.len)
    }

    fn ceil_column(&self, at: i64) -> i128 {
        -(-self.scaled(at)).div_euclid(self.len)
    }

    fn columns(&self, item: &TimelineItem<'_>) -> Option<(u16, u16)> {
        let width = self.width_limit();
        let (first, last) = match item.kind {
            Kind::Span { start, end } => {
                let first = self.floor_column(start);
                (first, self.ceil_column(end).max(first + 1))
            }
            Kind::Milestone { at } => {
                let column = self.floor_column(at);
                (column, column + 1)
            }
        };
        if last <= 0 || first >= width {
            return None;
        }
        Some((
            u16::try_from(first.max(0)).ok()?,
            u16::try_from(last.min(width)).ok()?,
        ))
    }

    fn progress_column(&self, item: &TimelineItem<'_>) -> Option<u16> {
        let Kind::Span { start, end } = item.kind else {
            return None;
        };
        let percent = match (item.progress, item.status) {
            (Some(percent), _) => i128::from(percent),
            (None, TimelineStatus::Planned) => 0,
            (None, _) => return None,
        };
        let first = self.floor_column(start);
        let last = self.ceil_column(end).max(first + 1);
        let split = first + (last - first) * percent / 100;
        u16::try_from(split.clamp(0, self.width_limit())).ok()
    }
}
