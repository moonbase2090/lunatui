// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::widget_suite::{CASES, Case, pin_color_output};
use lunatui_core::{
    Attrs, ClusterTable, Color, Grid, Rect, Style, clip_display_prefix, width as cols,
};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{
    Block, BorderType, Date, DateStyle, Monthly, RenderCx, Sparkline, Widget, WidgetState,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::Color as RatColor;
use ratatui::{Terminal, TerminalOptions, Viewport};
use time::{Date as TimeDate, Month};

type NativeDraw = fn(&mut Grid, &mut ClusterTable, &Theme, Rect, u32);
type BaselineDraw = fn(&mut Buffer, RatRect, &Theme, u32);

struct Preview {
    name: &'static str,
    native: NativeDraw,
    baseline: BaselineDraw,
}

const PREVIEWS: &[Preview] = &[
    Preview {
        name: "calendar",
        native: calendar_native,
        baseline: calendar_baseline,
    },
    Preview {
        name: "sparkline",
        native: sparkline_native,
        baseline: sparkline_baseline,
    },
];

fn preview(name: &str) -> Option<&'static Preview> {
    PREVIEWS.iter().find(|preview| preview.name == name)
}

const DAY_HOLD: u32 = 15;
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
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const SHORT_DAYS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

#[derive(Clone, Copy)]
enum Kind {
    Release,
    Review,
    OnCall,
    Incident,
}

struct Entry {
    month: u8,
    day: u8,
    time: &'static str,
    title: &'static str,
    detail: &'static str,
    kind: Kind,
}

const ENTRIES: &[Entry] = &[
    Entry {
        month: 10,
        day: 22,
        time: "09:30",
        title: "Branch cut",
        detail: "Open the release branch for the preview.",
        kind: Kind::Release,
    },
    Entry {
        month: 10,
        day: 23,
        time: "15:00",
        title: "Design review",
        detail: "Walk the month and the agenda together.",
        kind: Kind::Review,
    },
    Entry {
        month: 10,
        day: 24,
        time: "11:00",
        title: "Docs pass",
        detail: "Read the gallery notes before they freeze.",
        kind: Kind::Review,
    },
    Entry {
        month: 10,
        day: 26,
        time: "10:00",
        title: "Ship window",
        detail: "Tag the preview and publish the notes.",
        kind: Kind::Release,
    },
    Entry {
        month: 10,
        day: 26,
        time: "16:30",
        title: "On-call handoff",
        detail: "Primary swaps at the end of the day.",
        kind: Kind::OnCall,
    },
    Entry {
        month: 10,
        day: 28,
        time: "14:00",
        title: "Incident review",
        detail: "Trace last week's error budget.",
        kind: Kind::Incident,
    },
    Entry {
        month: 10,
        day: 30,
        time: "17:00",
        title: "Docs freeze",
        detail: "Gallery and changelog stop moving.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 1,
        time: "18:00",
        title: "November board",
        detail: "Move open notes onto the new month.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 2,
        time: "10:00",
        title: "Sprint plan",
        detail: "Pick the next set of widgets.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 3,
        time: "09:00",
        title: "Beta cut",
        detail: "Cut the beta from the release branch.",
        kind: Kind::Release,
    },
    Entry {
        month: 11,
        day: 4,
        time: "13:00",
        title: "Demo",
        detail: "Show the schedule on a real screen.",
        kind: Kind::Release,
    },
    Entry {
        month: 11,
        day: 6,
        time: "15:00",
        title: "Retro",
        detail: "Walk what slowed the last cycle.",
        kind: Kind::Review,
    },
    Entry {
        month: 11,
        day: 9,
        time: "11:00",
        title: "On-call drill",
        detail: "Page the backup and time the reply.",
        kind: Kind::OnCall,
    },
];

#[derive(Clone, Copy)]
struct Ink {
    text: Color,
    surface: Color,
    raised: Color,
    accent: Color,
    ok: Color,
    warn: Color,
    fail: Color,
    focus: Color,
}

impl Ink {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            raised: cx.token(Token::Surface1),
            accent: cx.token(Token::Accent),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
            fail: cx.token(Token::Fail),
            focus: cx.token(Token::FocusRing),
        }
    }

    fn kind(self, kind: Kind) -> Color {
        match kind {
            Kind::Release => self.ok,
            Kind::Review => self.warn,
            Kind::OnCall => self.accent,
            Kind::Incident => self.fail,
        }
    }
}

struct Scene {
    ink: Ink,
    tick: u32,
    width: u16,
    height: u16,
    selected: Date,
    today: Date,
}

struct DayMarks {
    today: Date,
    selected: Date,
    raised: Color,
    release: Color,
    review: Color,
    oncall: Color,
    incident: Color,
}

impl DateStyle for DayMarks {
    fn style_for(&self, date: Date) -> Style {
        if date == self.selected {
            return Style::new();
        }
        let mut style = Style::new();
        if date == self.today {
            style = style.bg(self.raised);
        }
        if let Some(entry) = first_entry(date) {
            let color = match entry.kind {
                Kind::Release => self.release,
                Kind::Review => self.review,
                Kind::OnCall => self.oncall,
                Kind::Incident => self.incident,
            };
            style = style.fg(color);
        }
        style
    }
}

fn on_day(entry: &Entry, date: Date) -> bool {
    entry.month == date.month() && entry.day == date.day()
}

fn first_entry(date: Date) -> Option<&'static Entry> {
    ENTRIES.iter().find(|entry| on_day(entry, date))
}

fn month_name(date: Date) -> &'static str {
    MONTHS
        .get(usize::from(date.month().saturating_sub(1)))
        .copied()
        .unwrap_or("")
}

fn month_abbr(date: Date) -> &'static str {
    let name = month_name(date);
    name.get(..3).unwrap_or(name)
}

fn weekday_index(date: Date) -> usize {
    let Ok(month) = time::Month::try_from(date.month()) else {
        return 0;
    };
    let Ok(civil) = time::Date::from_calendar_date(date.year(), month, date.day()) else {
        return 0;
    };
    match civil.weekday() {
        time::Weekday::Sunday => 0,
        time::Weekday::Monday => 1,
        time::Weekday::Tuesday => 2,
        time::Weekday::Wednesday => 3,
        time::Weekday::Thursday => 4,
        time::Weekday::Friday => 5,
        time::Weekday::Saturday => 6,
    }
}

fn weekday_name(date: Date) -> &'static str {
    WEEKDAYS.get(weekday_index(date)).copied().unwrap_or("Day")
}

fn short_day(date: Date) -> &'static str {
    SHORT_DAYS.get(weekday_index(date)).copied().unwrap_or("Da")
}

fn add_days(date: Date, days: i32) -> Date {
    let Ok(month) = time::Month::try_from(date.month()) else {
        return date;
    };
    let Ok(civil) = time::Date::from_calendar_date(date.year(), month, date.day()) else {
        return date;
    };
    let Some(next) = civil.checked_add(time::Duration::days(i64::from(days))) else {
        return date;
    };
    Date::from_ymd(next.year(), u8::from(next.month()), next.day()).unwrap_or(date)
}

fn selected_on(tick: u32) -> Date {
    let start = Date::from_ymd(2026, 10, 24).unwrap_or(Date::UNIX_EPOCH);
    let steps = i32::try_from(tick / DAY_HOLD).unwrap_or(0);
    add_days(start, steps)
}

fn today_date() -> Date {
    Date::from_ymd(2026, 10, 26).unwrap_or(Date::UNIX_EPOCH)
}

fn bold(style: Style) -> Style {
    style.with_attrs(Attrs::new().set_bold(true))
}

fn paint(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    cx.put_str(x, y, clip_display_prefix(text, width), style);
}

fn hint_for(tick: u32, selected: Date) -> &'static str {
    let into = tick % DAY_HOLD;
    if tick > 0 && into < 8 && selected.day() == 1 {
        "n  month"
    } else if tick > 0 && into < 6 {
        "l  day"
    } else {
        "h l day     n month"
    }
}

fn draw_chrome(cx: &mut RenderCx<'_>, scene: &Scene) {
    let bar = Style::new().fg(scene.ink.text).bg(scene.ink.raised);
    cx.set_style(Rect::new(0, 0, scene.width, 1), bar);
    let badge = bold(Style::new().fg(scene.ink.surface).bg(scene.ink.ok));
    paint(cx, 1, 0, 6, " live ", badge);
    paint(cx, 8, 0, 12, "schedule", bold(bar));
    let month = format!("{} {}", month_name(scene.selected), scene.selected.year());
    let month_cols = u16::try_from(month.len()).unwrap_or(0);
    let month_x = scene.width.saturating_sub(month_cols) / 2;
    if month_x > 20 {
        paint(
            cx,
            month_x,
            0,
            month_cols,
            &month,
            bold(Style::new().fg(scene.ink.accent).bg(scene.ink.raised)),
        );
    }
    let second = (scene.tick / 30) % 60;
    let clock = format!("14:02:{second:02}");
    let clock_cols = u16::try_from(clock.len()).unwrap_or(0);
    let clock_x = scene.width.saturating_sub(clock_cols.saturating_add(1));
    paint(
        cx,
        clock_x,
        0,
        clock_cols,
        &clock,
        Style::new().fg(scene.ink.accent).bg(scene.ink.raised),
    );
    paint(
        cx,
        1,
        1,
        28,
        "prod     team calendar",
        Style::new().fg(scene.ink.text).bg(scene.ink.surface),
    );
    let footer_y = scene.height.saturating_sub(1);
    cx.set_style(Rect::new(0, footer_y, scene.width, 1), bar);
    let hint = hint_for(scene.tick, scene.selected);
    let hint_color = if hint.starts_with('n') {
        scene.ink.accent
    } else if hint.starts_with('l') {
        scene.ink.focus
    } else {
        scene.ink.text
    };
    paint(
        cx,
        1,
        footer_y,
        24,
        hint,
        Style::new().fg(hint_color).bg(scene.ink.raised),
    );
    let stamp = format!(
        "{:04}-{:02}-{:02}",
        scene.selected.year(),
        scene.selected.month(),
        scene.selected.day()
    );
    let stamp_cols = u16::try_from(stamp.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(stamp_cols.saturating_add(1)),
        footer_y,
        stamp_cols,
        &stamp,
        Style::new().fg(scene.ink.focus).bg(scene.ink.raised),
    );
}

fn day_marks(scene: &Scene) -> DayMarks {
    DayMarks {
        today: scene.today,
        selected: scene.selected,
        raised: scene.ink.raised,
        release: scene.ink.ok,
        review: scene.ink.warn,
        oncall: scene.ink.accent,
        incident: scene.ink.fail,
    }
}

fn draw_month(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.width < 23 || area.height < 9 {
        return;
    }
    let ground = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(ground)
        .border_style(Style::new().fg(scene.ink.focus).bg(scene.ink.surface));
    frame.render(area, cx);
    let inner = frame.inner(area);
    let cal_w = 21_u16;
    if inner.width < cal_w || inner.height < 7 {
        return;
    }
    let pad = inner.width.saturating_sub(cal_w) / 2;
    Monthly::new(scene.selected, day_marks(scene))
        .state(WidgetState::Focused)
        .render(
            Rect::new(
                inner.x.saturating_add(pad),
                inner.y,
                cal_w,
                inner.height.min(8),
            ),
            cx,
        );
}

fn draw_legend(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.height == 0 || area.width < 12 {
        return;
    }
    let y = area.y;
    paint(
        cx,
        area.x,
        y,
        12,
        "• release",
        Style::new().fg(scene.ink.ok).bg(scene.ink.surface),
    );
    paint(
        cx,
        area.x.saturating_add(14),
        y,
        10,
        "• review",
        Style::new().fg(scene.ink.warn).bg(scene.ink.surface),
    );
    if area.height > 1 {
        let next = y.saturating_add(1);
        paint(
            cx,
            area.x,
            next,
            12,
            "• on-call",
            Style::new().fg(scene.ink.accent).bg(scene.ink.surface),
        );
        paint(
            cx,
            area.x.saturating_add(14),
            next,
            12,
            "• incident",
            Style::new().fg(scene.ink.fail).bg(scene.ink.surface),
        );
    }
}

fn week_line(date: Date) -> String {
    let count = ENTRIES.iter().filter(|entry| on_day(entry, date)).count();
    match first_entry(date) {
        Some(entry) if count > 1 => format!(
            "{} {:02}  {} +{}",
            short_day(date),
            date.day(),
            entry.title,
            count - 1
        ),
        Some(entry) => format!("{} {:02}  {}", short_day(date), date.day(), entry.title),
        None => format!("{} {:02}", short_day(date), date.day()),
    }
}

fn draw_week(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.height < 4 || area.width < 12 {
        return;
    }
    let label = Style::new().fg(scene.ink.accent).bg(scene.ink.surface);
    paint(cx, area.x, area.y, area.width, "This week", bold(label));
    let index = weekday_index(scene.selected);
    let sunday = add_days(scene.selected, -i32::try_from(index).unwrap_or(0));
    let text = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    for offset in 0_i32..7 {
        let y = area
            .y
            .saturating_add(1)
            .saturating_add(u16::try_from(offset).unwrap_or(0));
        if y >= area.y.saturating_add(area.height) {
            break;
        }
        let date = add_days(sunday, offset);
        let picked = date == scene.selected;
        let style = if picked {
            Style::new().fg(scene.ink.focus).bg(scene.ink.raised)
        } else if date == scene.today {
            Style::new().fg(scene.ink.ok).bg(scene.ink.surface)
        } else {
            text
        };
        if picked {
            cx.set_style(
                Rect::new(area.x, y, area.width, 1),
                Style::new().fg(scene.ink.text).bg(scene.ink.raised),
            );
        }
        paint(
            cx,
            area.x,
            y,
            area.width,
            &week_line(date),
            if picked { bold(style) } else { style },
        );
    }
}

fn draw_event_chip(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, entry: &Entry, y: u16) {
    let chip = Style::new().fg(scene.ink.text).bg(scene.ink.raised);
    cx.set_style(Rect::new(area.x, y, area.width, 2), chip);
    let mark = Style::new()
        .fg(scene.ink.kind(entry.kind))
        .bg(scene.ink.raised);
    paint(cx, area.x, y, 1, "▌", mark);
    paint(cx, area.x.saturating_add(2), y, 5, entry.time, mark);
    paint(
        cx,
        area.x.saturating_add(9),
        y,
        area.width.saturating_sub(9),
        entry.title,
        bold(chip),
    );
    paint(
        cx,
        area.x.saturating_add(2),
        y.saturating_add(1),
        area.width.saturating_sub(2),
        entry.detail,
        chip,
    );
}

fn draw_up_next(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, y: u16) {
    if y.saturating_add(1) >= area.y.saturating_add(area.height) {
        return;
    }
    paint(
        cx,
        area.x,
        y,
        area.width,
        "Up next",
        bold(Style::new().fg(scene.ink.accent).bg(scene.ink.surface)),
    );
    let mut line = y.saturating_add(1);
    let bottom = area.y.saturating_add(area.height);
    let text = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    for step in 1..22 {
        if line >= bottom {
            break;
        }
        let date = add_days(scene.selected, step);
        for entry in ENTRIES.iter().filter(|entry| on_day(entry, date)) {
            if line >= bottom {
                break;
            }
            paint(
                cx,
                area.x,
                line,
                area.width,
                &format!(
                    "{:02} {}  {}  {}",
                    date.day(),
                    month_abbr(date),
                    entry.time,
                    entry.title
                ),
                text.fg(scene.ink.kind(entry.kind)),
            );
            line = line.saturating_add(1);
        }
    }
}

fn draw_agenda(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.width < 24 || area.height < 8 {
        return;
    }
    let ground = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let title = if scene.selected == scene.today {
        format!(
            " {} {} {}  ·  today ",
            weekday_name(scene.selected),
            scene.selected.day(),
            month_name(scene.selected)
        )
    } else {
        format!(
            " {} {} {} ",
            weekday_name(scene.selected),
            scene.selected.day(),
            month_name(scene.selected)
        )
    };
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(ground)
        .border_style(Style::new().fg(scene.ink.accent).bg(scene.ink.surface))
        .title_style(bold(Style::new().fg(scene.ink.text).bg(scene.ink.surface)));
    frame.render(area, cx);
    let inner = frame.inner(area);
    if inner.width < 16 || inner.height < 4 {
        return;
    }
    let count = ENTRIES
        .iter()
        .filter(|entry| on_day(entry, scene.selected))
        .count();
    let summary = if count == 0 {
        "Clear day".to_owned()
    } else if count == 1 {
        "1 scheduled".to_owned()
    } else {
        format!("{count} scheduled")
    };
    paint(
        cx,
        inner.x,
        inner.y,
        inner.width,
        &summary,
        Style::new().fg(scene.ink.focus).bg(scene.ink.surface),
    );
    let mut y = inner.y.saturating_add(2);
    let bottom = inner.y.saturating_add(inner.height);
    for entry in ENTRIES.iter().filter(|entry| on_day(entry, scene.selected)) {
        if y.saturating_add(2) >= bottom {
            break;
        }
        draw_event_chip(cx, scene, inner, entry, y);
        y = y.saturating_add(3);
    }
    if count == 0 && y < bottom {
        paint(cx, inner.x, y, inner.width, "Nothing scheduled.", ground);
        y = y.saturating_add(2);
    } else {
        y = y.saturating_add(1);
    }
    draw_up_next(cx, scene, inner, y);
}

fn draw_board(cx: &mut RenderCx<'_>, scene: &Scene) {
    let top = 3_u16;
    let bottom = scene.height.saturating_sub(2);
    if bottom <= top {
        return;
    }
    let left_w = if scene.width >= 110 { 40 } else { 32 };
    let month = Rect::new(1, top, left_w, 9);
    draw_month(cx, scene, month);
    let mut y = top.saturating_add(10);
    if bottom.saturating_sub(y) >= 10 {
        draw_legend(cx, scene, Rect::new(2, y, left_w.saturating_sub(2), 2));
        y = y.saturating_add(3);
    }
    draw_week(
        cx,
        scene,
        Rect::new(2, y, left_w.saturating_sub(2), bottom.saturating_sub(y)),
    );
    let agenda_x = left_w.saturating_add(2);
    draw_agenda(
        cx,
        scene,
        Rect::new(
            agenda_x,
            top,
            scene.width.saturating_sub(agenda_x).saturating_sub(1),
            bottom.saturating_sub(top),
        ),
    );
}

fn calendar_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let scene = Scene {
        ink: Ink::new(&cx),
        tick,
        width: area.width,
        height: area.height,
        selected: selected_on(tick),
        today: today_date(),
    };
    draw_chrome(&mut cx, &scene);
    if scene.width < 64 || scene.height < 20 {
        draw_month(
            &mut cx,
            &scene,
            Rect::new(
                1,
                3,
                scene.width.saturating_sub(2),
                scene.height.saturating_sub(5),
            ),
        );
        return;
    }
    draw_board(&mut cx, &scene);
}

struct NoEvents;

impl ratatui::widgets::calendar::DateStyler for NoEvents {
    fn get_style(&self, _date: TimeDate) -> ratatui::style::Style {
        ratatui::style::Style::new()
    }
}

fn calendar_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let month = match ((tick / 4) % 12) + 1 {
        1 => Month::January,
        2 => Month::February,
        3 => Month::March,
        4 => Month::April,
        5 => Month::May,
        6 => Month::June,
        7 => Month::July,
        8 => Month::August,
        9 => Month::September,
        10 => Month::October,
        11 => Month::November,
        _ => Month::December,
    };
    let date = TimeDate::from_calendar_date(2026, month, 1).unwrap_or(TimeDate::MIN);
    let color = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    };
    let style = ratatui::style::Style::default().fg(color);
    ratatui::widgets::Widget::render(
        ratatui::widgets::calendar::Monthly::new(date, NoEvents)
            .default_style(style)
            .show_month_header(style)
            .show_weekdays_header(style)
            .show_surrounding(style),
        area,
        buffer,
    );
}

fn wave(index: u32) -> u64 {
    let phase = index % 72;
    let rise = if phase < 36 { phase } else { 72 - phase };
    u64::from(18 + rise.saturating_mul(3) + (index % 11).saturating_mul(4))
}

fn fill_wave(samples: &mut [u64], tick: u32) {
    for (offset, sample) in samples.iter_mut().enumerate() {
        let index = tick.saturating_add(u32::try_from(offset).unwrap_or(0));
        *sample = wave(index);
    }
}

const SELECTION_HOLD: u32 = 24;

#[derive(Clone, Copy)]
enum SparkKind {
    Cpu,
    Mem,
    NetIn,
    NetOut,
    Requests,
    Latency,
    Errors,
    Disk,
    Cache,
    Conns,
}

#[derive(Clone, Copy)]
struct Lane {
    name: &'static str,
    rows: u16,
    ceiling: u64,
    threshold: Option<u64>,
    kind: SparkKind,
}

const LANES: [Lane; 10] = [
    Lane {
        name: "cpu",
        rows: 2,
        ceiling: 100,
        threshold: None,
        kind: SparkKind::Cpu,
    },
    Lane {
        name: "mem",
        rows: 2,
        ceiling: 100,
        threshold: None,
        kind: SparkKind::Mem,
    },
    Lane {
        name: "net in",
        rows: 2,
        ceiling: 100,
        threshold: None,
        kind: SparkKind::NetIn,
    },
    Lane {
        name: "net out",
        rows: 2,
        ceiling: 80,
        threshold: None,
        kind: SparkKind::NetOut,
    },
    Lane {
        name: "req/s",
        rows: 3,
        ceiling: 2_000,
        threshold: None,
        kind: SparkKind::Requests,
    },
    Lane {
        name: "p99",
        rows: 2,
        ceiling: 200,
        threshold: None,
        kind: SparkKind::Latency,
    },
    Lane {
        name: "errors",
        rows: 2,
        ceiling: 24,
        threshold: Some(8),
        kind: SparkKind::Errors,
    },
    Lane {
        name: "disk",
        rows: 1,
        ceiling: 100,
        threshold: None,
        kind: SparkKind::Disk,
    },
    Lane {
        name: "cache",
        rows: 1,
        ceiling: 100,
        threshold: None,
        kind: SparkKind::Cache,
    },
    Lane {
        name: "conns",
        rows: 1,
        ceiling: 1_000,
        threshold: None,
        kind: SparkKind::Conns,
    },
];

struct SparkInk {
    text: Color,
    accent: Color,
    surface: Color,
    raised: Color,
    ok: Color,
    fail: Color,
    focus: Color,
}

struct Geom {
    width: u16,
    spark_x: u16,
    spark_w: u16,
    stat_x: u16,
    stat_w: u16,
}

struct SparkScene<'a> {
    ink: &'a SparkInk,
    geom: &'a Geom,
    height: u16,
    newest: u32,
    tick: u32,
}

struct Board {
    header_rows: u16,
    lanes: [(u16, u16); 10],
    shown: usize,
    inspector_y: u16,
    inspector_rows: u16,
}

fn ink_from(cx: &RenderCx<'_>) -> SparkInk {
    SparkInk {
        text: cx.token(Token::TextPrimary),
        accent: cx.token(Token::Accent),
        surface: cx.token(Token::Surface0),
        raised: cx.token(Token::Surface1),
        ok: cx.token(Token::Ok),
        fail: cx.token(Token::Fail),
        focus: cx.token(Token::FocusRing),
    }
}

fn geom(width: u16) -> Geom {
    let stat_w: u16 = 22;
    let spark_x = 12;
    let stat_x = width.saturating_sub(stat_w.saturating_add(1));
    Geom {
        width,
        spark_x,
        spark_w: stat_x.saturating_sub(spark_x).saturating_sub(1),
        stat_x,
        stat_w,
    }
}

fn arrange(height: u16) -> Board {
    let header_rows: u16 = if height >= 32 { 3 } else { 2 };
    let mut shown = LANES.len();
    let mut rows = LANES.map(|lane| lane.rows);
    let mut gaps = true;
    let mut inspector_rows: u16 = if height >= 22 { 3 } else { 0 };
    loop {
        let gap_n = if gaps && shown > 1 {
            u16::try_from(shown - 1).unwrap_or(0)
        } else {
            0
        };
        let body = rows
            .iter()
            .take(shown)
            .fold(0_u16, |acc, row| acc.saturating_add(*row));
        let inspector_block = if inspector_rows == 0 {
            0
        } else {
            inspector_rows.saturating_add(2)
        };
        let need = header_rows
            .saturating_add(body)
            .saturating_add(gap_n)
            .saturating_add(inspector_block);
        if need <= height || shown == 0 {
            break;
        }
        if gaps {
            gaps = false;
            continue;
        }
        if inspector_rows > 1 {
            inspector_rows -= 1;
            continue;
        }
        if inspector_rows == 1 {
            inspector_rows = 0;
            continue;
        }
        if shown > 6 {
            shown -= 1;
            continue;
        }
        if let Some(index) = rows.iter().take(shown).rposition(|row| *row > 1) {
            rows[index] -= 1;
            continue;
        }
        if shown > 1 {
            shown -= 1;
            continue;
        }
        break;
    }
    let mut y = header_rows;
    let mut lanes = [(0_u16, 0_u16); 10];
    for index in 0..shown {
        lanes[index] = (y, rows[index]);
        y = y.saturating_add(rows[index]);
        if gaps && index + 1 < shown {
            y = y.saturating_add(1);
        }
    }
    let inspector_block = if inspector_rows == 0 {
        0
    } else {
        inspector_rows.saturating_add(2)
    };
    let inspector_y = height.saturating_sub(inspector_block);
    if inspector_rows > 0 && y > inspector_y {
        inspector_rows = 0;
    }
    Board {
        header_rows,
        lanes,
        shown,
        inspector_y,
        inspector_rows,
    }
}

fn tri(t: u32, period: u32, low: u64, high: u64) -> u64 {
    let period = period.max(2);
    let phase = t % period;
    let half = period / 2;
    let up = if phase <= half { phase } else { period - phase };
    let span = high.saturating_sub(low);
    low.saturating_add(span.saturating_mul(u64::from(up)) / u64::from(half.max(1)))
}

fn wobble(t: u32, amp: u64) -> i64 {
    let amp_u = u32::try_from(amp).unwrap_or(u32::MAX);
    if amp_u == 0 {
        return 0;
    }
    let span = amp_u.saturating_mul(2).saturating_add(1);
    let mixed = t.wrapping_mul(17).wrapping_add(11) % span;
    i64::from(mixed) - i64::from(amp_u)
}

fn offset(base: u64, delta: i64, low: u64, high: u64) -> u64 {
    let next = if delta < 0 {
        base.saturating_sub(delta.unsigned_abs())
    } else {
        base.saturating_add(u64::try_from(delta).unwrap_or(0))
    };
    next.clamp(low, high)
}

fn sample_at(kind: SparkKind, t: u32) -> u64 {
    match kind {
        SparkKind::Cpu => offset(tri(t, 36, 18, 92), wobble(t, 4), 0, 100),
        SparkKind::Mem => offset(tri(t, 96, 62, 86), wobble(t, 2), 0, 100),
        SparkKind::NetIn => offset(tri(t, 28, 4, 84), wobble(t, 6), 0, 100),
        SparkKind::NetOut => offset(tri(t, 32, 2, 42), wobble(t, 3), 0, 80),
        SparkKind::Requests => offset(tri(t, 22, 420, 1_760), wobble(t, 40), 0, 2_000),
        SparkKind::Latency => {
            if t % 19 < 2 {
                168
            } else {
                offset(tri(t, 46, 32, 78), wobble(t, 3), 0, 200)
            }
        }
        SparkKind::Errors => {
            let pulse = if (120..186).contains(&t) {
                let span = if t < 153 { t - 120 } else { 186 - t };
                u64::from(span / 2)
            } else {
                0
            };
            1 + u64::from(t % 3) + pulse
        }
        SparkKind::Disk => offset(tri(t, 80, 22, 58), wobble(t, 2), 0, 100),
        SparkKind::Cache => offset(tri(t, 40, 90, 99), wobble(t, 1), 0, 100),
        SparkKind::Conns => offset(tri(t, 54, 260, 820), wobble(t, 12), 0, 1_000),
    }
}

fn fill_ending(kind: SparkKind, newest: u32, samples: &mut [u64]) {
    let last = samples.len().saturating_sub(1);
    for (index, slot) in samples.iter_mut().enumerate() {
        let age = u32::try_from(last - index).unwrap_or(0);
        *slot = sample_at(kind, newest.saturating_sub(age));
    }
}

fn bounds(samples: &[u64]) -> (u64, u64) {
    let mut low = u64::MAX;
    let mut high = 0_u64;
    for sample in samples {
        low = low.min(*sample);
        high = high.max(*sample);
    }
    if samples.is_empty() {
        (0, 0)
    } else {
        (low, high)
    }
}

fn numeral(kind: SparkKind, value: u64) -> String {
    match kind {
        SparkKind::Cpu
        | SparkKind::Mem
        | SparkKind::Disk
        | SparkKind::Cache
        | SparkKind::Errors => format!("{value}%"),
        SparkKind::NetIn | SparkKind::NetOut => format!("{value}M"),
        SparkKind::Requests => format!("{}.{}k", value / 1_000, (value % 1_000) / 100),
        SparkKind::Conns => format!("{value}"),
        SparkKind::Latency => format!("{value}ms"),
    }
}

fn right(text: &str, columns: usize) -> String {
    let limit = u16::try_from(columns).unwrap_or(0);
    let clipped = clip_display_prefix(text, limit);
    let pad = columns.saturating_sub(cols(clipped));
    let mut out = String::with_capacity(columns);
    out.extend(std::iter::repeat_n(' ', pad));
    out.push_str(clipped);
    out
}

fn clock_text(tick: u32) -> String {
    let total = 14 * 3_600 + 2 * 60 + tick / 30;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3_600,
        (total / 60) % 60,
        total % 60
    )
}

fn selected_index(tick: u32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    usize::try_from(tick / SELECTION_HOLD).unwrap_or(0) % count
}

fn newest_at(tick: u32, columns: u16) -> u32 {
    (tick / 2).saturating_add(u32::from(columns.saturating_sub(1)))
}

fn dim_on(ink: &SparkInk, bg: Color) -> Style {
    Style::new()
        .fg(ink.text)
        .bg(bg)
        .with_attrs(Attrs::new().set_dim(true))
}

fn draw_spark_chrome(
    cx: &mut RenderCx<'_>,
    area: Rect,
    ink: &SparkInk,
    tick: u32,
    header_rows: u16,
) {
    if area.height == 0 {
        return;
    }
    cx.set_style(Rect::new(0, 0, area.width, 1), Style::new().bg(ink.raised));
    let badge = Style::new()
        .fg(ink.surface)
        .bg(ink.ok)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(1, 0, " live ", badge);
    cx.put_str(
        8,
        0,
        "services",
        Style::new()
            .fg(ink.text)
            .bg(ink.raised)
            .with_attrs(Attrs::new().set_bold(true)),
    );
    let clock = clock_text(tick);
    let clock_w = u16::try_from(clock.len()).unwrap_or(0);
    let clock_x = area.width.saturating_sub(clock_w.saturating_add(1));
    if clock_x > 20 {
        cx.put_str(
            clock_x,
            0,
            &clock,
            Style::new().fg(ink.accent).bg(ink.raised),
        );
    }
    if header_rows >= 2 && area.height > 1 {
        let dim = dim_on(ink, ink.surface);
        cx.put_str(2, 1, "prod     60s window", dim);
        let since = tick % SELECTION_HOLD;
        let keys = if tick > 0 && since < 8 { "k  " } else { "j k" };
        let hint = format!("{keys}  move");
        let hint_w = u16::try_from(hint.len()).unwrap_or(0);
        let hint_x = area.width.saturating_sub(hint_w.saturating_add(1));
        if hint_x > 28 {
            cx.put_str(hint_x, 1, &hint, dim);
        }
    }
    if header_rows >= 3 && area.height > 2 {
        let dim = dim_on(ink, ink.surface);
        cx.put_str(2, 2, "metric", dim);
        let geom = geom(area.width);
        let head = format!("{:>8} {:>6} {:>6}", "now", "min", "max");
        cx.put_str(geom.stat_x, 2, clip_display_prefix(&head, geom.stat_w), dim);
    }
}

fn draw_lane(
    cx: &mut RenderCx<'_>,
    scene: &SparkScene<'_>,
    lane: &Lane,
    top: u16,
    rows: u16,
    selected: bool,
) {
    if rows == 0 {
        return;
    }
    let ink = scene.ink;
    let geom = scene.geom;
    let newest = scene.newest;
    let bg = if selected { ink.raised } else { ink.surface };
    if selected {
        cx.set_style(
            Rect::new(0, top, geom.width, rows),
            Style::new().bg(ink.raised),
        );
    }
    let mut samples = [0_u64; 240];
    let columns = usize::from(geom.spark_w).min(samples.len());
    fill_ending(lane.kind, newest, &mut samples[..columns]);
    let latest = samples.get(columns.saturating_sub(1)).copied().unwrap_or(0);
    let (low, high) = bounds(&samples[..columns]);
    let hot = lane.threshold.is_some_and(|limit| latest > limit);
    if columns > 0 {
        let spark = Sparkline::new(&samples[..columns]).max(lane.ceiling);
        let spark = if hot {
            spark.style(Style::new().fg(ink.fail))
        } else if selected {
            spark.state(WidgetState::Focused)
        } else {
            spark.state(WidgetState::Normal)
        };
        spark.render(Rect::new(geom.spark_x, top, geom.spark_w, rows), cx);
    }
    if selected || hot {
        let marker = if selected { ink.focus } else { ink.fail };
        for row in 0..rows {
            cx.put_str(0, top.saturating_add(row), " ", Style::new().bg(marker));
        }
    }
    let label_fg = if hot {
        ink.fail
    } else if selected {
        ink.focus
    } else {
        ink.text
    };
    let label_attr = if selected || hot {
        Attrs::new().set_bold(true)
    } else {
        Attrs::new()
    };
    let text_y = top.saturating_add(rows.saturating_sub(1) / 2);
    cx.put_str(
        2,
        text_y,
        clip_display_prefix(lane.name, 9),
        Style::new().fg(label_fg).bg(bg).with_attrs(label_attr),
    );
    let now_fg = if hot { ink.fail } else { ink.text };
    let now_attr = if selected || hot {
        Attrs::new().set_bold(true)
    } else {
        Attrs::new()
    };
    cx.put_str(
        geom.stat_x,
        text_y,
        &right(&numeral(lane.kind, latest), 8),
        Style::new().fg(now_fg).bg(bg).with_attrs(now_attr),
    );
    let rest = format!(
        "{} {}",
        right(&numeral(lane.kind, low), 6),
        right(&numeral(lane.kind, high), 6)
    );
    cx.put_str(
        geom.stat_x.saturating_add(9),
        text_y,
        clip_display_prefix(&rest, geom.stat_w.saturating_sub(9)),
        dim_on(ink, bg),
    );
}

fn draw_inspector(
    cx: &mut RenderCx<'_>,
    scene: &SparkScene<'_>,
    lane: &Lane,
    board: &Board,
    selected: usize,
) {
    let ink = scene.ink;
    let geom = scene.geom;
    let newest = scene.newest;
    let tick = scene.tick;
    let width = geom.width;
    let height = scene.height;
    let y = board.inspector_y;
    let rows = board.inspector_rows;
    if rows == 0 || y >= height {
        return;
    }
    cx.set_style(Rect::new(0, y, width, 1), Style::new().bg(ink.raised));
    let mut samples = [0_u64; 240];
    let columns = usize::from(geom.spark_w).min(samples.len());
    fill_ending(lane.kind, newest, &mut samples[..columns]);
    let latest = samples.get(columns.saturating_sub(1)).copied().unwrap_or(0);
    let (low, high) = bounds(&samples[..columns]);
    let hot = lane.threshold.is_some_and(|limit| latest > limit);
    let name_style = Style::new()
        .fg(if hot { ink.fail } else { ink.focus })
        .bg(ink.raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(2, y, clip_display_prefix(lane.name, 12), name_style);
    let summary = format!(
        "{}   min {}   max {}",
        numeral(lane.kind, latest),
        numeral(lane.kind, low),
        numeral(lane.kind, high)
    );
    let summary_x = 16_u16;
    cx.put_str(
        summary_x,
        y,
        clip_display_prefix(&summary, width.saturating_sub(summary_x).saturating_sub(28)),
        Style::new()
            .fg(if hot { ink.fail } else { ink.text })
            .bg(ink.raised),
    );
    if let Some(limit) = lane.threshold {
        let mark = if hot { "over" } else { "clear" };
        let phrase = format!("threshold {}  {mark}", numeral(lane.kind, limit));
        let phrase_w = u16::try_from(cols(&phrase)).unwrap_or(0);
        let phrase_x = width.saturating_sub(phrase_w.saturating_add(1));
        let phrase_fg = if hot { ink.fail } else { ink.ok };
        if phrase_x > summary_x {
            cx.put_str(
                phrase_x,
                y,
                &phrase,
                Style::new().fg(phrase_fg).bg(ink.raised),
            );
        }
    }
    let plot_y = y.saturating_add(1);
    if columns > 0 && plot_y < height {
        let spark = Sparkline::new(&samples[..columns]).max(lane.ceiling);
        let spark = if hot {
            spark.style(Style::new().fg(ink.fail))
        } else {
            spark.state(WidgetState::Focused)
        };
        let plot_h = rows.min(height.saturating_sub(plot_y).saturating_sub(1));
        if plot_h > 0 {
            spark.render(Rect::new(geom.spark_x, plot_y, geom.spark_w, plot_h), cx);
        }
    }
    let footer_y = y.saturating_add(rows).saturating_add(1);
    if footer_y < height {
        let since = tick % SELECTION_HOLD;
        let keys = if tick > 0 && since < 8 { "k  " } else { "j k" };
        let place = format!(
            "{keys}  move     {} / {}",
            selected.saturating_add(1),
            board.shown
        );
        cx.put_str(
            2,
            footer_y,
            clip_display_prefix(&place, width.saturating_sub(3)),
            dim_on(ink, ink.surface),
        );
    }
}

fn sparkline_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ink = ink_from(&cx);
    let shape = geom(area.width);
    let board = arrange(area.height);
    draw_spark_chrome(&mut cx, area, &ink, tick, board.header_rows);
    let selected = selected_index(tick, board.shown);
    let scene = SparkScene {
        ink: &ink,
        geom: &shape,
        height: area.height,
        newest: newest_at(tick, shape.spark_w),
        tick,
    };
    for (index, lane) in LANES.iter().enumerate().take(board.shown) {
        let (top, rows) = board.lanes[index];
        if top.saturating_add(rows) > area.height {
            continue;
        }
        draw_lane(&mut cx, &scene, lane, top, rows, index == selected);
    }
    if board.inspector_rows > 0 && board.shown > 0 {
        draw_inspector(&mut cx, &scene, &LANES[selected], &board, selected);
    }
}

fn sparkline_baseline(
    buffer: &mut ratatui::buffer::Buffer,
    area: RatRect,
    theme: &Theme,
    tick: u32,
) {
    let mut samples = [0; 160];
    let columns = usize::from(area.width).min(samples.len());
    fill_wave(&mut samples[..columns], tick.saturating_mul(2));
    let color = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => ratatui::style::Color::Rgb(red, green, blue),
        Color::Indexed(index) => ratatui::style::Color::Indexed(index),
        _ => ratatui::style::Color::Reset,
    };
    ratatui::widgets::Widget::render(
        ratatui::widgets::Sparkline::default()
            .data(samples[..columns].to_vec())
            .style(ratatui::style::Style::default().fg(color)),
        area,
        buffer,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Lunatui,
    Ratatui,
}

struct Options {
    case: String,
    side: Side,
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    profile: Profile,
    variant: Variant,
}

fn main() -> io::Result<()> {
    let options = parse(&env::args().skip(1).collect::<Vec<_>>())?;
    let Some(case) = CASES.iter().find(|case| case.name == options.case) else {
        return Err(invalid(format!("unknown case: {}", options.case)));
    };
    pin_color_output();
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let result = match options.side {
        Side::Lunatui => play_lunatui(&mut out, case, &options),
        Side::Ratatui => play_ratatui(case, &options),
    };
    out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
    result
}

fn play_lunatui(out: &mut impl Write, case: &'static Case, options: &Options) -> io::Result<()> {
    let theme = Theme::builtin(options.variant);
    let area = Rect::new(0, 0, options.width, options.height);
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;
    for tick in 0..options.frames {
        let started = Instant::now();
        match preview(&options.case) {
            Some(preview) => (preview.native)(&mut next, &mut clusters, &theme, area, tick),
            None => (case.lunatui)(&mut next, &mut clusters, &theme, area, tick),
        }
        downsample_grid(&mut next, options.profile);
        out.write_all(encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor))?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        thread::sleep(period.saturating_sub(started.elapsed()));
    }
    Ok(())
}

fn play_ratatui(case: &'static Case, options: &Options) -> io::Result<()> {
    let area = RatRect::new(0, 0, options.width, options.height);
    let theme = Theme::builtin(options.variant);
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )?;
    let period = Duration::from_secs(1) / options.fps;
    for tick in 0..options.frames {
        let started = Instant::now();
        terminal.draw(|frame| {
            let buffer = frame.buffer_mut();
            match preview(&options.case) {
                Some(preview) => (preview.baseline)(buffer, area, &theme, tick),
                None => (case.ratatui)(buffer, area, tick),
            }
            downsample_buffer(buffer, options.profile);
        })?;
        thread::sleep(period.saturating_sub(started.elapsed()));
    }
    Ok(())
}

fn rat_color(color: Color) -> RatColor {
    match color {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    }
}

fn from_rat(color: RatColor) -> Color {
    match color {
        RatColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
        RatColor::Indexed(index) => Color::Indexed(index),
        _ => Color::Default,
    }
}

fn downsample_grid(grid: &mut Grid, profile: Profile) {
    if profile == Profile::TrueColor {
        return;
    }
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let Some(cell) = grid.get(x, y) else {
                continue;
            };
            let style = cell.style();
            let mapped = Style {
                fg: profile.downsample(style.fg),
                bg: profile.downsample(style.bg),
                attrs: style.attrs,
            };
            if mapped != style {
                grid.set_style(Rect::new(x, y, 1, 1), mapped);
            }
        }
    }
}

fn downsample_buffer(buffer: &mut Buffer, profile: Profile) {
    if profile == Profile::TrueColor {
        return;
    }
    for cell in &mut buffer.content {
        cell.fg = rat_color(profile.downsample(from_rat(cell.fg)));
        cell.bg = rat_color(profile.downsample(from_rat(cell.bg)));
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn parse(args: &[String]) -> io::Result<Options> {
    let mut options = Options {
        case: "block".to_owned(),
        side: Side::Lunatui,
        width: 80,
        height: 24,
        frames: 90,
        fps: 20,
        profile: Profile::TrueColor,
        variant: Variant::Dark,
    };
    let mut index = 0;
    while index < args.len() {
        let value = args.get(index + 1);
        let flag = args[index].clone();
        let need = |value: Option<&String>| -> io::Result<String> {
            value
                .cloned()
                .ok_or_else(|| invalid(format!("{flag} needs a value")))
        };
        match args[index].as_str() {
            "--case" => options.case = need(value)?,
            "--side" => {
                options.side = match need(value)?.as_str() {
                    "lunatui" => Side::Lunatui,
                    "ratatui" => Side::Ratatui,
                    other => return Err(invalid(format!("unknown side: {other}"))),
                };
            }
            "--profile" => {
                options.profile = match need(value)?.as_str() {
                    "truecolor" => Profile::TrueColor,
                    "ansi256" => Profile::Ansi256,
                    "ansi16" => Profile::Ansi16,
                    other => return Err(invalid(format!("unknown profile: {other}"))),
                };
            }
            "--theme" => {
                options.variant = match need(value)?.as_str() {
                    "dark" => Variant::Dark,
                    "light" => Variant::Light,
                    other => return Err(invalid(format!("unknown theme: {other}"))),
                };
            }
            "--width" => options.width = positive(&need(value)?, "--width")?,
            "--height" => options.height = positive(&need(value)?, "--height")?,
            "--frames" => options.frames = positive(&need(value)?, "--frames")?,
            "--fps" => options.fps = positive(&need(value)?, "--fps")?,
            flag => return Err(invalid(format!("unknown flag: {flag}"))),
        }
        index += 2;
    }
    Ok(options)
}

fn positive<T: std::str::FromStr + Default + PartialEq>(value: &str, flag: &str) -> io::Result<T> {
    let parsed: T = value
        .parse()
        .map_err(|_| invalid(format!("invalid {flag}")))?;
    if parsed == T::default() {
        return Err(invalid(format!("{flag} must be greater than zero")));
    }
    Ok(parsed)
}
