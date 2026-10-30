// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, UnderlineStyle, clip_display_prefix};
use lunatui_theme::Token;

use crate::calendar::{month_label, month_length};
use crate::{Date, RenderCx, WeekStart, Widget, WidgetState};

const WIDTH: u16 = 21;
const HEIGHT: u16 = 8;
const CELL: u16 = 3;
const FIRST_WEEK_ROW: u16 = 2;
const WEEKDAYS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

/// A month grid for choosing a date or a range of dates.
///
/// The app owns the cursor (the date the keyboard is on, which also picks the month shown), the
/// selection, and the bounds. The widget draws a month header with previous and next arrows, a
/// weekday row, and the days in cells three columns wide. A single selected date or the two ends
/// of a range are filled with the accent color; days between them sit on a band. Days outside the
/// [`min`](DatePicker::min) and [`max`](DatePicker::max) bounds are dimmed and are not reported by
/// [`date_at`](DatePicker::date_at). With [`WidgetState::Focused`] the cursor day is underlined;
/// [`today`](DatePicker::today) marks a day in the accent color.
///
/// [`shift_days`](DatePicker::shift_days), [`shift_months`](DatePicker::shift_months) and
/// [`clamped`](DatePicker::clamped) move a cursor for arrow and page keys, and
/// [`prev_area`](DatePicker::prev_area), [`next_area`](DatePicker::next_area) and
/// [`date_at`](DatePicker::date_at) are for hit testing. The picker is always
/// [`WIDTH`](DatePicker::WIDTH) columns by [`HEIGHT`](DatePicker::HEIGHT) rows so the layout does
/// not jump between months.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatePicker {
    cursor: Date,
    selection: Option<(Date, Date)>,
    today: Option<Date>,
    min: Option<Date>,
    max: Option<Date>,
    week_start: WeekStart,
    state: WidgetState,
}

impl DatePicker {
    /// Columns the picker draws in.
    pub const WIDTH: u16 = WIDTH;
    /// Rows the picker draws in.
    pub const HEIGHT: u16 = HEIGHT;

    /// A picker showing the month of `cursor`, with the cursor on that day.
    #[must_use]
    pub const fn new(cursor: Date) -> Self {
        Self {
            cursor,
            selection: None,
            today: None,
            min: None,
            max: None,
            week_start: WeekStart::Sunday,
            state: WidgetState::Normal,
        }
    }

    /// Marks `date` as the chosen day.
    #[must_use]
    pub const fn selected(mut self, date: Date) -> Self {
        self.selection = Some((date, date));
        self
    }

    /// Marks the days from `a` to `b`, in either order, as the chosen range.
    #[must_use]
    pub fn range(mut self, a: Date, b: Date) -> Self {
        self.selection = Some(if a <= b { (a, b) } else { (b, a) });
        self
    }

    /// Draws `date` in the accent color as today.
    #[must_use]
    pub const fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    /// The earliest date that can be chosen.
    #[must_use]
    pub const fn min(mut self, date: Date) -> Self {
        self.min = Some(date);
        self
    }

    /// The latest date that can be chosen.
    #[must_use]
    pub const fn max(mut self, date: Date) -> Self {
        self.max = Some(date);
        self
    }

    /// The first weekday of each row. Sunday by default.
    #[must_use]
    pub const fn week_start(mut self, week_start: WeekStart) -> Self {
        self.week_start = week_start;
        self
    }

    /// Interaction state: focused underlines the cursor day, disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Whether `date` is within the bounds.
    #[must_use]
    pub fn allows(&self, date: Date) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    /// `date` moved to the nearest date within the bounds.
    #[must_use]
    pub fn clamped(&self, date: Date) -> Date {
        let raised = self.min.map_or(date, |min| date.max(min));
        self.max.map_or(raised, |max| raised.min(max))
    }

    /// `date` moved by `delta` days, for arrow keys. The cost grows with the size of `delta`.
    #[must_use]
    pub fn shift_days(date: Date, delta: i32) -> Date {
        let step = if delta.is_negative() {
            Date::pred
        } else {
            Date::succ
        };
        let mut moved = date;
        for _ in 0..delta.unsigned_abs() {
            moved = step(moved);
        }
        moved
    }

    /// `date` moved by `delta` months, for page keys. A day that the new month does not have
    /// becomes the last day of it.
    #[must_use]
    pub fn shift_months(date: Date, delta: i32) -> Date {
        let months = i64::from(date.year()) * 12 + i64::from(date.month()) - 1 + i64::from(delta);
        let year = i32::try_from(months.div_euclid(12)).unwrap_or_else(|_| {
            if delta.is_negative() {
                i32::MIN
            } else {
                i32::MAX
            }
        });
        let month = u8::try_from(months.rem_euclid(12)).unwrap_or(0) + 1;
        let day = date.day().min(month_length(year, month));
        Date::from_ymd(year, month, day).unwrap_or(date)
    }

    /// The cells of the previous month arrow in `area`.
    #[must_use]
    pub fn prev_area(&self, area: Rect) -> Rect {
        Rect::new(area.x, area.y, CELL.min(area.width), area.height.min(1))
    }

    /// The cells of the next month arrow in `area`.
    #[must_use]
    pub fn next_area(&self, area: Rect) -> Rect {
        let x = area.x.saturating_add(WIDTH - CELL);
        let right = area.x.saturating_add(area.width);
        Rect::new(
            x.min(right),
            area.y,
            CELL.min(right.saturating_sub(x)),
            area.height.min(1),
        )
    }

    /// The date of the day drawn at (`x`, `y`) in `area`, if it is within the bounds.
    #[must_use]
    pub fn date_at(&self, area: Rect, x: u16, y: u16) -> Option<Date> {
        let column = x.checked_sub(area.x)?;
        let row = y.checked_sub(area.y)?.checked_sub(FIRST_WEEK_ROW)?;
        if column >= WIDTH.min(area.width) || row + FIRST_WEEK_ROW >= HEIGHT.min(area.height) {
            return None;
        }
        let index = row * 7 + column / CELL;
        let day = index.checked_sub(self.offset())? + 1;
        let date = Date::from_ymd(
            self.cursor.year(),
            self.cursor.month(),
            u8::try_from(day).ok()?,
        )?;
        self.allows(date).then_some(date)
    }

    fn offset(&self) -> u16 {
        let weekday = u16::from(self.cursor.first_of_month().weekday());
        match self.week_start {
            WeekStart::Sunday => weekday,
            WeekStart::Monday => (weekday + 6) % 7,
        }
    }

    fn can_go_back(&self) -> bool {
        self.min
            .is_none_or(|min| min < self.cursor.first_of_month())
    }

    fn can_go_forward(&self) -> bool {
        let last = Date::from_ymd(
            self.cursor.year(),
            self.cursor.month(),
            month_length(self.cursor.year(), self.cursor.month()),
        );
        self.max
            .is_none_or(|max| last.is_none_or(|last| max > last))
    }

    fn styled(cx: &RenderCx<'_>, fg: Token, bg: Token, flags: (bool, bool, bool)) -> Style {
        let (bold, dim, underline) = flags;
        let line = if underline {
            UnderlineStyle::Single
        } else {
            UnderlineStyle::None
        };
        Style::new()
            .fg(cx.token(fg))
            .bg(cx.token(bg))
            .with_attrs(Attrs::new().set_bold(bold).set_dim(dim).set_underline(line))
    }

    fn render_header(&self, area: Rect, dim: bool, cx: &mut RenderCx<'_>) {
        let fill = Self::styled(cx, Token::TextPrimary, Token::Surface0, (true, dim, false));
        let mut bytes = [0_u8; 32];
        let label = month_label(self.cursor, &mut bytes);
        let at = (WIDTH - u16::try_from(label.len()).unwrap_or(WIDTH)) / 2;
        put(cx, area, at, 0, label, fill);
        let back = Self::styled(
            cx,
            Token::Accent,
            Token::Surface0,
            (self.can_go_back(), dim || !self.can_go_back(), false),
        );
        let forward = Self::styled(
            cx,
            Token::Accent,
            Token::Surface0,
            (self.can_go_forward(), dim || !self.can_go_forward(), false),
        );
        put(cx, area, 1, 0, "‹", back);
        put(cx, area, WIDTH - 2, 0, "›", forward);
        let heading = Self::styled(
            cx,
            Token::TextPrimary,
            Token::Surface0,
            (false, true, false),
        );
        for column in 0..7_u16 {
            let name = match self.week_start {
                WeekStart::Sunday => column,
                WeekStart::Monday => (column + 1) % 7,
            };
            put(
                cx,
                area,
                column * CELL + 1,
                1,
                WEEKDAYS[usize::from(name)],
                heading,
            );
        }
    }

    fn day_styles(&self, date: Date, cx: &RenderCx<'_>) -> (Style, Style) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let plain = Self::styled(
            cx,
            Token::TextPrimary,
            Token::Surface0,
            (false, disabled, false),
        );
        let selection = self.selection;
        let band = selection.is_some_and(|(start, end)| start < date && date <= end);
        let gutter = if band {
            Self::styled(
                cx,
                Token::TextPrimary,
                Token::Surface1,
                (false, disabled, false),
            )
        } else {
            plain
        };
        let end = selection.is_some_and(|(start, end)| date == start || date == end);
        let inside = selection.is_some_and(|(start, end)| start < date && date < end);
        let today = self.today == Some(date);
        let cursor = focused && date == self.cursor;
        let flags = (
            end || today || cursor,
            disabled || !self.allows(date),
            cursor,
        );
        let digits = if end {
            Self::styled(cx, Token::Surface0, Token::Accent, flags)
        } else if inside {
            Self::styled(cx, Token::TextPrimary, Token::Surface1, flags)
        } else if today {
            Self::styled(cx, Token::Accent, Token::Surface0, flags)
        } else {
            Self::styled(cx, Token::TextPrimary, Token::Surface0, flags)
        };
        (gutter, digits)
    }

    fn render_days(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let offset = self.offset();
        let length = month_length(self.cursor.year(), self.cursor.month());
        for day in 1..=length {
            let Some(date) = Date::from_ymd(self.cursor.year(), self.cursor.month(), day) else {
                continue;
            };
            let index = offset + u16::from(day) - 1;
            let (column, row) = (index % 7, index / 7);
            let (gutter, digits) = self.day_styles(date, cx);
            let text = [
                if day < 10 { b' ' } else { b'0' + day / 10 },
                b'0' + day % 10,
            ];
            let at = column * CELL;
            put(cx, area, at, FIRST_WEEK_ROW + row, " ", gutter);
            put(
                cx,
                area,
                at + 1,
                FIRST_WEEK_ROW + row,
                core::str::from_utf8(&text).unwrap_or(""),
                digits,
            );
        }
    }
}

fn put(cx: &mut RenderCx<'_>, area: Rect, dx: u16, dy: u16, text: &str, style: Style) {
    if dx >= area.width || dy >= area.height {
        return;
    }
    let shown = clip_display_prefix(text, area.width - dx);
    cx.put_str(area.x + dx, area.y + dy, shown, style);
}

impl Widget for DatePicker {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let disabled = matches!(self.state, WidgetState::Disabled);
        let base = Self::styled(
            cx,
            Token::TextPrimary,
            Token::Surface0,
            (false, disabled, false),
        );
        cx.fill(
            Rect::new(
                area.x,
                area.y,
                WIDTH.min(area.width),
                HEIGHT.min(area.height),
            ),
            base,
            ' ',
        );
        self.render_header(area, disabled, cx);
        self.render_days(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(WIDTH, HEIGHT).clamp(avail)
    }
}
