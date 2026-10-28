// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::widget_suite::{CASES, Case, pin_color_output};
use lunatui_core::{
    Attrs, ClusterTable, Color, Grid, Rect, Style, UnderlineStyle, clip_display_prefix,
    width as cols,
};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::canvas::{Circle, Line};
use lunatui_widgets::{
    Alignment, Axis, Bar, BarChart, BarDirection, BarGroup, BigText, Block, BorderType,
    Breadcrumbs, Canvas, Chart, Dataset, Date, DateStyle, DividerId, GraphType, LegendPosition,
    List, MarkdownView, Marker, Monthly, PaneId, Paragraph, PixelSize, RenderCx, Scrollbar,
    Sparkline, SplitAxis, SplitDrag, SplitTree, StatusBar, ToastKind, ToastScope, ToastStack,
    Widget, WidgetState,
};

#[path = "deploy_board.rs"]
mod deploy_board;
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
        name: "toast",
        native: toast_native,
        baseline: toast_baseline,
    },
    Preview {
        name: "split",
        native: split_native,
        baseline: split_baseline,
    },
    Preview {
        name: "breadcrumbs",
        native: breadcrumbs_native,
        baseline: breadcrumbs_baseline,
    },
    Preview {
        name: "calendar",
        native: calendar_native,
        baseline: calendar_baseline,
    },
    Preview {
        name: "bigtext",
        native: bigtext_native,
        baseline: bigtext_baseline,
    },
    Preview {
        name: "chart",
        native: chart_native,
        baseline: chart_baseline,
    },
    Preview {
        name: "barchart",
        native: barchart_native,
        baseline: barchart_baseline,
    },
    Preview {
        name: "markdown",
        native: markdown_native,
        baseline: markdown_baseline,
    },
    Preview {
        name: "canvas",
        native: canvas_native,
        baseline: canvas_baseline,
    },
    Preview {
        name: "sparkline",
        native: sparkline_native,
        baseline: sparkline_baseline,
    },
    Preview {
        name: "deploy",
        native: deploy_native,
        baseline: deploy_baseline,
    },
];

fn preview(name: &str) -> Option<&'static Preview> {
    PREVIEWS.iter().find(|preview| preview.name == name)
}

const DAY_HOLD: u32 = 11;
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
    let start = Date::from_ymd(2026, 10, 22).unwrap_or(Date::UNIX_EPOCH);
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

fn paint_day_row(cx: &mut RenderCx<'_>, scene: &Scene, x: u16, y: u16, width: u16, date: Date) {
    let picked = date == scene.selected;
    let text = Style::new().fg(scene.ink.text).bg(scene.ink.surface);
    let style = if picked {
        Style::new().fg(scene.ink.focus).bg(scene.ink.raised)
    } else if date == scene.today {
        Style::new().fg(scene.ink.ok).bg(scene.ink.surface)
    } else {
        text
    };
    if picked {
        cx.set_style(
            Rect::new(x, y, width, 1),
            Style::new().fg(scene.ink.text).bg(scene.ink.raised),
        );
    }
    paint(
        cx,
        x,
        y,
        width,
        &week_line(date),
        if picked { bold(style) } else { style },
    );
}

fn draw_week(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect) {
    if area.height < 4 || area.width < 12 {
        return;
    }
    let label = Style::new().fg(scene.ink.accent).bg(scene.ink.surface);
    paint(cx, area.x, area.y, area.width, "This week", bold(label));
    let index = weekday_index(scene.selected);
    let sunday = add_days(scene.selected, -i32::try_from(index).unwrap_or(0));
    let bottom = area.y.saturating_add(area.height);
    let mut row = area.y.saturating_add(1);
    let mut offset = 0_i32;
    let mut labeled_next = false;
    while row < bottom {
        if offset >= 7 && !labeled_next {
            if row.saturating_add(1) >= bottom {
                break;
            }
            paint(cx, area.x, row, area.width, "Next", bold(label));
            labeled_next = true;
            row = row.saturating_add(1);
            continue;
        }
        paint_day_row(cx, scene, area.x, row, area.width, add_days(sunday, offset));
        offset = offset.saturating_add(1);
        row = row.saturating_add(1);
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

fn upcoming_count(selected: Date) -> u16 {
    let mut count = 0_u16;
    for step in 1..22 {
        let date = add_days(selected, step);
        for _entry in ENTRIES.iter().filter(|entry| on_day(entry, date)) {
            count = count.saturating_add(1);
        }
    }
    count
}

fn draw_month_ledger(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, y: u16) {
    let bottom = area.y.saturating_add(area.height);
    if y.saturating_add(1) >= bottom {
        return;
    }
    paint(
        cx,
        area.x,
        y,
        area.width,
        month_name(scene.selected),
        bold(Style::new().fg(scene.ink.accent).bg(scene.ink.surface)),
    );
    let start =
        Date::from_ymd(scene.selected.year(), scene.selected.month(), 1).unwrap_or(scene.selected);
    let mut line = y.saturating_add(1);
    let mut offset = 0_i32;
    while line < bottom {
        paint_day_row(cx, scene, area.x, line, area.width, add_days(start, offset));
        offset = offset.saturating_add(1);
        line = line.saturating_add(1);
    }
}

fn draw_up_next(cx: &mut RenderCx<'_>, scene: &Scene, area: Rect, y: u16) -> u16 {
    if upcoming_count(scene.selected) == 0
        || y.saturating_add(1) >= area.y.saturating_add(area.height)
    {
        return y;
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
    line
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
    let next = draw_up_next(cx, scene, inner, y);
    draw_month_ledger(cx, scene, inner, next);
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

#[derive(Clone, Copy)]
struct CutInk {
    text: Color,
    accent: Color,
    surface: Color,
    raised: Color,
    focus: Color,
    ok: Color,
    warn: Color,
    fail: Color,
}

impl CutInk {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            accent: cx.token(Token::Accent),
            surface: cx.token(Token::Surface0),
            raised: cx.token(Token::Surface1),
            focus: cx.token(Token::FocusRing),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
            fail: cx.token(Token::Fail),
        }
    }
}

struct CutScene {
    ink: CutInk,
    tick: u32,
    width: u16,
    height: u16,
}

impl CutScene {
    fn second(&self) -> u32 {
        (self.tick / 30) % 60
    }

    fn remain(&self) -> u32 {
        6_u32.saturating_sub(self.tick / 30)
    }

    fn live(&self) -> bool {
        self.tick / 30 >= 6
    }

    fn metric(&self) -> usize {
        let tick = self.tick;
        if (72..120).contains(&tick) {
            1
        } else if (120..192).contains(&tick) {
            2
        } else {
            0
        }
    }
}

fn requests(tick: u32) -> u32 {
    1_480 + tick.saturating_mul(2)
}

fn latency(tick: u32) -> u32 {
    60 + (tick / 3) % 30
}

fn error_count(tick: u32) -> u32 {
    if tick < 120 {
        1
    } else if tick < 192 {
        2 + ((tick - 120) / 3).min(12)
    } else {
        14_u32.saturating_sub((tick - 192) / 2).max(3)
    }
}

fn live_count(tick: u32) -> u32 {
    18_420 + tick.saturating_mul(17)
}

fn count_text(tick: u32) -> String {
    let count = live_count(tick);
    format!("{count:06}")
}

fn cut_hint(tick: u32) -> &'static str {
    let into = tick % 48;
    if tick > 0 && into < 8 {
        "l  metric"
    } else {
        "h l metric"
    }
}

fn draw_cut_chrome(cx: &mut RenderCx<'_>, scene: &CutScene) {
    let bar = Style::new().fg(scene.ink.text).bg(scene.ink.raised);
    cx.set_style(Rect::new(0, 0, scene.width, 1), bar);
    paint(
        cx,
        1,
        0,
        6,
        " live ",
        bold(Style::new().fg(scene.ink.surface).bg(scene.ink.ok)),
    );
    paint(cx, 8, 0, 10, "cutover", bold(bar));
    let clock = format!("14:02:{:02}", scene.second());
    let clock_cols = u16::try_from(clock.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(clock_cols.saturating_add(1)),
        0,
        clock_cols,
        &clock,
        Style::new().fg(scene.ink.accent).bg(scene.ink.raised),
    );
    paint(
        cx,
        1,
        1,
        32,
        "prod     preview window",
        Style::new().fg(scene.ink.text).bg(scene.ink.surface),
    );
    let footer_y = scene.height.saturating_sub(1);
    cx.set_style(Rect::new(0, footer_y, scene.width, 1), bar);
    let hint = cut_hint(scene.tick);
    let hint_color = if hint.starts_with('l') {
        scene.ink.focus
    } else {
        scene.ink.text
    };
    paint(
        cx,
        1,
        footer_y,
        16,
        hint,
        Style::new().fg(hint_color).bg(scene.ink.raised),
    );
    let stamp = if scene.live() { "live" } else { "waiting" };
    let stamp_cols = u16::try_from(stamp.len()).unwrap_or(0);
    let stamp_color = if scene.live() {
        scene.ink.ok
    } else {
        scene.ink.warn
    };
    paint(
        cx,
        scene.width.saturating_sub(stamp_cols.saturating_add(1)),
        footer_y,
        stamp_cols,
        stamp,
        Style::new().fg(stamp_color).bg(scene.ink.raised),
    );
}

fn hero_size(width: u16) -> PixelSize {
    if width >= 100 {
        PixelSize::HalfHeight
    } else {
        PixelSize::Quadrant
    }
}

fn draw_clock(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let clock = format!("14:02:{:02}", scene.second());
    let size = hero_size(scene.width);
    let (_width, height) = size.cells();
    BigText::new(&clock)
        .pixel_size(size)
        .alignment(Alignment::Center)
        .state(WidgetState::Focused)
        .render(Rect::new(0, y, scene.width, height), cx);
}

fn draw_status(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let word = if scene.live() { "LIVE" } else { "WAIT" };
    let color = if scene.live() {
        scene.ink.ok
    } else {
        scene.ink.warn
    };
    let size = hero_size(scene.width);
    let (_width, height) = size.cells();
    BigText::new(word)
        .pixel_size(size)
        .alignment(Alignment::Center)
        .style(Style::new().fg(color))
        .render(Rect::new(0, y, scene.width, height), cx);
}

fn draw_countdown(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let left = format!("00:00:{:02}", scene.remain());
    let color = if scene.live() {
        scene.ink.ok
    } else {
        scene.ink.warn
    };
    let size = hero_size(scene.width);
    let (_width, height) = size.cells();
    BigText::new(&left)
        .pixel_size(size)
        .alignment(Alignment::Center)
        .style(Style::new().fg(color))
        .render(Rect::new(0, y, scene.width, height), cx);
}

fn draw_metric(
    cx: &mut RenderCx<'_>,
    scene: &CutScene,
    area: Rect,
    label: &str,
    value: &str,
    hot: bool,
    picked: bool,
) {
    let (_width, glyph_h) = PixelSize::HalfHeight.cells();
    if area.width < 8 || area.height < glyph_h.saturating_add(1) {
        return;
    }
    let label_style = if picked {
        bold(Style::new().fg(scene.ink.focus).bg(scene.ink.raised))
    } else {
        Style::new().fg(scene.ink.text).bg(scene.ink.surface)
    };
    if picked {
        cx.set_style(area, Style::new().fg(scene.ink.text).bg(scene.ink.raised));
    }
    let label_cols = u16::try_from(label.len()).unwrap_or(0);
    let label_x = area.x + area.width.saturating_sub(label_cols) / 2;
    paint(cx, label_x, area.y, label_cols, label, label_style);
    let mut text = BigText::new(value)
        .pixel_size(PixelSize::HalfHeight)
        .alignment(Alignment::Center);
    text = if hot {
        text.style(Style::new().fg(scene.ink.fail))
    } else if picked {
        text.state(WidgetState::Focused)
    } else {
        text.state(WidgetState::Normal)
    };
    text.render(
        Rect::new(area.x, area.y.saturating_add(1), area.width, glyph_h),
        cx,
    );
}

fn draw_metrics(cx: &mut RenderCx<'_>, scene: &CutScene, y: u16) {
    let gap = 2_u16;
    let col_w = scene
        .width
        .saturating_sub(4)
        .saturating_sub(gap.saturating_mul(2))
        / 3;
    let (glyph_w, _) = PixelSize::HalfHeight.cells();
    if col_w < glyph_w.saturating_mul(4) {
        return;
    }
    let values = [
        ("req/s", format!("{:04}", requests(scene.tick)), false),
        ("p99 ms", format!("{:03}", latency(scene.tick)), false),
        (
            "errors",
            format!("{:03}", error_count(scene.tick)),
            error_count(scene.tick) >= 8,
        ),
    ];
    let picked = scene.metric();
    for (index, (label, value, hot)) in values.iter().enumerate() {
        let x = 2 + u16::try_from(index).unwrap_or(0) * (col_w + gap);
        let (_width, glyph_h) = PixelSize::HalfHeight.cells();
        draw_metric(
            cx,
            scene,
            Rect::new(x, y, col_w, glyph_h.saturating_add(1)),
            label,
            value,
            *hot,
            index == picked,
        );
    }
}

fn readout(scene: &CutScene) -> String {
    match scene.metric() {
        0 => format!("requests  {} /s", requests(scene.tick)),
        1 => format!("p99  {} ms", latency(scene.tick)),
        _ => format!("errors  {}", error_count(scene.tick)),
    }
}

fn draw_cut_board(cx: &mut RenderCx<'_>, scene: &CutScene) {
    let clock_y = 2;
    draw_clock(cx, scene, clock_y);
    let (_width, clock_h) = hero_size(scene.width).cells();
    let caption_y = clock_y.saturating_add(clock_h);
    let caption = "local time";
    let caption_cols = u16::try_from(caption.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(caption_cols) / 2,
        caption_y,
        caption_cols,
        caption,
        Style::new().fg(scene.ink.text).bg(scene.ink.surface),
    );
    let status_y = caption_y.saturating_add(2);
    draw_status(cx, scene, status_y);
    let (_width, status_h) = hero_size(scene.width).cells();
    let label_y = status_y.saturating_add(status_h);
    let label = "cutover";
    let label_cols = u16::try_from(label.len()).unwrap_or(0);
    paint(
        cx,
        scene.width.saturating_sub(label_cols) / 2,
        label_y,
        label_cols,
        label,
        Style::new().fg(scene.ink.accent).bg(scene.ink.surface),
    );
    let count_y = label_y.saturating_add(1);
    draw_countdown(cx, scene, count_y);
    let (_width, count_h) = hero_size(scene.width).cells();
    let metrics_y = count_y.saturating_add(count_h).saturating_add(1);
    draw_metrics(cx, scene, metrics_y);
    let (_width, metric_h) = PixelSize::HalfHeight.cells();
    let note = readout(scene);
    paint(
        cx,
        2,
        metrics_y.saturating_add(metric_h).saturating_add(1),
        scene.width.saturating_sub(4),
        &note,
        Style::new().fg(scene.ink.focus).bg(scene.ink.surface),
    );
}

fn draw_compact(cx: &mut RenderCx<'_>, scene: &CutScene) {
    let clock_y = 2;
    draw_clock(cx, scene, clock_y);
    let (_width, clock_h) = hero_size(scene.width).cells();
    let count_y = clock_y.saturating_add(clock_h).saturating_add(1);
    let (_width, count_h) = hero_size(scene.width).cells();
    if count_y.saturating_add(count_h) < scene.height.saturating_sub(1) {
        draw_countdown(cx, scene, count_y);
    }
}

fn bigtext_native(
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
    let scene = CutScene {
        ink: CutInk::new(&cx),
        tick,
        width: area.width,
        height: area.height,
    };
    draw_cut_chrome(&mut cx, &scene);
    if scene.width < 100 || scene.height < 34 {
        draw_compact(&mut cx, &scene);
        return;
    }
    draw_cut_board(&mut cx, &scene);
}

fn bigtext_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let count = count_text(tick);
    let text = BigText::new(&count).pixel_size(PixelSize::Quadrant);
    let color = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    };
    let style = ratatui::style::Style::default().fg(color);
    let (_width, height) = PixelSize::Quadrant.cells();
    for row in 0..height.min(area.height) {
        let mut glyphs = [' '; 64];
        let columns = text.row_glyphs(row, &mut glyphs);
        let y = area.y.saturating_add(row);
        let limit = columns.min(glyphs.len()).min(usize::from(area.width));
        for (col, glyph) in glyphs.iter().take(limit).enumerate() {
            let x = area.x.saturating_add(u16::try_from(col).unwrap_or(0));
            if x >= area.x.saturating_add(area.width) {
                break;
            }
            buffer[(x, y)].set_char(*glyph).set_style(style);
        }
    }
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

fn deploy_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    _area: Rect,
    tick: u32,
) {
    deploy_board::draw(grid, clusters, theme, u64::from(tick).saturating_mul(100));
}

fn deploy_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let accent = match theme.resolve(Token::Accent) {
        Color::Rgb(red, green, blue) => ratatui::style::Color::Rgb(red, green, blue),
        Color::Indexed(index) => ratatui::style::Color::Indexed(index),
        _ => ratatui::style::Color::Reset,
    };
    let block = ratatui::widgets::Block::default()
        .title(" production deploy ")
        .borders(ratatui::widgets::Borders::ALL);
    let inner = block.inner(area);
    ratatui::widgets::Widget::render(block, area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    ratatui::widgets::Widget::render(
        ratatui::widgets::Paragraph::new("release 2.4.1    us-west-3    rolling"),
        RatRect::new(inner.x, inner.y, inner.width, 1),
        buffer,
    );
    let ratio = f64::from(tick % 80) / 80.0;
    let bar_y = inner.y.saturating_add(inner.height.saturating_sub(3));
    let bar_h = inner.height.min(3);
    ratatui::widgets::Widget::render(
        ratatui::widgets::Gauge::default()
            .ratio(ratio)
            .label("rollout")
            .gauge_style(ratatui::style::Style::default().fg(accent)),
        RatRect::new(inner.x, bar_y, inner.width, bar_h),
        buffer,
    );
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

const MD_PAGES: [(&str, &str); 3] = [
    (
        "Getting started",
        "# Getting started\n\nLunatui draws terminal interfaces from small **widgets**. Each widget takes an area and a `RenderCx` and writes cells; nothing else is needed to get a frame on screen.\n\n## Install\n\n1. Add the crate with the features you use.\n2. Create a `Grid` and a `ClusterTable`.\n3. Render a widget into the grid.\n4. Diff the grid and write the changes.\n\n```toml\n[dependencies]\nlunatui = \"0.1\"\nlunatui-widgets = { version = \"0.1\", features = [\"markdown\"] }\n```\n\n## Your first frame\n\n> Every widget redraws the whole of its own area, so a frame never shows leftovers from the one before it.\n\n- [x] Block and Paragraph\n- [x] List and Table\n- [ ] Your own widget\n  - implement `Widget`\n  - keep `render` free of side effects\n\n---\n\nRead the *architecture overview* next, or jump to the [widget gallery](https://example.com/gallery).\n",
    ),
    (
        "Layout",
        "# Layout\n\nA layout splits an area into rectangles from a list of **constraints**. The solver is exact for the common cases and falls back to a general solver only when it must.\n\n## Constraints\n\n| constraint | meaning |\n|:--|--:|\n| `Length(n)` | exactly n cells |\n| `Min(n)` | at least n cells |\n| `Percentage(p)` | p percent of the axis |\n| `Fill(w)` | a weighted share of the rest |\n\n## Flex\n\n- `Start`, `Center`, and `End` place the group\n- `SpaceBetween` spreads the gaps\n  - a lone segment sits at the start\n- `Legacy` keeps the older rules\n\n```rust\nlet [left, right] = Layout::horizontal([Length(24), Fill(1)])\n    .split(area)\n    .into_array();\n```\n\n> Floors and caps are opt in: turn on the `size-limits` feature to bound a segment.\n",
    ),
    (
        "Widgets",
        "# Widgets\n\nThe widget set covers text, lists, tables, gauges, charts, and now **Markdown**.\n\n## Markdown\n\nThe view lays a parsed document out at the width it is given: headings get rules, lists keep a hanging indent, quotes keep their bar, and tables are boxed. ~~Raw source~~ is never shown.\n\n1. Parse once with `markdown::parse`.\n2. Render with `MarkdownView::new(&document)`.\n3. Scroll with `.scroll(rows)` and size a scrollbar from `content_height`.\n\n### Inline styles\n\nText can be *emphasised*, **strong**, ***both***, ~~struck~~, or `code`, and a [link](https://example.com) is underlined.\n\n| widget | feature |\n|:--|:-:|\n| `Sparkline` | sparkline |\n| `BarChart` | barchart |\n| `Chart` | chart |\n| `MarkdownView` | markdown |\n\n- [x] headings, lists, quotes\n- [x] code blocks and tables\n- [x] task items\n\n---\n\nThat is the whole tour.\n",
    ),
];

fn md_documents() -> &'static [lunatui_text::markdown::Document] {
    static DOCUMENTS: std::sync::OnceLock<Vec<lunatui_text::markdown::Document>> =
        std::sync::OnceLock::new();
    DOCUMENTS.get_or_init(|| {
        MD_PAGES
            .iter()
            .map(|(_, source)| {
                lunatui_text::markdown::parse(source, lunatui_text::markdown::Options::gfm())
            })
            .collect()
    })
}

fn markdown_native(
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
    let raised = cx.token(Token::Surface1);
    let accent = cx.token(Token::Accent);
    let fill = Style::new().fg(text).bg(surface);
    let page = usize::try_from(tick / 70).unwrap_or(0) % MD_PAGES.len();
    let (title, _) = MD_PAGES[page];
    let document = &md_documents()[page];
    StatusBar::new(format!(" Docs  ›  {title}"))
        .style(
            Style::new()
                .fg(accent)
                .bg(raised)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let body = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(2),
    );
    let side_w = 26.min(body.width / 3);
    let side = Rect::new(body.x, body.y, side_w, body.height);
    let panel = |title: &str| {
        Block::bordered()
            .title(format!(" {title} "))
            .style(fill)
            .border_style(Style::new().fg(raised).bg(surface))
            .title_style(fill)
    };
    let side_block = panel("Pages");
    side_block.render(side, &mut cx);
    let names: Vec<Vec<String>> = MD_PAGES
        .iter()
        .map(|(name, _)| vec![(*name).to_owned()])
        .collect();
    let source = lunatui_widgets::VecSource::from_rows(names);
    List::new(source)
        .style(fill)
        .highlight_style(
            Style::new()
                .fg(accent)
                .bg(raised)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .selected(Some(page))
        .render(side_block.inner(side), &mut cx);
    let main = Rect::new(body.x + side_w, body.y, body.width - side_w, body.height);
    let main_block = panel("Reader");
    main_block.render(main, &mut cx);
    let inner = main_block.inner(main);
    let view = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(3),
        inner.height,
    );
    let content = MarkdownView::new(document).content_height(view.width);
    let travel = content.saturating_sub(usize::from(view.height));
    let phase = usize::try_from(tick % 70)
        .unwrap_or(0)
        .saturating_sub(8)
        .min(54);
    let scroll = travel * phase / 54;
    MarkdownView::new(document)
        .background(fill)
        .scroll(scroll)
        .render(view, &mut cx);
    Scrollbar::vertical(content, usize::from(view.height), scroll)
        .track_style(Style::new().fg(raised).bg(surface))
        .thumb_style(Style::new().fg(accent).bg(surface))
        .render(
            Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height),
            &mut cx,
        );
    let percent = (scroll * 100).checked_div(travel).unwrap_or(100);
    StatusBar::new(format!(" tab pages   ↑↓ scroll   q quit   {percent}%"))
        .style(
            Style::new()
                .fg(text)
                .bg(raised)
                .with_attrs(Attrs::new().set_dim(true)),
        )
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
}

fn markdown_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "markdown") {
        (case.ratatui)(buffer, area, tick);
    }
}

const TOAST_TICK: Duration = Duration::from_millis(50);
const TOAST_SCRIPT: [(u32, u32, ToastScope, ToastKind, &str); 7] = [
    (
        20,
        70,
        ToastScope::Pane(1),
        ToastKind::Success,
        "api 4.12 deployed",
    ),
    (
        45,
        80,
        ToastScope::Pane(1),
        ToastKind::Info,
        "web: 3 checks queued",
    ),
    (
        70,
        90,
        ToastScope::Pane(2),
        ToastKind::Warning,
        "Log stream is lagging",
    ),
    (
        95,
        60,
        ToastScope::Status,
        ToastKind::Info,
        "Syncing with origin",
    ),
    (
        110,
        100,
        ToastScope::Pane(1),
        ToastKind::Error,
        "jobs: migration failed",
    ),
    (
        130,
        50,
        ToastScope::Status,
        ToastKind::Success,
        "Synced 14 files",
    ),
    (
        150,
        70,
        ToastScope::Pane(2),
        ToastKind::Info,
        "Tailing build 4821",
    ),
];

fn toast_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    let raised = theme.resolve(Token::Surface1);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let left = Rect::new(body.x, body.y, body.width / 2, body.height);
    let right = Rect::new(
        left.x + left.width,
        body.y,
        body.width - left.width,
        body.height,
    );
    for (title, pane) in [(" Pipelines ", left), (" Activity ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    let pipelines = [
        "api", "web", "jobs", "edge", "docs", "search", "billing", "auth",
    ];
    for (row, name) in (0_u16..).zip(pipelines) {
        let y = left.y + 1 + row;
        if y + 1 >= left.y + left.height {
            break;
        }
        let state = if name == "jobs" && tick >= 110 {
            "failed "
        } else {
            "passing"
        };
        let line = format!(
            " {name:<10}{state}  #{}",
            4800 + u32::from(row) * 3 + tick / 40
        );
        cx.put_str(
            left.x + 1,
            y,
            clip_display_prefix(&line, left.width.saturating_sub(2)),
            fill,
        );
    }
    for row in 0..right.height.saturating_sub(2) {
        let second = tick / 4 + u32::from(row);
        let line = format!(
            " 12:{:02}:{:02}  worker-{}  ok",
            (second / 60) % 60,
            second % 60,
            row % 5 + 1
        );
        cx.put_str(
            right.x + 1,
            right.y + 1 + row,
            clip_display_prefix(&line, right.width.saturating_sub(2)),
            dim,
        );
    }
    StatusBar::new(" main  ●  3 workers  ●  q quit")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(area.x, area.y + body.height, area.width, 1),
            &mut cx,
        );
    let now = TOAST_TICK * tick;
    let mut stack = ToastStack::new();
    for (start, ttl, scope, kind, message) in TOAST_SCRIPT {
        if tick >= start {
            stack.push(
                TOAST_TICK * start,
                Some(TOAST_TICK * ttl),
                scope,
                kind,
                message,
            );
        }
    }
    let panes = |id: u32| match id {
        1 => Some(left),
        2 => Some(right),
        _ => None,
    };
    stack.render(now, area, panes, &mut cx);
}

fn toast_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "toast") {
        (case.ratatui)(buffer, area, tick);
    }
}

struct SplitMove {
    from: u32,
    to: u32,
    divider: usize,
    target: (i32, i32),
}

const SPLIT_MOVES: [SplitMove; 4] = [
    SplitMove {
        from: 10,
        to: 50,
        divider: 0,
        target: (42, 0),
    },
    SplitMove {
        from: 60,
        to: 100,
        divider: 1,
        target: (0, 19),
    },
    SplitMove {
        from: 110,
        to: 145,
        divider: 2,
        target: (84, 0),
    },
    SplitMove {
        from: 205,
        to: 235,
        divider: 0,
        target: (28, 0),
    },
];

fn lerp(start: i32, end: i32, spent: u32, span: u32) -> i32 {
    let delta = i64::from(end) - i64::from(start);
    let step = delta * i64::from(spent) / i64::from(span.max(1));
    i32::try_from(i64::from(start) + step).unwrap_or(start)
}

fn split_scene(area: Rect, tick: u32) -> (SplitTree, SplitDrag) {
    let (mut tree, files) = SplitTree::new();
    let editor = tree
        .split(files, SplitAxis::Horizontal, 250)
        .unwrap_or(files);
    tree.split(editor, SplitAxis::Vertical, 700);
    tree.split(editor, SplitAxis::Horizontal, 720);
    for divider in 0..3 {
        tree.set_min(DividerId(divider), 12, 12);
    }
    let mut drag = SplitDrag::new();
    let mut start = (0_i32, 0_i32);
    for now in 0..=tick {
        for step in &SPLIT_MOVES {
            if now == step.from {
                let layout = tree.layout(area);
                if let Some(line) = layout.dividers.get(step.divider) {
                    let (x, y) = (line.rect.x, line.rect.y + line.rect.height / 2);
                    start = (i32::from(x), i32::from(y));
                    drag.press(&tree, &layout, x, y);
                }
            } else if now > step.from && now <= step.to {
                let span = step.to - step.from;
                let spent = now - step.from;
                let column = u16::try_from(lerp(start.0, step.target.0, spent, span)).unwrap_or(0);
                let row = u16::try_from(lerp(start.1, step.target.1, spent, span)).unwrap_or(0);
                drag.drag(&mut tree, column, row);
                if now == step.to {
                    drag.release();
                }
            }
        }
    }
    (tree, drag)
}

fn split_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let (mut tree, drag) = split_scene(body, tick);
    if (150..195).contains(&tick) {
        tree.zoom(Some(PaneId(1)));
    }
    let layout = tree.layout(body);
    let titles = ["Files", "Editor", "Terminal", "Inspector"];
    let content: [&[&str]; 4] = [
        &[
            "src/",
            "  block.rs",
            "  split.rs",
            "  tabs.rs",
            "tests/",
            "Cargo.toml",
            "README.md",
        ],
        &[
            "fn layout(&self, area: Rect) -> SplitLayout {",
            "    let mut layout = SplitLayout::default();",
            "    place(&self.root, area, &mut 0, &mut layout);",
            "    layout",
            "}",
        ],
        &[
            "$ cargo test -p lunatui-widgets",
            "   Compiling lunatui-widgets",
            "test result: ok. 29 passed",
        ],
        &["panes     4", "dividers  3", "zoomed    none"],
    ];
    for area in &layout.panes {
        let index = usize::try_from(area.pane.0).unwrap_or(0).min(3);
        let title = if tree.zoomed().is_some() {
            format!(" {} (zoomed) ", titles[index])
        } else {
            format!(" {} ", titles[index])
        };
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(area.rect, &mut cx);
        let room = area.rect.width.saturating_sub(4);
        for (row, line) in (0_u16..).zip(content[index]) {
            if row + 2 >= area.rect.height {
                break;
            }
            cx.put_str(
                area.rect.x + 2,
                area.rect.y + 1 + row,
                clip_display_prefix(line, room),
                fill,
            );
        }
    }
    layout.render_dividers(
        dim,
        drag.active(),
        Style::new().fg(accent).bg(surface),
        &mut cx,
    );
    StatusBar::new(" drag a divider to resize  ●  zoom with z")
        .style(Style::new().fg(text).bg(raised))
        .render(
            Rect::new(area.x, area.y + body.height, area.width, 1),
            &mut cx,
        );
}

fn split_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "split") {
        (case.ratatui)(buffer, area, tick);
    }
}

const CRUMB_PATH: [&str; 6] = [
    "Home",
    "Projects",
    "Moonbase2090",
    "lunatui",
    "lunatui-widgets",
    "src",
];

type CrumbEntry = (&'static str, u32, &'static str);

const CRUMB_DIRS: [&[CrumbEntry]; 6] = [
    &[
        ("Documents/", 0, "Oct 12"),
        ("Downloads/", 0, "Oct 27"),
        ("Pictures/", 0, "Sep 30"),
        ("Projects/", 0, "Oct 28"),
        ("notes.txt", 2_400, "Oct 21"),
        ("todo.md", 1_100, "Oct 26"),
    ],
    &[
        ("Moonbase2090/", 0, "Oct 28"),
        ("Scratch/", 0, "Oct 03"),
        ("archive/", 0, "Aug 19"),
        ("README.md", 3_900, "Oct 14"),
    ],
    &[
        ("lunatui/", 0, "Oct 28"),
        ("prismattyc/", 0, "Oct 27"),
        ("rookrunner/", 0, "Oct 28"),
        ("scorecard/", 0, "Oct 25"),
        ("waypoint/", 0, "Oct 26"),
        ("NOTES.md", 8_200, "Oct 24"),
    ],
    &[
        ("benches/", 0, "Oct 27"),
        ("examples/", 0, "Oct 28"),
        ("lunatui-core/", 0, "Oct 26"),
        ("lunatui-text/", 0, "Oct 26"),
        ("lunatui-theme/", 0, "Oct 20"),
        ("lunatui-widgets/", 0, "Oct 28"),
        ("CHANGELOG.md", 21_700, "Oct 28"),
        ("Cargo.toml", 1_900, "Oct 27"),
        ("README.md", 15_300, "Oct 28"),
    ],
    &[
        ("src/", 0, "Oct 28"),
        ("tests/", 0, "Oct 28"),
        ("Cargo.toml", 4_100, "Oct 28"),
        ("README.md", 6_800, "Oct 28"),
    ],
    &[
        ("block.rs", 18_400, "Oct 24"),
        ("borders.rs", 4_900, "Oct 20"),
        ("breadcrumbs.rs", 8_600, "Oct 28"),
        ("gauge.rs", 12_100, "Oct 26"),
        ("lib.rs", 5_300, "Oct 28"),
        ("list.rs", 3_200, "Oct 26"),
        ("table.rs", 21_900, "Oct 27"),
        ("tabs.rs", 9_700, "Oct 28"),
    ],
];

fn crumb_size(bytes: u32) -> String {
    if bytes == 0 {
        String::from("--")
    } else if bytes < 1_000 {
        format!("{bytes} B")
    } else {
        format!("{}.{} KB", bytes / 1_000, bytes % 1_000 / 100)
    }
}

fn crumb_preview(name: &str, bytes: u32, level: usize) -> Vec<String> {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str());
    let kind = match extension {
        _ if name.ends_with('/') => "Folder",
        Some("rs") => "Rust source",
        Some("toml") => "TOML",
        Some("md") => "Markdown",
        _ => "Text",
    };
    let mut lines = vec![
        name.trim_end_matches('/').to_owned(),
        format!("{kind}  ·  {}", crumb_size(bytes)),
        String::new(),
    ];
    if name.ends_with('/') {
        let inside = CRUMB_DIRS.get(level + 1).map_or(0, |dir| dir.len());
        lines.push(format!("{inside} items"));
        lines.push(String::from("Press Enter to open"));
    } else if extension == Some("rs") {
        for line in [
            "// Copyright (c) 2026 Moonbase 2090 LLC",
            "// SPDX-License-Identifier: MIT",
            "",
            "use lunatui_core::{Rect, Size, Style};",
            "",
            "use crate::{RenderCx, Widget};",
            "",
            "#[derive(Clone, Debug, PartialEq, Eq)]",
        ] {
            lines.push(line.to_owned());
        }
    } else if extension == Some("toml") {
        for line in [
            "[package]",
            "name = \"lunatui-widgets\"",
            "version.workspace = true",
            "",
            "[features]",
            "default = []",
            "widget-state = []",
        ] {
            lines.push(line.to_owned());
        }
    } else {
        for line in [
            "# Notes",
            "",
            "- check the release PRs",
            "- review the showcase media",
            "- retarget the stacked branches",
        ] {
            lines.push(line.to_owned());
        }
    }
    lines
}

fn crumb_panes(
    cx: &mut RenderCx<'_>,
    area: Rect,
    level: usize,
    tick: u32,
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let [fill, dim] = styles;
    let [accent, raised] = colors;
    let (text, surface) = (fill.fg, fill.bg);
    let entries = CRUMB_DIRS[level];
    let body_top = area.y;
    let body_height = area.height.min(18);
    let list = Rect::new(area.x + 2, body_top, 72, body_height);
    let side = Rect::new(
        list.x + list.width + 1,
        body_top,
        area.width.saturating_sub(list.width + 5),
        body_height,
    );
    let selected = usize::try_from(tick / 14).unwrap_or(0) % entries.len();
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", CRUMB_PATH[level]))
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(list, cx);
    let columns = [list.x + 2, list.x + 44, list.x + 58];
    cx.put_str(columns[0], list.y + 1, "Name", dim);
    cx.put_str(columns[1], list.y + 1, "Size", dim);
    cx.put_str(columns[2], list.y + 1, "Modified", dim);
    for (row, (name, bytes, modified)) in (0_u16..).zip(entries.iter()) {
        let y = list.y + 3 + row;
        if y + 1 >= list.y + list.height {
            break;
        }
        let picked = usize::from(row) == selected;
        let style = if picked {
            Style::new().fg(text).bg(raised)
        } else if name.ends_with('/') {
            Style::new().fg(accent).bg(surface)
        } else {
            fill
        };
        cx.put_str(
            list.x + 1,
            y,
            &" ".repeat(usize::from(list.width.saturating_sub(2))),
            style,
        );
        cx.put_str(columns[0], y, name, style);
        cx.put_str(columns[1], y, &crumb_size(*bytes), style);
        cx.put_str(columns[2], y, modified, style);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Preview ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(side, cx);
    let (name, bytes, _) = entries[selected];
    for (row, line) in (0_u16..).zip(crumb_preview(name, bytes, level)) {
        let y = side.y + 1 + row;
        if y + 1 >= side.y + side.height {
            break;
        }
        let style = if row == 0 {
            fill.fg(accent)
        } else if row == 1 {
            dim
        } else {
            fill
        };
        cx.put_str(
            side.x + 2,
            y,
            clip_display_prefix(&line, side.width.saturating_sub(4)),
            style,
        );
    }
    let recent = Rect::new(
        area.x + 2,
        body_top + body_height,
        area.width.saturating_sub(4),
        area.height.saturating_sub(body_height),
    );
    crumb_visited(cx, recent, level, styles, colors);
}

fn crumb_visited(
    cx: &mut RenderCx<'_>,
    recent: Rect,
    level: usize,
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let [fill, dim] = styles;
    let [accent, _] = colors;
    let surface = fill.bg;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Visited ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(surface))
        .render(recent, cx);
    for (row, depth) in (0_u16..).zip(1..=level + 1) {
        let y = recent.y + 1 + row;
        if y + 1 >= recent.y + recent.height {
            break;
        }
        let path = CRUMB_PATH[..depth].join("/");
        let count = CRUMB_DIRS[depth - 1].len();
        let here = depth == level + 1;
        let marker = if here { "▶" } else { " " };
        let line = format!(" {marker} {path:<52}{count:>3} items");
        let style = if here { fill.fg(accent) } else { dim };
        cx.put_str(
            recent.x + 1,
            y,
            clip_display_prefix(&line, recent.width.saturating_sub(2)),
            style,
        );
    }
}

fn breadcrumbs_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let surface = theme.resolve(Token::Surface0);
    let raised = theme.resolve(Token::Surface1);
    let text = theme.resolve(Token::TextPrimary);
    let accent = theme.resolve(Token::Accent);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let fill = Style::new().fg(text).bg(surface);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let header = Style::new().fg(text).bg(raised);

    let depth = if tick < 190 {
        (1 + usize::try_from(tick / 28).unwrap_or(0)).min(CRUMB_PATH.len())
    } else {
        2
    };
    let span = area.width.saturating_sub(4);
    let squeeze = if (70..150).contains(&tick) {
        tri(tick - 70, 80, u64::from(span), 44)
    } else {
        u64::from(span)
    };
    let wide = u16::try_from(squeeze).unwrap_or(span).min(span);
    let bar = Rect::new(area.x + 2, area.y + 1, wide, 1);
    let mut crumbs = Breadcrumbs::new(CRUMB_PATH[..depth].iter().copied())
        .style(header)
        .current_style(header.fg(accent).with_attrs(Attrs::new().set_bold(true)))
        .separator_style(header.with_attrs(Attrs::new().set_dim(true)));
    let areas = crumbs.segment_areas(bar);
    let hover = if (150..190).contains(&tick) {
        areas
            .iter()
            .map(|(index, _)| *index)
            .find(|index| *index == 1)
    } else if tick < 150 && tick % 40 >= 24 && !areas.is_empty() {
        let pick = usize::try_from(tick / 40).unwrap_or(0) % areas.len();
        areas.get(pick).map(|(index, _)| *index)
    } else {
        None
    };
    crumbs = crumbs.hover(
        hover,
        Style::new().with_attrs(Attrs::new().set_underline(UnderlineStyle::Single)),
    );
    cx.put_str(
        area.x,
        area.y + 1,
        &" ".repeat(usize::from(area.width)),
        header,
    );
    crumbs.render(bar, &mut cx);
    cx.put_str(area.x + 1, area.y, " Files", fill.fg(accent));

    let level = depth - 1;
    let entries = CRUMB_DIRS[level];
    let panes = Rect::new(
        area.x,
        area.y + 3,
        area.width,
        area.height.saturating_sub(5),
    );
    crumb_panes(&mut cx, panes, level, tick, [fill, dim], [accent, raised]);
    StatusBar::new(format!(
        " {} items  ●  {}  ●  click a path segment to jump back",
        entries.len(),
        CRUMB_PATH[..depth].join("/")
    ))
    .style(Style::new().fg(text).bg(raised))
    .render(
        Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        ),
        &mut cx,
    );
}

fn breadcrumbs_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "breadcrumbs") {
        (case.ratatui)(buffer, area, tick);
    }
}

const BAR_SERVICES: [&str; 4] = ["api", "web", "jobs", "edge"];
const BAR_PERCENTILES: [&str; 3] = ["p50", "p95", "p99"];
const BAR_REGIONS: [(&str, u32); 6] = [
    ("us-east", 41),
    ("us-west", 57),
    ("eu-west", 73),
    ("eu-north", 88),
    ("ap-south", 119),
    ("ap-east", 96),
];

fn bar_requests(tick: u32, service: u32, tier: u32) -> u64 {
    let base = [180, 120, 70][usize::try_from(tier).unwrap_or(0)] + service * 14;
    let swing = tri(tick + service * 9 + tier * 5, 48, 0, 60);
    u64::from(base) + swing
}

fn bar_latency(tick: u32, region: u32, base: u32) -> u64 {
    u64::from(base) + tri(tick + region * 7, 60, 0, 34)
}

struct BarBoard {
    text: Color,
    surface: Color,
    edge: Color,
    ok: Color,
    accent: Color,
    warn: Color,
    focus: Color,
    tick: u32,
}

impl BarBoard {
    fn new(cx: &RenderCx<'_>, tick: u32) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            edge: cx.token(Token::Surface1),
            ok: cx.token(Token::Ok),
            accent: cx.token(Token::Accent),
            warn: cx.token(Token::Warn),
            focus: cx.token(Token::FocusRing),
            tick,
        }
    }

    fn panel(&self, title: &str, area: Rect, cx: &mut RenderCx<'_>) -> Rect {
        let panel = Block::bordered()
            .title(format!(" {title} "))
            .style(Style::new().fg(self.text).bg(self.surface))
            .border_style(Style::new().fg(self.edge).bg(self.surface))
            .title_style(Style::new().fg(self.text).bg(self.surface));
        panel.render(area, cx);
        let inner = panel.inner(area);
        Rect::new(
            inner.x + 1,
            inner.y + 1,
            inner.width.saturating_sub(2),
            inner.height.saturating_sub(1),
        )
    }

    fn label_style(&self) -> Style {
        Style::new().fg(self.text)
    }

    fn fill(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    fn requests(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Requests per minute", area, cx);
        let tiers = [self.ok, self.accent, self.warn].map(|color| Style::new().fg(color));
        let texts: Vec<Vec<String>> = (0..4_u32)
            .map(|service| {
                (0..3_u32)
                    .map(|tier| bar_requests(self.tick, service, tier).to_string())
                    .collect()
            })
            .collect();
        let bars: Vec<Vec<Bar<'_>>> = (0..4_u32)
            .map(|service| {
                (0..3_u32)
                    .map(|tier| {
                        let value = bar_requests(self.tick, service, tier);
                        let (service, tier) = (
                            usize::try_from(service).unwrap_or(0),
                            usize::try_from(tier).unwrap_or(0),
                        );
                        Bar::new(value)
                            .label(BAR_PERCENTILES[tier])
                            .text_value(&texts[service][tier])
                            .style(tiers[tier])
                    })
                    .collect()
            })
            .collect();
        let groups: Vec<BarGroup<'_>> = bars
            .iter()
            .zip(BAR_SERVICES)
            .map(|(group, name)| BarGroup::new(group).label(name))
            .collect();
        BarChart::from_groups(&groups)
            .background(self.fill())
            .bar_width(4)
            .bar_gap(1)
            .group_gap(3)
            .max(300)
            .label_style(self.label_style())
            .render(plot, cx);
    }

    fn latency(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Latency by region", area, cx);
        let values: Vec<u64> = BAR_REGIONS
            .iter()
            .zip(0_u32..)
            .map(|((_, base), region)| bar_latency(self.tick, region, *base))
            .collect();
        let texts: Vec<String> = values.iter().map(|value| format!("{value} ms")).collect();
        let bars: Vec<Bar<'_>> = BAR_REGIONS
            .iter()
            .zip(&values)
            .zip(&texts)
            .map(|(((name, _), value), text)| {
                let color = if *value > 120 { self.warn } else { self.accent };
                Bar::new(*value)
                    .label(name)
                    .text_value(text)
                    .style(Style::new().fg(color))
            })
            .collect();
        BarChart::from_bars(&bars)
            .background(self.fill())
            .direction(BarDirection::Horizontal)
            .bar_width(2)
            .bar_gap(1)
            .max(160)
            .label_style(self.label_style())
            .render(plot, cx);
    }

    fn runs(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Build time, last runs", area, cx);
        let columns = usize::from(plot.width / 3).min(24);
        let bars: Vec<Bar<'_>> = (0..columns)
            .map(|index| {
                let run = u32::try_from(index).unwrap_or(0);
                let value = 40 + tri(run * 5 + self.tick / 2, 37, 0, 50) + u64::from(run % 4) * 6;
                let color = if index + 1 == columns {
                    self.focus
                } else {
                    self.accent
                };
                Bar::new(value).style(Style::new().fg(color))
            })
            .collect();
        BarChart::from_bars(&bars)
            .background(self.fill())
            .bar_width(2)
            .bar_gap(1)
            .max(130)
            .render(plot, cx);
    }

    fn states(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Queue depth by state", area, cx);
        let cell_w = plot.width / 3;
        let depth: Vec<Bar<'_>> = (0..8_u32)
            .map(|index| Bar::new(6 + tri(self.tick + index * 6, 40, 0, 26)))
            .collect();
        let states = [
            ("normal", WidgetState::Normal),
            ("focused", WidgetState::Focused),
            ("disabled", WidgetState::Disabled),
        ];
        for (index, (name, state)) in (0_u16..).zip(states) {
            let x = plot.x + index * cell_w;
            let chart = Rect::new(
                x,
                plot.y,
                cell_w.saturating_sub(1),
                plot.height.saturating_sub(2),
            );
            BarChart::from_bars(&depth)
                .background(self.fill())
                .state(state)
                .max(32)
                .render(chart, cx);
            Paragraph::new(name)
                .style(Style::new().fg(self.text).bg(self.surface))
                .render(
                    Rect::new(x, plot.y + plot.height - 1, cell_w.saturating_sub(1), 1),
                    cx,
                );
        }
    }
}

fn barchart_native(
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
    let board = BarBoard::new(&cx, tick);
    let wide = area.width >= 100 && area.height >= 30;
    let body = area.height.saturating_sub(1);
    let top = (body * 55 / 100).max(8);
    let split = if wide {
        area.width * 60 / 100
    } else {
        area.width
    };
    board.requests(Rect::new(area.x, area.y, split, top), &mut cx);
    board.runs(
        Rect::new(area.x, area.y + top, split, body.saturating_sub(top)),
        &mut cx,
    );
    if wide {
        let rest = area.width - split;
        board.latency(Rect::new(area.x + split, area.y, rest, top), &mut cx);
        board.states(
            Rect::new(area.x + split, area.y + top, rest, body.saturating_sub(top)),
            &mut cx,
        );
    }
    Paragraph::new(
        "p50 / p95 / p99 per service  |  latency in ms  |  queue depth: normal, focused, disabled",
    )
    .style(
        Style::new()
            .fg(board.text)
            .bg(board.surface)
            .with_attrs(Attrs::new().set_dim(true)),
    )
    .render(
        Rect::new(area.x + 1, area.y + body, area.width.saturating_sub(2), 1),
        &mut cx,
    );
}

fn barchart_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "barchart") {
        (case.ratatui)(buffer, area, tick);
    }
}

struct ChartBoard {
    text: Color,
    surface: Color,
    edge: Color,
    tick: u32,
}

fn chart_wave(tick: u32, index: u32, base: f64, swing: f64, speed: f64, phase: f64) -> f64 {
    let x = f64::from(index);
    let t = f64::from(tick) * speed;
    base + swing * ((x * 0.22 + t + phase).sin() * 0.7 + (x * 0.61 + t * 1.7 + phase).cos() * 0.3)
}

fn trace(tick: u32, base: f64, swing: f64, speed: f64, phase: f64) -> Vec<(f64, f64)> {
    (0..64_u32)
        .map(|index| {
            (
                f64::from(index),
                chart_wave(tick, index, base, swing, speed, phase).max(0.0),
            )
        })
        .collect()
}

impl ChartBoard {
    fn new(cx: &RenderCx<'_>, tick: u32) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            edge: cx.token(Token::Surface1),
            tick,
        }
    }

    fn panel(&self, title: &str, area: Rect, cx: &mut RenderCx<'_>) -> Rect {
        let panel = Block::bordered()
            .title(format!(" {title} "))
            .style(Style::new().fg(self.text).bg(self.surface))
            .border_style(Style::new().fg(self.edge).bg(self.surface))
            .title_style(Style::new().fg(self.text).bg(self.surface));
        panel.render(area, cx);
        let inner = panel.inner(area);
        Rect::new(
            inner.x + 1,
            inner.y,
            inner.width.saturating_sub(2),
            inner.height,
        )
    }

    fn fill(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    fn throughput(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Throughput", area, cx);
        let requests = trace(self.tick, 330.0, 150.0, 0.11, 0.0);
        let p95 = trace(self.tick, 190.0, 80.0, 0.09, 2.0);
        let errors = trace(self.tick, 60.0, 40.0, 0.14, 4.0);
        let sets = [
            Dataset::new(&requests)
                .name("requests")
                .graph_type(GraphType::Line),
            Dataset::new(&p95)
                .name("p95 ms")
                .graph_type(GraphType::Line),
            Dataset::new(&errors)
                .name("errors")
                .graph_type(GraphType::Line),
        ];
        Chart::new(&sets)
            .background(self.fill())
            .x_axis(
                Axis::new()
                    .title("last 64 s")
                    .bounds([0.0, 63.0])
                    .labels(&["-60s", "-30s", "now"]),
            )
            .y_axis(
                Axis::new()
                    .title("per second")
                    .bounds([0.0, 500.0])
                    .labels(&["0", "250", "500"]),
            )
            .render(plot, cx);
    }

    fn scatter(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Latency by request size", area, cx);
        let point = |index: u32, drift: f64, spread: f64| {
            let x = f64::from(index) * 0.5 + drift;
            let y = 20.0
                + x * 3.4
                + spread * (f64::from(index) * 2.399 + f64::from(self.tick) * 0.05).sin();
            (x.rem_euclid(32.0), y)
        };
        let fast: Vec<(f64, f64)> = (0..48).map(|index| point(index, 0.0, 14.0)).collect();
        let slow: Vec<(f64, f64)> = (0..36).map(|index| point(index, 3.0, 30.0)).collect();
        let sets = [
            Dataset::new(&fast).name("cached").marker(Marker::Dot),
            Dataset::new(&slow).name("cold").marker(Marker::Dot),
        ];
        Chart::new(&sets)
            .background(self.fill())
            .x_axis(
                Axis::new()
                    .title("KiB")
                    .bounds([0.0, 32.0])
                    .labels(&["0", "16", "32"]),
            )
            .y_axis(
                Axis::new()
                    .title("ms")
                    .bounds([0.0, 160.0])
                    .labels(&["0", "80", "160"]),
            )
            .legend_position(Some(LegendPosition::BottomRight))
            .render(plot, cx);
    }

    fn histogram(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Requests by hour", area, cx);
        let bars: Vec<(f64, f64)> = (0..24_u32)
            .map(|hour| {
                let day = (f64::from(hour) / 24.0 * std::f64::consts::TAU - 1.9).sin() * 0.5 + 0.5;
                let beat = (f64::from(self.tick) * 0.12 + f64::from(hour) * 0.7).sin() * 0.06;
                (
                    f64::from(hour),
                    (day * 0.8 + 0.12 + beat).clamp(0.0, 1.0) * 100.0,
                )
            })
            .collect();
        let sets = [Dataset::new(&bars).graph_type(GraphType::Bar)];
        Chart::new(&sets)
            .background(self.fill())
            .x_axis(
                Axis::new()
                    .bounds([0.0, 23.0])
                    .labels(&["00:00", "12:00", "23:00"]),
            )
            .y_axis(Axis::new().bounds([0.0, 100.0]).labels(&["0", "50", "100"]))
            .render(plot, cx);
    }

    fn saturation(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let plot = self.panel("Saturation", area, cx);
        let cpu = trace(self.tick, 62.0, 26.0, 0.1, 1.0);
        let memory = trace(self.tick, 44.0, 10.0, 0.06, 3.0);
        let sets = [
            Dataset::new(&cpu)
                .name("cpu %")
                .marker(Marker::HalfBlock)
                .graph_type(GraphType::Line),
            Dataset::new(&memory)
                .name("mem %")
                .marker(Marker::HalfBlock)
                .graph_type(GraphType::Line),
        ];
        Chart::new(&sets)
            .background(self.fill())
            .state(WidgetState::Focused)
            .x_axis(Axis::new().bounds([0.0, 63.0]).labels(&["-60s", "now"]))
            .y_axis(Axis::new().bounds([0.0, 100.0]).labels(&["0", "50", "100"]))
            .legend_position(Some(LegendPosition::TopLeft))
            .render(plot, cx);
    }
}

fn chart_native(
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
    let board = ChartBoard::new(&cx, tick);
    let wide = area.width >= 100 && area.height >= 30;
    let top = (area.height * 55 / 100).max(8);
    let split = if wide {
        area.width * 60 / 100
    } else {
        area.width
    };
    board.throughput(Rect::new(area.x, area.y, split, top), &mut cx);
    board.histogram(
        Rect::new(area.x, area.y + top, split, area.height.saturating_sub(top)),
        &mut cx,
    );
    if wide {
        let rest = area.width - split;
        board.scatter(Rect::new(area.x + split, area.y, rest, top), &mut cx);
        board.saturation(
            Rect::new(
                area.x + split,
                area.y + top,
                rest,
                area.height.saturating_sub(top),
            ),
            &mut cx,
        );
    }
}

fn chart_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "chart") {
        (case.ratatui)(buffer, area, tick);
    }
}

struct CanvasBoard {
    text: Color,
    surface: Color,
    edge: Color,
    accent: Color,
    ok: Color,
    warn: Color,
    focus: Color,
    tick: u32,
}

impl CanvasBoard {
    fn new(cx: &RenderCx<'_>, tick: u32) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            surface: cx.token(Token::Surface0),
            edge: cx.token(Token::Surface1),
            accent: cx.token(Token::Accent),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
            focus: cx.token(Token::FocusRing),
            tick,
        }
    }

    fn fill(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    fn panel(&self, title: &str, area: Rect, cx: &mut RenderCx<'_>) -> Rect {
        let panel = Block::bordered()
            .title(format!(" {title} "))
            .style(self.fill())
            .border_style(Style::new().fg(self.edge).bg(self.surface))
            .title_style(self.fill());
        panel.render(area, cx);
        panel.inner(area)
    }

    fn radar(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let inner = self.panel("Radar", area, cx);
        let (half_w, half_h) = (f64::from(inner.width), f64::from(inner.height) * 2.0);
        let radius = half_w.min(half_h) - 2.0;
        let sweep = f64::from(self.tick) * 0.045;
        let (text, accent, ok, warn) = (self.text, self.accent, self.ok, self.warn);
        let blips: [(f64, f64, &str); 5] = [
            (0.62, 0.9, "alpha"),
            (0.35, 2.2, "bravo"),
            (0.8, 3.6, "charlie"),
            (0.5, 4.9, "delta"),
            (0.7, 5.8, "echo"),
        ];
        Canvas::new(move |ctx| {
            let ring = Style::new().fg(text).with_attrs(Attrs::new().set_dim(true));
            for fraction in [0.25, 0.5, 0.75, 1.0] {
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: radius * fraction,
                    style: ring,
                });
            }
            ctx.draw(&Line {
                x1: -radius,
                y1: 0.0,
                x2: radius,
                y2: 0.0,
                style: ring,
            });
            ctx.draw(&Line {
                x1: 0.0,
                y1: -radius,
                x2: 0.0,
                y2: radius,
                style: ring,
            });
            ctx.layer();
            let trail = [
                Style::new().fg(ok).with_attrs(Attrs::new().set_dim(true)),
                Style::new().fg(ok),
                Style::new()
                    .fg(accent)
                    .with_attrs(Attrs::new().set_dim(true)),
                Style::new().fg(accent),
            ];
            for (step, style) in trail.iter().enumerate() {
                let behind = f64::from(3_u8 - u8::try_from(step).unwrap_or(0));
                let angle = sweep - 0.09 * behind;
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: radius * angle.cos(),
                    y2: radius * angle.sin(),
                    style: *style,
                });
            }
            ctx.layer();
            for &(distance, bearing, name) in &blips {
                let (x, y) = (
                    radius * distance * bearing.cos(),
                    radius * distance * bearing.sin(),
                );
                let since = (sweep - bearing).rem_euclid(core::f64::consts::TAU);
                let hot = since < 1.6;
                let style = if hot {
                    Style::new().fg(warn)
                } else {
                    Style::new().fg(text).with_attrs(Attrs::new().set_dim(true))
                };
                ctx.draw(&Circle {
                    x,
                    y,
                    radius: 1.6,
                    style,
                });
                if hot {
                    ctx.print(x + 3.0, y, name, style);
                }
            }
        })
        .x_bounds([-half_w, half_w])
        .y_bounds([-half_h, half_h])
        .marker(Marker::Sextant)
        .background(self.fill())
        .render(inner, cx);
    }

    fn scope(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let inner = self.panel("Signals", area, cx);
        let (half_w, half_h) = (f64::from(inner.width), f64::from(inner.height) * 2.0);
        let phase = f64::from(self.tick) * 0.05;
        let (accent, focus, edge) = (self.accent, self.focus, self.edge);
        Canvas::new(move |ctx| {
            ctx.draw(&Line {
                x1: -half_w,
                y1: 0.0,
                x2: half_w,
                y2: 0.0,
                style: Style::new().fg(edge),
            });
            let wave = |x: f64, k: f64, shift: f64| (x * k + shift).sin() * half_h * 0.8;
            let mut last: [Option<(f64, f64)>; 2] = [None, None];
            let mut x = -half_w;
            while x <= half_w {
                let points = [
                    (x, wave(x, 0.11, phase)),
                    (x, wave(x, 0.19, -phase * 1.4) * 0.6),
                ];
                for (index, point) in points.iter().enumerate() {
                    if let Some(previous) = last[index] {
                        let style = Style::new().fg(if index == 0 { accent } else { focus });
                        ctx.draw(&Line {
                            x1: previous.0,
                            y1: previous.1,
                            x2: point.0,
                            y2: point.1,
                            style,
                        });
                    }
                    last[index] = Some(*point);
                }
                x += 1.0;
            }
        })
        .x_bounds([-half_w, half_w])
        .y_bounds([-half_h, half_h])
        .marker(Marker::Sextant)
        .background(self.fill())
        .render(inner, cx);
    }

    fn markers(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let inner = self.panel("Markers", area, cx);
        let names = [
            ("braille", Marker::Braille),
            ("sextant", Marker::Sextant),
            ("quadrant", Marker::Quadrant),
            ("half block", Marker::HalfBlock),
            ("block", Marker::Block),
            ("dot", Marker::Dot),
        ];
        let cell_w = inner.width / 3;
        let cell_h = inner.height / 2;
        let angle = f64::from(self.tick) * 0.06;
        let (accent, text) = (self.accent, self.text);
        for (index, (name, marker)) in (0_u16..).zip(names) {
            let at = Rect::new(
                inner.x + (index % 3) * cell_w,
                inner.y + (index / 3) * cell_h,
                cell_w.saturating_sub(1),
                cell_h,
            );
            let (half_w, half_h) = (f64::from(at.width), f64::from(at.height) * 2.0);
            let radius = half_w.min(half_h) * 0.8;
            Canvas::new(move |ctx| {
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius,
                    style: Style::new().fg(accent),
                });
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: radius * angle.cos(),
                    y2: radius * angle.sin(),
                    style: Style::new().fg(text),
                });
                ctx.print(
                    -half_w,
                    half_h,
                    name,
                    Style::new().fg(text).with_attrs(Attrs::new().set_dim(true)),
                );
            })
            .x_bounds([-half_w, half_w])
            .y_bounds([-half_h, half_h])
            .marker(marker)
            .background(self.fill())
            .render(at, cx);
        }
    }
}

fn canvas_native(
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
    let board = CanvasBoard::new(&cx, tick);
    let split = area.width * 55 / 100;
    board.radar(Rect::new(area.x, area.y, split, area.height), &mut cx);
    let top = area.height * 40 / 100;
    board.scope(
        Rect::new(area.x + split, area.y, area.width - split, top),
        &mut cx,
    );
    board.markers(
        Rect::new(
            area.x + split,
            area.y + top,
            area.width - split,
            area.height - top,
        ),
        &mut cx,
    );
}

fn canvas_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "canvas") {
        (case.ratatui)(buffer, area, tick);
    }
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
    let case = CASES.iter().find(|case| case.name == options.case);
    if case.is_none() && preview(&options.case).is_none() {
        return Err(invalid(format!("unknown case: {}", options.case)));
    }
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

fn play_lunatui(
    out: &mut impl Write,
    case: Option<&'static Case>,
    options: &Options,
) -> io::Result<()> {
    let theme = Theme::builtin(options.variant);
    let area = Rect::new(0, 0, options.width, options.height);
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;
    for tick in 0..options.frames {
        let started = Instant::now();
        match (preview(&options.case), case) {
            (Some(preview), _) => (preview.native)(&mut next, &mut clusters, &theme, area, tick),
            (None, Some(case)) => (case.lunatui)(&mut next, &mut clusters, &theme, area, tick),
            (None, None) => return Err(invalid(format!("unknown case: {}", options.case))),
        }
        downsample_grid(&mut next, options.profile);
        out.write_all(encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor))?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        thread::sleep(period.saturating_sub(started.elapsed()));
    }
    Ok(())
}

fn play_ratatui(case: Option<&'static Case>, options: &Options) -> io::Result<()> {
    let area = RatRect::new(0, 0, options.width, options.height);
    let theme = Theme::builtin(options.variant);
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )?;
    if preview(&options.case).is_none() && case.is_none() {
        return Err(invalid(format!("unknown case: {}", options.case)));
    }
    let period = Duration::from_secs(1) / options.fps;
    for tick in 0..options.frames {
        let started = Instant::now();
        terminal.draw(|frame| {
            let buffer = frame.buffer_mut();
            match (preview(&options.case), case) {
                (Some(preview), _) => (preview.baseline)(buffer, area, &theme, tick),
                (None, Some(case)) => (case.ratatui)(buffer, area, tick),
                (None, None) => {}
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
