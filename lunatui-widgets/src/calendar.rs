// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt::{self, Write as _};

use lunatui_core::{Attrs, Rect, Size, Style};
use lunatui_theme::Token;

use crate::{Block, RenderCx, Widget, WidgetState};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: &str = " Su Mo Tu We Th Fr Sa";
const WEEKDAYS_MONDAY: &str = " Mo Tu We Th Fr Sa Su";
const GRID_WIDTH: u16 = 21;

/// First weekday shown in a monthly calendar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WeekStart {
    /// Sunday through Saturday, the default.
    #[default]
    Sunday,
    /// Monday through Sunday.
    Monday,
}

/// A Gregorian calendar date.
///
/// Years follow the proleptic Gregorian calendar. Month and day are 1-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    /// 1970-01-01.
    pub const UNIX_EPOCH: Self = Self {
        year: 1970,
        month: 1,
        day: 1,
    };

    /// `Some` when `month` is 1..=12 and `day` falls in that month.
    #[must_use]
    pub const fn from_ymd(year: i32, month: u8, day: u8) -> Option<Self> {
        if month == 0 || month > 12 || day == 0 || day > month_length(year, month) {
            None
        } else {
            Some(Self { year, month, day })
        }
    }

    /// Year, which may be negative.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// Month in `1..=12`.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// Day of the month, starting at 1.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// English month name, matching the `time` crate's month labels.
    #[must_use]
    pub const fn month_name(self) -> &'static str {
        MONTHS[self.month as usize - 1]
    }

    const fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    const fn succ(self) -> Self {
        let len = month_length(self.year, self.month);
        if self.day < len {
            return Self {
                day: self.day + 1,
                ..self
            };
        }
        if self.month == 12 {
            Self {
                year: self.year.saturating_add(1),
                month: 1,
                day: 1,
            }
        } else {
            Self {
                month: self.month + 1,
                day: 1,
                ..self
            }
        }
    }

    const fn pred(self) -> Self {
        if self.day > 1 {
            return Self {
                day: self.day - 1,
                ..self
            };
        }
        if self.month == 1 {
            let year = self.year.saturating_sub(1);
            Self {
                year,
                month: 12,
                day: 31,
            }
        } else {
            let month = self.month - 1;
            Self {
                month,
                day: month_length(self.year, month),
                ..self
            }
        }
    }

    fn weekday(self) -> u8 {
        let days = days_from_civil(self.year, u32::from(self.month), u32::from(self.day));
        u8::try_from(days.wrapping_add(4).rem_euclid(7)).unwrap_or(0)
    }

    fn sunday_on_or_before(self) -> Self {
        let mut date = self;
        let mut left = self.weekday();
        while left > 0 {
            date = date.pred();
            left -= 1;
        }
        date
    }

    fn monday_on_or_before(self) -> Self {
        let mut date = self;
        let mut left = (self.weekday() + 6) % 7;
        while left > 0 {
            date = date.pred();
            left -= 1;
        }
        date
    }
}

/// Style for one date. The last matching sample wins when several are provided.
pub trait DateStyle {
    /// Style layered onto the date's base style. [`Color::Default`](lunatui_core::Color::Default)
    /// leaves that channel alone.
    fn style_for(&self, date: Date) -> Style;

    /// Whether this date has an event, independent of its event style.
    fn has_event(&self, _date: Date) -> bool {
        false
    }
}

impl DateStyle for &[(Date, Style)] {
    fn style_for(&self, date: Date) -> Style {
        let mut style = Style::new();
        for (event, event_style) in *self {
            if *event == date {
                style = *event_style;
            }
        }
        style
    }

    fn has_event(&self, date: Date) -> bool {
        self.iter().any(|(event, _)| *event == date)
    }
}

impl DateStyle for () {
    fn style_for(&self, _date: Date) -> Style {
        Style::new()
    }
}

/// One month, Sunday through Saturday, in the same grid ratatui's `Monthly` uses.
///
/// Calling [`state`](Monthly::state) paints from theme tokens: the month title uses the accent,
/// the selected day uses the accent or the focus ring, surrounding days are dim, and disabled
/// cells are dim. Explicit header and date styles follow ratatui instead, including a blank
/// cell for days outside the month unless [`show_surrounding`](Monthly::show_surrounding) is set.
#[derive(Clone, Debug)]
pub struct Monthly<S> {
    date: Date,
    events: S,
    week_start: WeekStart,
    event_markers: bool,
    surrounding: Option<Style>,
    weekday: Option<Style>,
    month_header: Option<Style>,
    default_style: Style,
    explicit_default: bool,
    block: Option<Block>,
    state: WidgetState,
    themed: bool,
}

impl<S> Monthly<S> {
    /// Calendar for the month containing `date`. `date` itself is the selected day.
    ///
    /// # Examples
    ///
    /// ```
    /// use lunatui_core::{ClusterTable, Grid, Rect};
    /// use lunatui_theme::{Theme, Variant};
    /// use lunatui_widgets::{Date, Monthly, RenderCx, Widget};
    ///
    /// let Some(date) = Date::from_ymd(2024, 2, 1) else {
    ///     return;
    /// };
    /// let theme = Theme::builtin(Variant::Dark);
    /// let mut grid = Grid::new(21, 6);
    /// let mut clusters = ClusterTable::new();
    /// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    /// Monthly::new(date, ()).render(Rect::new(0, 0, 21, 6), &mut cx);
    /// ```
    #[must_use]
    pub fn new(date: Date, events: S) -> Self {
        Self {
            date,
            events,
            week_start: WeekStart::Sunday,
            event_markers: false,
            surrounding: None,
            weekday: None,
            month_header: None,
            default_style: Style::new(),
            explicit_default: false,
            block: None,
            state: WidgetState::Normal,
            themed: false,
        }
    }

    /// First weekday column. The default is Sunday.
    #[must_use]
    pub fn week_start(mut self, week_start: WeekStart) -> Self {
        self.week_start = week_start;
        self
    }

    /// Adds a dot in the date gutter for every event sample.
    #[must_use]
    pub fn event_markers(mut self, enabled: bool) -> Self {
        self.event_markers = enabled;
        self
    }

    /// Style for days outside this month. Without it, those cells are blank.
    #[must_use]
    pub fn show_surrounding(mut self, style: Style) -> Self {
        self.surrounding = Some(style);
        self
    }

    /// Header row ` Su Mo Tu We Th Fr Sa`.
    #[must_use]
    pub fn show_weekdays_header(mut self, style: Style) -> Self {
        self.weekday = Some(style);
        self
    }

    /// Centered month name and year.
    #[must_use]
    pub fn show_month_header(mut self, style: Style) -> Self {
        self.month_header = Some(style);
        self
    }

    /// Style for days in this month, before event styles are layered on.
    #[must_use]
    pub fn default_style(mut self, style: Style) -> Self {
        self.default_style = style;
        self.explicit_default = true;
        self
    }

    /// Optional frame. The grid is drawn in its inner area.
    #[must_use]
    pub fn block(mut self, block: Block) -> Self {
        self.block = Some(block);
        self
    }

    /// Turns on token colors for any header or date that has no explicit style.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self.themed = true;
        self
    }

    /// Columns used by the day grid, plus the block frame.
    #[must_use]
    pub fn width(&self) -> u16 {
        let (extra, _) = self.chrome();
        GRID_WIDTH.saturating_add(extra)
    }

    /// Week rows, plus headers and the block frame.
    #[must_use]
    pub fn height(&self) -> u16 {
        let (_, extra) = self.chrome();
        u16::from(week_rows(self.date, self.week_start))
            .saturating_add(self.header_rows())
            .saturating_add(extra)
    }

    fn header_rows(&self) -> u16 {
        u16::from(self.show_month()) + u16::from(self.show_weekdays())
    }

    fn show_month(&self) -> bool {
        self.month_header.is_some() || self.themed
    }

    fn show_weekdays(&self) -> bool {
        self.weekday.is_some() || self.themed
    }

    fn chrome(&self) -> (u16, u16) {
        let Some(block) = &self.block else {
            return (0, 0);
        };
        let area = Rect::new(0, 0, 400, 200);
        let inner = block.inner(area);
        (
            area.width.saturating_sub(inner.width),
            area.height.saturating_sub(inner.height),
        )
    }
}

impl<S: DateStyle> Widget for Monthly<S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let inner = if let Some(block) = &self.block {
            block.render(area, cx);
            block.inner(area)
        } else {
            area
        };
        self.render_month(inner, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(self.width(), self.height()).clamp(avail)
    }
}

impl<S: DateStyle> Monthly<S> {
    fn render_month(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let mut y = area.y;
        let bottom = area.y.saturating_add(area.height);
        if self.show_month() && y < bottom {
            self.paint_month_header(Rect::new(area.x, y, area.width, 1), cx);
            y = y.saturating_add(1);
        }
        if self.show_weekdays() && y < bottom {
            self.paint_weekdays(Rect::new(area.x, y, area.width, 1), cx);
            y = y.saturating_add(1);
        }
        let first = self.date.first_of_month();
        let mut cursor = match self.week_start {
            WeekStart::Sunday => first.sunday_on_or_before(),
            WeekStart::Monday => first.monday_on_or_before(),
        };
        let stop = next_month(self.date.month);
        let mut rows = 0_u8;
        while cursor.month != stop && rows < 6 {
            if y < bottom {
                self.paint_week(area.x, y, area.width, cursor, cx);
            }
            for _ in 0..7 {
                cursor = cursor.succ();
            }
            y = y.saturating_add(1);
            rows += 1;
        }
    }

    fn paint_month_header(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = self.month_style(cx);
        cx.patch_area(area, style);
        let mut bytes = [0_u8; 32];
        let label = month_label(self.date, &mut bytes);
        let width = usize::from(area.width);
        let indent = width.saturating_sub(label.len()) / 2;
        let x = area.x.saturating_add(u16::try_from(indent).unwrap_or(0));
        for (offset, ch) in label.chars().enumerate() {
            let column = x.saturating_add(u16::try_from(offset).unwrap_or(0));
            if column >= area.x.saturating_add(area.width) {
                break;
            }
            cx.put_char_patched(column, area.y, ch, style);
        }
    }

    fn paint_weekdays(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let style = self.weekday_style(cx);
        let weekdays = match self.week_start {
            WeekStart::Sunday => WEEKDAYS,
            WeekStart::Monday => WEEKDAYS_MONDAY,
        };
        for (offset, ch) in weekdays.chars().enumerate() {
            let column = area.x.saturating_add(u16::try_from(offset).unwrap_or(0));
            if column >= area.x.saturating_add(area.width) {
                break;
            }
            cx.put_char_patched(column, area.y, ch, style);
        }
    }

    fn paint_week(&self, x: u16, y: u16, width: u16, mut cursor: Date, cx: &mut RenderCx<'_>) {
        let right = x.saturating_add(width);
        let mut column = x;
        for index in 0..7 {
            if column < right {
                let gutter = if index == 0 {
                    Style::new()
                } else {
                    self.background_style()
                };
                let has_marker = self.event_markers
                    && cursor.month == self.date.month
                    && self.events.has_event(cursor);
                let marker = if has_marker { '•' } else { ' ' };
                let marker_style = if has_marker {
                    self.events.style_for(cursor)
                } else {
                    gutter
                };
                cx.put_char_patched(column, y, marker, marker_style);
            }
            column = column.saturating_add(1);
            let (text, style) = self.format_date(cursor, cx);
            for ch in text {
                if column < right {
                    cx.put_char_patched(column, y, ch, style);
                }
                column = column.saturating_add(1);
            }
            cursor = cursor.succ();
        }
    }

    fn format_date(&self, date: Date, cx: &RenderCx<'_>) -> ([char; 2], Style) {
        let event = self.events.style_for(date);
        if date.month == self.date.month {
            let base = if self.explicit_default {
                self.default_style
            } else if self.themed {
                self.token_style(cx, date.day == self.date.day)
            } else {
                Style::new()
            };
            (day_chars(date.day), patch(base, event))
        } else if let Some(surrounding) = self.surrounding {
            let base = patch(self.default_style, surrounding);
            (day_chars(date.day), patch(base, event))
        } else if self.themed && !self.explicit_default {
            (day_chars(date.day), patch(surrounding_style(cx), event))
        } else {
            ([' ', ' '], self.background_style())
        }
    }

    fn background_style(&self) -> Style {
        match self.default_style.bg {
            lunatui_core::Color::Default => Style::new(),
            color => Style::new().bg(color),
        }
    }

    fn month_style(&self, cx: &RenderCx<'_>) -> Style {
        if let Some(style) = self.month_header {
            return style;
        }
        let (token, bold, dim) = match self.state {
            WidgetState::Focused => (Token::FocusRing, true, false),
            WidgetState::Disabled => (Token::TextPrimary, false, true),
            _ => (Token::Accent, false, false),
        };
        token_style(cx, token, bold, dim)
    }

    fn weekday_style(&self, cx: &RenderCx<'_>) -> Style {
        if let Some(style) = self.weekday {
            return style;
        }
        let dim = self.state == WidgetState::Disabled;
        token_style(cx, Token::TextPrimary, false, dim)
    }

    fn token_style(&self, cx: &RenderCx<'_>, selected: bool) -> Style {
        let (token, bold, dim) = match self.state {
            WidgetState::Disabled => (Token::TextPrimary, false, true),
            WidgetState::Focused if selected => (Token::FocusRing, true, false),
            WidgetState::Focused => (Token::TextPrimary, false, false),
            _ if selected => (Token::Accent, false, false),
            _ => (Token::TextPrimary, false, false),
        };
        token_style(cx, token, bold, dim)
    }
}

fn surrounding_style(cx: &RenderCx<'_>) -> Style {
    token_style(cx, Token::TextPrimary, false, true)
}

fn token_style(cx: &RenderCx<'_>, token: Token, bold: bool, dim: bool) -> Style {
    Style::new()
        .fg(cx.token(token))
        .with_attrs(Attrs::new().set_bold(bold).set_dim(dim))
}

fn patch(base: Style, over: Style) -> Style {
    crate::render_cx::patch_style(base, over)
}

fn day_chars(day: u8) -> [char; 2] {
    if day < 10 {
        [' ', char::from(b'0' + day)]
    } else {
        [char::from(b'0' + day / 10), char::from(b'0' + day % 10)]
    }
}

fn month_label(date: Date, bytes: &mut [u8; 32]) -> &str {
    let len = {
        let mut writer = ArrayWriter { bytes, len: 0 };
        let _ = write!(writer, "{} {}", date.month_name(), date.year);
        writer.len
    };
    core::str::from_utf8(&bytes[..len]).unwrap_or("")
}

struct ArrayWriter<'a> {
    bytes: &'a mut [u8],
    len: usize,
}

impl fmt::Write for ArrayWriter<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let next = self.len.saturating_add(text.len());
        if next > self.bytes.len() {
            return Err(fmt::Error);
        }
        self.bytes[self.len..next].copy_from_slice(text.as_bytes());
        self.len = next;
        Ok(())
    }
}

fn next_month(month: u8) -> u8 {
    if month == 12 { 1 } else { month + 1 }
}

fn week_rows(date: Date, week_start: WeekStart) -> u8 {
    let first = date.first_of_month();
    let mut cursor = match week_start {
        WeekStart::Sunday => first.sunday_on_or_before(),
        WeekStart::Monday => first.monday_on_or_before(),
    };
    let stop = next_month(date.month);
    let mut rows = 0_u8;
    while cursor.month != stop && rows < 6 {
        for _ in 0..7 {
            cursor = cursor.succ();
        }
        rows += 1;
    }
    rows
}

const fn month_length(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn is_leap(year: i32) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

fn days_from_civil(mut year: i32, month: u32, day: u32) -> i32 {
    if month <= 2 {
        year -= 1;
    }
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = u32::try_from(year - era * 400).unwrap_or(0);
    let month_adj = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * month_adj + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + i32::try_from(doe).unwrap_or(0) - 719_468
}
