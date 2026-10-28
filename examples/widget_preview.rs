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
use lunatui_runtime::keymap::{Command, KeyChord, Keymap};
use lunatui_term::{ColorMode, Encoder, KeyCode, Modifiers};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::canvas::{Circle, Line};
use lunatui_widgets::{
    Alignment, Axis, Bar, BarChart, BarDirection, BarGroup, BigText, Block, BorderType,
    Breadcrumbs, Canvas, Chart, Constraint, Dataset, Date, DateStyle, DividerId, Gauge, GraphType,
    KeyHintBar, LegendPosition, List, MarkdownView, Marker, Monthly, Overlay, PaneId, Paragraph,
    PixelSize, RenderCx, Scrollbar, Sparkline, SplitAxis, SplitDrag, SplitTree, StatusBar, TabMove,
    Table, Tabs, TabsDrag, ToastKind, ToastScope, ToastStack, Widget, WidgetState,
};
#[path = "deploy_board.rs"]
mod deploy_board;
#[cfg(feature = "line-gauge")]
use lunatui_widgets::{LineGauge, LineSet};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::style::Color as RatColor;
#[cfg(feature = "line-gauge")]
use ratatui::widgets::{Block as RatBlock, LineGauge as RatLineGauge, Widget as RatWidget};
use ratatui::{Terminal, TerminalOptions, Viewport};
use time::{Date as TimeDate, Month};

type NativeDraw = fn(&mut Grid, &mut ClusterTable, &Theme, Rect, u32);
type BaselineDraw = fn(&mut Buffer, RatRect, &Theme, u32);

struct Preview {
    name: &'static str,
    native: NativeDraw,
    baseline: BaselineDraw,
}

#[cfg(feature = "line-gauge")]
const LINE_GAUGE_LABELS: [&str; 10] = [
    "CPU", "QUEUE", "DISK", "MEMORY", "NETWORK", "CACHE", "JOBS", "LATENCY", "ERRORS", "BUDGET",
];

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
        name: "tabs-drag",
        native: tabs_drag_native,
        baseline: tabs_drag_baseline,
    },
    Preview {
        name: "breadcrumbs",
        native: breadcrumbs_native,
        baseline: breadcrumbs_baseline,
    },
    Preview {
        name: "key-hints",
        native: key_hints_native,
        baseline: key_hints_baseline,
    },
    Preview {
        name: "overlay",
        native: overlay_native,
        baseline: overlay_baseline,
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
    #[cfg(feature = "line-gauge")]
    Preview {
        name: "line-gauge",
        native: line_gauge_motion,
        baseline: line_gauge_baseline,
    },
];

#[cfg(feature = "line-gauge")]
fn line_gauge_motion(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let frame = Block::bordered()
        .title(" service health ")
        .style(
            Style::new()
                .fg(theme.resolve(Token::TextPrimary))
                .bg(theme.resolve(Token::Surface0)),
        )
        .title_style(
            Style::new()
                .fg(theme.resolve(Token::TextStrong))
                .bg(theme.resolve(Token::Surface0)),
        )
        .border_style(Style::new().fg(theme.resolve(Token::Border)));
    frame.render(area, &mut RenderCx::new(grid, clusters, theme));
    let inner = frame.inner(area);
    let row_gap = if inner.height > 40 { 5 } else { 3 };
    let row_count = usize::from(inner.height.saturating_add(row_gap - 1) / row_gap)
        .min(LINE_GAUGE_LABELS.len());
    for (index, label) in LINE_GAUGE_LABELS
        .iter()
        .copied()
        .take(row_count)
        .enumerate()
    {
        let offset = u16::try_from(index)
            .unwrap_or(u16::MAX)
            .saturating_mul(row_gap);
        let row = Rect::new(inner.x, inner.y.saturating_add(offset), inner.width, 1);
        let set = line_gauge_set(index);
        let state = line_gauge_state(index);
        let ratio = line_gauge_ratio(tick, index);
        LineGauge::new()
            .label(label)
            .ratio(ratio)
            .line_set(set)
            .subcell(set != LineSet::Ascii)
            .style(Style::new().bg(theme.resolve(Token::Surface0)))
            .state(state)
            .render(row, &mut RenderCx::new(grid, clusters, theme));
    }
}

#[cfg(feature = "line-gauge")]
fn line_gauge_baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let frame = RatBlock::bordered()
        .title(" service health ")
        .style(
            ratatui::style::Style::new()
                .fg(rat_color(theme.resolve(Token::TextPrimary)))
                .bg(rat_color(theme.resolve(Token::Surface0))),
        )
        .title_style(
            ratatui::style::Style::new()
                .fg(rat_color(theme.resolve(Token::TextStrong)))
                .bg(rat_color(theme.resolve(Token::Surface0))),
        )
        .border_style(ratatui::style::Style::new().fg(rat_color(theme.resolve(Token::Border))));
    let inner = frame.inner(area);
    frame.render(area, buffer);
    let row_gap = if inner.height > 40 { 5 } else { 3 };
    let row_count = usize::from(inner.height.saturating_add(row_gap - 1) / row_gap)
        .min(LINE_GAUGE_LABELS.len());
    for (index, label) in LINE_GAUGE_LABELS
        .iter()
        .copied()
        .take(row_count)
        .enumerate()
    {
        let offset = u16::try_from(index)
            .unwrap_or(u16::MAX)
            .saturating_mul(row_gap);
        let row = RatRect::new(inner.x, inner.y.saturating_add(offset), inner.width, 1);
        let set = line_gauge_set(index);
        let state = line_gauge_state(index);
        let ratio = line_gauge_ratio(tick, index);
        let label = format!("{label}  {:3.0}%", ratio * 100.0);
        let fill = if state == WidgetState::Disabled {
            Token::TextMuted
        } else if ratio >= 0.95 {
            Token::Fail
        } else if ratio >= 0.8 {
            Token::Warn
        } else if state == WidgetState::Focused {
            Token::FocusRing
        } else {
            Token::Accent
        };
        let track = if state == WidgetState::Disabled {
            Token::TextMuted
        } else {
            Token::Border
        };
        let (filled, unfilled) = line_gauge_symbols(set);
        RatLineGauge::default()
            .ratio(ratio)
            .label(label)
            .filled_symbol(filled)
            .unfilled_symbol(unfilled)
            .style(
                ratatui::style::Style::new()
                    .fg(rat_color(theme.resolve(Token::TextPrimary)))
                    .bg(rat_color(theme.resolve(Token::Surface0))),
            )
            .filled_style(ratatui::style::Style::new().fg(rat_color(theme.resolve(fill))))
            .unfilled_style(ratatui::style::Style::new().fg(rat_color(theme.resolve(track))))
            .render(row, buffer);
    }
}

#[cfg(feature = "line-gauge")]
fn line_gauge_set(index: usize) -> LineSet {
    match index % 4 {
        0 => LineSet::Normal,
        1 => LineSet::Thick,
        2 => LineSet::Double,
        _ => LineSet::Ascii,
    }
}

#[cfg(feature = "line-gauge")]
fn line_gauge_symbols(set: LineSet) -> (&'static str, &'static str) {
    match set {
        LineSet::Normal => ("─", "─"),
        LineSet::Thick => ("━", "━"),
        LineSet::Double => ("═", "═"),
        LineSet::Ascii => ("#", "-"),
        _ => ("━", "─"),
    }
}

#[cfg(feature = "line-gauge")]
fn line_gauge_state(index: usize) -> WidgetState {
    if index == 0 {
        WidgetState::Focused
    } else if index == LINE_GAUGE_LABELS.len() - 1 {
        WidgetState::Disabled
    } else {
        WidgetState::Normal
    }
}

#[cfg(feature = "line-gauge")]
fn line_gauge_ratio(tick: u32, row: usize) -> f64 {
    let phase = f64::from((tick + u32::try_from(row).unwrap_or(0) * 24) % 240) / 240.0
        * std::f64::consts::TAU;
    0.18 + (0.76 * (0.5 + phase.sin() * 0.5))
}

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
        "# Getting started\n\nLunatui draws terminal interfaces from small **widgets**. Each widget takes an area and a `RenderCx` and writes cells; nothing else is needed to get a frame on screen.\n\n## Install\n\n1. Add the crate with the features you use.\n2. Create a `Grid` and a `ClusterTable`.\n3. Render a widget into the grid.\n4. Diff the grid and write the changes.\n\n```toml\n[dependencies]\nlunatui = \"0.1\"\nlunatui-widgets = { version = \"0.1\", features = [\"markdown\"] }\n```\n\n## Your first frame\n\n> Every widget redraws the whole of its own area, so a frame never shows leftovers from the one before it.\n\n- [x] Block and Paragraph\n- [x] List and Table\n- [ ] Your own widget\n  - implement `Widget`\n  - keep `render` free of side effects\n\n## Themes\n\nColors come from semantic tokens, so a theme change recolors every widget without touching the widget code. The built-in dark and light themes cover the common terminals, and a prepared palette downsamples once instead of on every cell.\n\n| token | used for |\n|:--|:--|\n| `Accent` | headings and highlights |\n| `FocusRing` | links and focus |\n| `Surface1` | raised panels and code |\n\n## Input\n\nKeys, mouse, and paste events arrive as plain values. The runtime maps them to focus changes and hit tests, and keeps a small keymap so shortcuts stay discoverable.\n\n1. Parse bytes into events.\n2. Route the event to the focused widget.\n3. Redraw only what changed.\n\n---\n\nRead the *architecture overview* next, or jump to the [widget gallery](https://example.com/gallery).\n",
    ),
    (
        "Layout",
        "# Layout\n\nA layout splits an area into rectangles from a list of **constraints**. The solver is exact for the common cases and falls back to a general solver only when it must.\n\n## Constraints\n\n| constraint | meaning |\n|:--|--:|\n| `Length(n)` | exactly n cells |\n| `Min(n)` | at least n cells |\n| `Percentage(p)` | p percent of the axis |\n| `Fill(w)` | a weighted share of the rest |\n\n## Flex\n\n- `Start`, `Center`, and `End` place the group\n- `SpaceBetween` spreads the gaps\n  - a lone segment sits at the start\n- `Legacy` keeps the older rules\n\n```rust\nlet [left, right] = Layout::horizontal([Length(24), Fill(1)])\n    .split(area)\n    .into_array();\n```\n\n> Floors and caps are opt in: turn on the `size-limits` feature to bound a segment.\n\n## Wrapping\n\nSegments that do not fit move onto the next line, so a row of tags or buttons reflows when the window narrows. Lines break greedily: a segment joins the current line when its natural size plus the spacing fits in what is left.\n\n1. Measure each segment.\n2. Break the list into lines.\n3. Split every line with the same solver.\n\n## Spacing\n\nOne `gap` setting spaces segments and lines alike, which keeps grids and toolbars consistent. Negative spacing overlaps neighbours by a cell, which is how bordered panels share an edge.\n\n- [x] horizontal and vertical splits\n- [x] wrapped lines\n- [ ] nested constraints in one pass\n\n---\n\nThe solver never allocates on the fast path.\n",
    ),
    (
        "Widgets",
        "# Widgets\n\nThe widget set covers text, lists, tables, gauges, charts, and now **Markdown**.\n\n## Markdown\n\nThe view lays a parsed document out at the width it is given: headings get rules, lists keep a hanging indent, quotes keep their bar, and tables are boxed. ~~Raw source~~ is never shown.\n\n1. Parse once with `markdown::parse`.\n2. Render with `MarkdownView::new(&document)`.\n3. Scroll with `.scroll(rows)` and size a scrollbar from `content_height`.\n\n### Inline styles\n\nText can be *emphasised*, **strong**, ***both***, ~~struck~~, or `code`, and a [link](https://example.com) is underlined.\n\n| widget | feature |\n|:--|:-:|\n| `Sparkline` | sparkline |\n| `BarChart` | barchart |\n| `Chart` | chart |\n| `MarkdownView` | markdown |\n\n## Lists and tasks\n\n- [x] headings, lists, quotes\n- [x] code blocks and tables\n- [x] task items\n  - nested items indent\n    - and nest again\n\n## Quotes\n\n> A quote keeps its bar on every row, even when the text wraps to a second or a third line.\n>\n> > Quotes nest as deep as the text needs.\n\n## Code\n\n```rust\nlet document = markdown::parse(source, Options::gfm());\nMarkdownView::new(&document)\n    .scroll(offset)\n    .render(area, &mut cx);\n```\n\n---\n\nThat is the whole tour.\n",
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
    let page = usize::try_from(tick / 80).unwrap_or(0) % MD_PAGES.len();
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
    let phase = usize::try_from(tick % 80)
        .unwrap_or(0)
        .saturating_sub(6)
        .min(68);
    let scroll = travel * phase / 68;
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

const TAB_TITLES: [&str; 5] = ["Overview", "Builds", "Deploys", "Logs", "Settings"];
const TAB_HOVER: u32 = 10;
const TAB_LIFT: u32 = 6;
const TAB_HOLD: u32 = 8;
const TAB_SETTLE: u32 = 9;

struct TabDragStep {
    from: u32,
    to: u32,
    grab: usize,
    drop: usize,
}

const TAB_STEPS: [TabDragStep; 3] = [
    TabDragStep {
        from: 24,
        to: 80,
        grab: 1,
        drop: 3,
    },
    TabDragStep {
        from: 105,
        to: 155,
        grab: 1,
        drop: 0,
    },
    TabDragStep {
        from: 180,
        to: 228,
        grab: 2,
        drop: 4,
    },
];

const BUILDS: [(&str, &str, &str, &str, &str, u32); 10] = [
    (
        "4820",
        "feature/split-tree",
        "9be2d71",
        "Resize panes with the pointer",
        "passed",
        252,
    ),
    (
        "4819",
        "main",
        "5f03a8c",
        "Clip list text before its mark",
        "passed",
        198,
    ),
    (
        "4818",
        "fix/toast-expiry",
        "c47e10b",
        "Expire toasts on the virtual clock",
        "failed",
        87,
    ),
    (
        "4817",
        "main",
        "e81a4f2",
        "Add Canvas quadrant markers",
        "passed",
        231,
    ),
    (
        "4816",
        "feature/canvas",
        "02d6b9e",
        "Draw circles with sextants",
        "passed",
        244,
    ),
    (
        "4815",
        "docs/porting",
        "7ac3e51",
        "Explain the theme tokens",
        "passed",
        64,
    ),
    (
        "4814",
        "main",
        "b3f90d7",
        "Keep the image ring background",
        "passed",
        205,
    ),
    (
        "4813",
        "fix/scroll-edge",
        "4d2c8a6",
        "Clip popups at the grid edge",
        "canceled",
        31,
    ),
    (
        "4812",
        "main",
        "f17e0c3",
        "Add the BarChart widget",
        "passed",
        219,
    ),
    (
        "4811",
        "feature/barchart",
        "6e5b2d4",
        "Label grouped bars",
        "passed",
        226,
    ),
];

const BUILD_STEPS: [&str; 6] = ["checkout", "build", "clippy", "test", "bench", "package"];

const DEPLOYS: [(&str, &str, &str, u32, &str); 9] = [
    ("web", "prod", "v2.14.0", 4, "14m"),
    ("worker", "prod", "v2.13.2", 8, "3h"),
    ("edge", "prod", "v2.14.0", 12, "16m"),
    ("scheduler", "prod", "v1.9.4", 2, "2d"),
    ("billing", "prod", "v3.2.1", 3, "5d"),
    ("api", "staging", "v2.15.0-rc.1", 2, "41m"),
    ("web", "staging", "v2.15.0-rc.1", 2, "41m"),
    ("worker", "staging", "v2.15.0-rc.1", 4, "40m"),
    ("edge", "staging", "v2.15.0-rc.1", 3, "39m"),
];

const LOG_SOURCES: [&str; 5] = ["api", "worker-2", "edge", "scheduler", "billing"];

struct TabInk {
    fill: Style,
    dim: Style,
    muted: Style,
    header: Style,
    active: Style,
    frame: Style,
    marker: Style,
    lifted: Style,
    lifted_title: Style,
    pointer: Style,
    status: Style,
    status_key: Style,
    ok: Style,
    warn: Style,
    fail: Style,
    gauge: Style,
    track: Style,
    soft: Style,
    lead: Style,
}

impl TabInk {
    fn new(theme: &Theme) -> Self {
        let surface = theme.resolve(Token::Surface0);
        let raised = theme.resolve(Token::Surface1);
        let text = theme.resolve(Token::TextPrimary);
        let accent = theme.resolve(Token::Accent);
        let bold = Attrs::new().set_bold(true);
        let fill = Style::new().fg(text).bg(surface);
        Self {
            fill,
            dim: fill.with_attrs(Attrs::new().set_dim(true)),
            muted: fill.with_attrs(Attrs::new().set_dim(true)),
            header: fill.with_attrs(Attrs::new().set_dim(true)),
            active: Style::new().fg(accent).bg(surface).with_attrs(bold),
            frame: Style::new().fg(accent).bg(surface),
            marker: Style::new()
                .fg(accent)
                .bg(surface)
                .with_attrs(Attrs::new().set_dim(true)),
            lifted: Style::new().fg(accent).bg(raised).with_attrs(bold),
            lifted_title: Style::new().fg(text).bg(raised).with_attrs(bold),
            pointer: Style::new().fg(text).bg(surface).with_attrs(bold),
            status: Style::new().fg(text).bg(raised),
            status_key: Style::new().fg(accent).bg(raised).with_attrs(bold),
            ok: Style::new().fg(theme.resolve(Token::Ok)),
            warn: Style::new().fg(theme.resolve(Token::Warn)),
            fail: Style::new().fg(theme.resolve(Token::Fail)),
            gauge: Style::new().fg(accent).bg(raised),
            track: Style::new().bg(raised),
            soft: Style::new().with_attrs(Attrs::new().set_dim(true)),
            lead: Style::new().fg(accent).with_attrs(bold),
        }
    }
}

struct TabGhost {
    title: &'static str,
    x: u16,
    lift: u16,
}

struct TabDragFrame {
    titles: Vec<&'static str>,
    shown: Vec<&'static str>,
    selected: usize,
    hole: Option<usize>,
    drag: TabsDrag,
    ghost: Option<TabGhost>,
    pointer: Option<u16>,
    moved: Option<(&'static str, usize)>,
}

fn tab_slot_width(title: &str) -> u16 {
    u16::try_from(cols(title)).unwrap_or(0).saturating_add(4)
}

fn tab_strip_areas(titles: &[&'static str], strip: Rect) -> Vec<(usize, Rect)> {
    Tabs::new(titles.iter().copied())
        .boxed(Style::new(), Style::new())
        .tab_areas(strip)
}

fn tab_slot(areas: &[(usize, Rect)], index: usize) -> Rect {
    areas
        .iter()
        .find(|(i, _)| *i == index)
        .map_or(Rect::new(0, 0, 0, 0), |(_, rect)| *rect)
}

fn tab_center(areas: &[(usize, Rect)], index: usize) -> u16 {
    let slot = tab_slot(areas, index);
    slot.x + slot.width / 2
}

fn eased(start: u16, end: u16, spent: u32, span: u32) -> u16 {
    let span = i64::from(span.max(1));
    let spent = i64::from(spent).min(span);
    let curve = spent * spent * (3 * span - 2 * spent);
    let (start, end) = (i64::from(start), i64::from(end));
    u16::try_from(start + (end - start) * curve / (span * span * span)).unwrap_or(0)
}

fn tab_drag_frame(strip: Rect, tick: u32) -> TabDragFrame {
    let row = strip.y + 1;
    let mut titles = TAB_TITLES.to_vec();
    let mut selected = 3_usize;
    let mut drag = TabsDrag::new();
    let mut column = 0_u16;
    let mut pressed = 0_u32;
    let mut released: Option<(u32, usize, u16)> = None;
    for now in 0..=tick {
        for step in &TAB_STEPS {
            let areas = tab_strip_areas(&titles, strip);
            if now == step.from {
                column = tab_center(&areas, step.grab);
                drag.press(&areas, column, row);
                selected = step.grab;
                pressed = now;
                released = None;
            } else if now > step.from && now <= step.to {
                let travel = step.from + TAB_LIFT;
                let span = (step.to - TAB_HOLD).saturating_sub(travel);
                column = eased(
                    tab_center(&areas, step.grab),
                    tab_center(&areas, step.drop),
                    now.saturating_sub(travel),
                    span,
                );
                drag.drag(&areas, column, row);
                if now == step.to
                    && let Some(reorder) = drag.release()
                {
                    reorder.apply(&mut titles);
                    selected = reorder.selection(selected);
                    released = Some((now, reorder.to, column));
                }
            }
        }
    }
    let mut frame = TabDragFrame {
        shown: titles.clone(),
        titles,
        selected,
        hole: None,
        drag,
        ghost: None,
        pointer: None,
        moved: None,
    };
    if let Some(from) = drag.from() {
        let to = drag.target().unwrap_or(from);
        TabMove { from, to }.apply(&mut frame.shown);
        let title = frame.titles.get(from).copied().unwrap_or("");
        frame.hole = Some(to);
        frame.ghost = Some(TabGhost {
            title,
            x: column.saturating_sub(tab_slot_width(title) / 2),
            lift: u16::from(tick >= pressed + 2),
        });
        frame.pointer = Some(column);
        return frame;
    }
    if let Some((at, to, from_column)) = released {
        let title = frame.titles.get(to).copied().unwrap_or("");
        frame.moved = Some((title, to));
        let spent = tick - at;
        if spent < TAB_SETTLE {
            let rest = tab_slot(&tab_strip_areas(&frame.titles, strip), to).x;
            let start = from_column.saturating_sub(tab_slot_width(title) / 2);
            frame.hole = Some(to);
            frame.ghost = Some(TabGhost {
                title,
                x: eased(start, rest, spent, TAB_SETTLE - 3),
                lift: u16::from(spent < TAB_SETTLE - 3),
            });
        }
    }
    let next = TAB_STEPS
        .iter()
        .find(|step| step.from > tick && step.from - tick <= TAB_HOVER);
    if let Some(step) = next {
        frame.pointer = Some(tab_center(
            &tab_strip_areas(&frame.titles, strip),
            step.grab,
        ));
    }
    frame
}

fn draw_tab_strip(frame: &TabDragFrame, strip: Rect, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let labels = frame.shown.iter().enumerate().map(|(index, title)| {
        if frame.hole == Some(index) {
            " ".repeat(cols(title))
        } else {
            (*title).to_string()
        }
    });
    let (select, box_style) = match frame.hole {
        Some(hole) => (hole, ink.marker),
        None => (frame.selected, ink.frame),
    };
    Tabs::new(labels)
        .select(select)
        .style(ink.dim)
        .highlight_style(ink.active)
        .boxed(box_style, ink.dim)
        .render(strip, cx);
    if let Some(ghost) = &frame.ghost {
        let width = tab_slot_width(ghost.title);
        let y = strip.y.saturating_sub(ghost.lift);
        Block::bordered()
            .border_type(BorderType::Rounded)
            .style(ink.lifted_title)
            .border_style(ink.lifted)
            .render(Rect::new(ghost.x, y, width, 3), cx);
        Paragraph::new(format!(" {} ", ghost.title))
            .style(ink.lifted_title)
            .render(Rect::new(ghost.x + 1, y + 1, width - 2, 1), cx);
    }
    if let Some(column) = frame.pointer {
        Paragraph::new("↖")
            .style(ink.pointer)
            .render(Rect::new(column, strip.y + 2, 1, 1), cx);
    }
}

fn duration_label(seconds: u32) -> String {
    format!("{}m{:02}s", seconds / 60, seconds % 60)
}

fn status_style(status: &str, ink: &TabInk) -> Style {
    match status {
        "passed" | "healthy" => ink.ok,
        "failed" | "degraded" => ink.fail,
        "running" | "rolling" => ink.warn,
        _ => ink.soft,
    }
}

fn tick_ratio(tick: u32, start: u32, span: u32) -> f32 {
    let spent = u16::try_from(tick.saturating_sub(start).min(span)).unwrap_or(0);
    f32::from(spent) / f32::from(u16::try_from(span.max(1)).unwrap_or(u16::MAX))
}

fn progress_gauge(title: String, ratio: f32, area: Rect, ink: &TabInk, cx: &mut RenderCx<'_>) {
    Gauge::new(ratio)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(title)
                .style(ink.fill)
                .border_style(ink.dim)
                .title_style(ink.header),
        )
        .gauge_style(ink.gauge)
        .track_style(ink.track)
        .sub_cell(true)
        .render(area, cx);
}

fn builds_body(area: Rect, tick: u32, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let ratio = 0.3 + 0.7 * tick_ratio(tick, 0, 240);
    let done = ratio >= 1.0;
    let step = BUILD_STEPS
        .get(usize::try_from(tick * 6 / 241).unwrap_or(0))
        .copied()
        .unwrap_or("package");
    let running_status = if done { "passed" } else { "running" };
    let mut rows = vec![vec![
        "#4821".to_string(),
        "feature/tabs-drag".to_string(),
        "882be46".to_string(),
        "Reorder tabs with the pointer".to_string(),
        running_status.to_string(),
        duration_label(151 + tick.min(240) / 2),
    ]];
    let mut styles = vec![vec![
        ink.lead,
        Style::new(),
        ink.soft,
        Style::new(),
        status_style(running_status, ink),
        Style::new(),
    ]];
    for (build, branch, commit, message, status, seconds) in BUILDS {
        rows.push(vec![
            format!("#{build}"),
            branch.to_string(),
            commit.to_string(),
            message.to_string(),
            status.to_string(),
            duration_label(seconds),
        ]);
        styles.push(vec![
            Style::new(),
            Style::new(),
            ink.soft,
            Style::new(),
            status_style(status, ink),
            Style::new(),
        ]);
    }
    let table_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(4));
    Table::new(
        lunatui_widgets::VecSource::from_rows(rows),
        vec![
            Constraint::Length(7),
            Constraint::Length(20),
            Constraint::Length(9),
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Length(7),
        ],
    )
    .header(
        ["Build", "Branch", "Commit", "Message", "Status", "Time"].map(|title| (title, ink.header)),
    )
    .style(ink.fill)
    .alignments(vec![
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Right,
    ])
    .odd_row_style(ink.track)
    .cell_styles(styles)
    .render(table_area, cx);
    let percent = u32::from(u16::try_from(tick.min(240) * 70 / 240).unwrap_or(0)) + 30;
    let label = if done {
        " #4821 feature/tabs-drag  passed ".to_string()
    } else {
        format!(" #4821 feature/tabs-drag  {step}  {percent}% ")
    };
    let gauge_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(3),
        area.width,
        3,
    );
    progress_gauge(label, ratio, gauge_area, ink, cx);
}

fn deploys_body(area: Rect, tick: u32, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let ready = (1 + tick.saturating_sub(80) / 14).min(6);
    let rolling = if ready < 6 { "rolling" } else { "healthy" };
    let mut rows = vec![vec![
        "api".to_string(),
        "prod".to_string(),
        "v2.14.0".to_string(),
        format!("{ready}/6"),
        rolling.to_string(),
        format!("{}s", 20 + tick / 3),
    ]];
    let mut styles = vec![vec![
        ink.lead,
        Style::new(),
        Style::new(),
        Style::new(),
        status_style(rolling, ink),
        Style::new(),
    ]];
    for (index, (service, env, version, replicas, age)) in (0_u32..).zip(DEPLOYS) {
        let up = if index == 1 { replicas - 1 } else { replicas };
        let status = if up < replicas { "degraded" } else { "healthy" };
        rows.push(vec![
            service.to_string(),
            env.to_string(),
            version.to_string(),
            format!("{up}/{replicas}"),
            status.to_string(),
            age.to_string(),
        ]);
        styles.push(vec![
            Style::new(),
            if env == "prod" {
                Style::new()
            } else {
                ink.soft
            },
            Style::new(),
            Style::new(),
            status_style(status, ink),
            Style::new(),
        ]);
    }
    let table_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(4));
    Table::new(
        lunatui_widgets::VecSource::from_rows(rows),
        vec![
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(9),
            Constraint::Length(10),
            Constraint::Fill(1),
        ],
    )
    .header(
        ["Service", "Env", "Version", "Ready", "Status", "Age"].map(|title| (title, ink.header)),
    )
    .style(ink.fill)
    .alignments(vec![
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Right,
    ])
    .odd_row_style(ink.track)
    .cell_styles(styles)
    .render(table_area, cx);
    let label = format!(" api prod  v2.13.2 → v2.14.0  {ready}/6 ready ");
    let ratio = f32::from(u16::try_from(ready).unwrap_or(0)) / 6.0;
    let gauge_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(3),
        area.width,
        3,
    );
    progress_gauge(label, ratio, gauge_area, ink, cx);
}

fn log_line(index: u32) -> (String, &'static str, &'static str, String) {
    let tenths = 24_000 + index * 37 / 10;
    let time = format!(
        "12:{:02}:{:02}.{}",
        tenths / 600,
        tenths / 10 % 60,
        tenths % 10
    );
    let source = LOG_SOURCES[usize::try_from(index % 5).unwrap_or(0)];
    let (level, message) = if index % 19 == 11 {
        (
            "ERROR",
            format!(
                "deploy hook timed out after 30s (attempt {})",
                index % 3 + 1
            ),
        )
    } else if index % 7 == 3 {
        (
            "WARN",
            format!("slow query on builds: {}ms", 240 + index % 9 * 31),
        )
    } else {
        let message = match index % 6 {
            0 => format!("GET /v1/builds 200 {}ms", 12 + index % 23),
            1 => format!(
                "job {} finished in {}.{}s",
                8800 + index,
                1 + index % 4,
                index % 10
            ),
            2 => format!("cache hit ratio 0.9{}", index % 10),
            3 => format!("POST /v1/deploys 201 {}ms", 40 + index % 37),
            4 => format!("pool connections {}/32", 10 + index % 17),
            _ => format!("health check ok in {}ms", 2 + index % 7),
        };
        ("INFO", message)
    };
    (time, level, source, message)
}

fn logs_body(area: Rect, tick: u32, ink: &TabInk, cx: &mut RenderCx<'_>) {
    let count = 60 + tick / 5;
    let visible = u32::from(area.height.saturating_sub(1));
    let mut rows = Vec::new();
    let mut styles = Vec::new();
    for index in count.saturating_sub(visible)..count {
        let (time, level, source, message) = log_line(index);
        let level_style = match level {
            "ERROR" => ink.fail,
            "WARN" => ink.warn,
            _ => ink.ok,
        };
        rows.push(vec![time, level.to_string(), source.to_string(), message]);
        styles.push(vec![ink.soft, level_style, ink.soft, Style::new()]);
    }
    Table::new(
        lunatui_widgets::VecSource::from_rows(rows),
        vec![
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(11),
            Constraint::Fill(1),
        ],
    )
    .header(["Time", "Level", "Source", "Message"].map(|title| (title, ink.header)))
    .style(ink.fill)
    .cell_styles(styles)
    .render(area, cx);
}

fn tabs_drag_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let ink = TabInk::new(theme);
    grid.fill(area, ink.fill, ' ', clusters);
    let mut cx = RenderCx::new(grid, clusters, theme);
    Paragraph::new(" Release pipelines")
        .style(ink.header)
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let strip = Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 3);
    let frame = tab_drag_frame(strip, tick);
    let body = Rect::new(
        area.x + 2,
        strip.y + 3,
        area.width.saturating_sub(4),
        area.height.saturating_sub(strip.y + 3 - area.y + 1),
    );
    let name = frame.titles.get(frame.selected).copied().unwrap_or("");
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {name} "))
        .style(ink.fill)
        .border_style(ink.dim)
        .title_style(ink.active);
    let inner = block.inner(body);
    block.render(body, &mut cx);
    let content = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    match name {
        "Builds" => builds_body(content, tick, &ink, &mut cx),
        "Deploys" => deploys_body(content, tick, &ink, &mut cx),
        "Logs" => logs_body(content, tick, &ink, &mut cx),
        _ => Paragraph::new("5 services healthy, 1 rolling out")
            .style(ink.muted)
            .render(content, &mut cx),
    }
    draw_tab_strip(&frame, strip, &ink, &mut cx);
    let hint = match (frame.drag.from(), frame.drag.target(), frame.moved) {
        (Some(from), Some(target), _) => format!(
            " moving {} to position {} of {}   release to drop",
            frame.titles.get(from).copied().unwrap_or(""),
            target + 1,
            frame.titles.len()
        ),
        (Some(_), None, _) => " release to cancel".to_string(),
        (None, _, Some((title, to))) => format!(" moved {title} to position {}", to + 1),
        (None, _, None) => " drag a tab to reorder".to_string(),
    };
    StatusBar::new(hint)
        .style(ink.status)
        .left([(" main ", ink.status_key)])
        .right([(" 6 services  3 workers ", ink.status)])
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

fn tabs_drag_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "tabs") {
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
    let squeeze = if (96..176).contains(&tick) {
        u64::from(span) - tri(tick - 96, 80, 0, u64::from(span.saturating_sub(26)))
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

const KH_MOVE: &[KeyChord] = &[KeyChord::new(KeyCode::Down, Modifiers::new())];
const KH_OPEN: &[KeyChord] = &[KeyChord::new(KeyCode::Enter, Modifiers::new())];
const KH_SEARCH: &[KeyChord] = &[KeyChord::new(KeyCode::Char('/'), Modifiers::new())];
const KH_PUBLISH: &[KeyChord] = &[KeyChord::new(KeyCode::Char('p'), Modifiers::new())];
const KH_QUIT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];
const KH_ESC: &[KeyChord] = &[KeyChord::new(KeyCode::Esc, Modifiers::new())];
const KH_CLEAR: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('u'),
    Modifiers::new().control(),
)];
const KH_YES: &[KeyChord] = &[KeyChord::new(KeyCode::Char('y'), Modifiers::new())];
const KH_NO: &[KeyChord] = &[KeyChord::new(KeyCode::Char('n'), Modifiers::new())];

fn kh_keymap(mode: usize) -> Keymap {
    let mut keymap = Keymap::new();
    let commands: &[Command] = match mode {
        0 => &[
            Command::new("move", "Move", KH_MOVE),
            Command::new("open", "Open log", KH_OPEN),
            Command::new("search", "Search checks", KH_SEARCH),
            Command::new("publish", "Publish", KH_PUBLISH),
            Command::new("quit", "Quit", KH_QUIT),
        ],
        1 => &[
            Command::new("apply", "Apply filter", KH_OPEN),
            Command::new("clear", "Clear", KH_CLEAR),
            Command::new("cancel", "Cancel", KH_ESC),
        ],
        _ => &[
            Command::new("yes", "Publish v0.1.13", KH_YES),
            Command::new("no", "Go back", KH_NO),
            Command::new("cancel", "Cancel", KH_ESC),
        ],
    };
    for command in commands {
        keymap.register(*command);
    }
    keymap
}

const KH_CHECKS: [(&str, &str); 8] = [
    ("fmt", "18 s"),
    ("clippy", "41 s"),
    ("test", "2 m 12 s"),
    ("msrv", "49 s"),
    ("deny", "2 m 22 s"),
    ("docs", "25 s"),
    ("bench", "3 m 32 s"),
    ("scorecard", "1 m 54 s"),
];

fn kh_state(tick: u32) -> (usize, String, usize) {
    let mode = if (150..200).contains(&tick) {
        2
    } else {
        usize::from((60..130).contains(&tick))
    };
    let query = if mode == 1 {
        let typed = usize::try_from((tick - 60) / 14).unwrap_or(0).min(4);
        "test".chars().take(typed.min(4)).collect()
    } else if (130..150).contains(&tick) {
        String::from("test")
    } else {
        String::new()
    };
    (mode, query, usize::try_from(tick / 12).unwrap_or(0))
}

fn key_hints_native(
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
    let (mode, query, step) = kh_state(tick);
    let header = Style::new()
        .fg(accent)
        .bg(raised)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(area.x, area.y, &" ".repeat(usize::from(area.width)), header);
    cx.put_str(
        area.x + 1,
        area.y,
        "Release console  ·  lunatui v0.1.13",
        header,
    );
    let shown: Vec<(&str, &str)> = KH_CHECKS
        .iter()
        .copied()
        .filter(|(name, _)| query.is_empty() || name.contains(query.as_str()))
        .collect();
    let panes = Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 15);
    kh_log(
        &mut cx,
        Rect::new(
            area.x + 2,
            area.y + 17,
            area.width.saturating_sub(4),
            area.height.saturating_sub(19),
        ),
        tick,
        [fill, dim],
        [accent, raised],
    );
    kh_panes(
        &mut cx,
        panes,
        &shown,
        (mode, &query, step),
        [fill, dim],
        [accent, raised],
    );
    if mode == 2 {
        kh_confirm(&mut cx, area, [fill, dim], accent);
    }
    let bar = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        1,
    );
    cx.put_str(
        bar.x,
        bar.y,
        &" ".repeat(usize::from(bar.width)),
        Style::new().fg(text).bg(raised),
    );
    KeyHintBar::new(&kh_keymap(mode)).gap(3).render(
        Rect::new(bar.x + 1, bar.y, bar.width.saturating_sub(2), 1),
        &mut cx,
    );
}

fn kh_panes(
    cx: &mut RenderCx<'_>,
    area: Rect,
    shown: &[(&str, &str)],
    state: (usize, &str, usize),
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let (mode, query, step) = state;
    let [fill, dim] = styles;
    let [accent, raised] = colors;
    let left = Rect::new(area.x, area.y, 46, area.height);
    let right = Rect::new(
        left.x + left.width + 1,
        area.y,
        area.width.saturating_sub(left.width + 1),
        area.height,
    );
    let title = if mode == 1 {
        format!(" Checks  /{query}▏ ")
    } else {
        String::from(" Checks ")
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(left, cx);
    let selected = if shown.is_empty() {
        0
    } else {
        step % shown.len()
    };
    for (row, (name, took)) in (0_u16..).zip(shown) {
        let y = left.y + 2 + row;
        let picked = usize::from(row) == selected && mode != 1;
        let style = if picked {
            Style::new().fg(fill.fg).bg(raised)
        } else {
            fill
        };
        cx.put_str(
            left.x + 1,
            y,
            &" ".repeat(usize::from(left.width - 2)),
            style,
        );
        cx.put_str(left.x + 2, y, &format!("✔ {name:<14}{took:>20}"), style);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Release notes ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(right, cx);
    let (name, took) = shown.get(selected).copied().unwrap_or(("none", ""));
    for (row, line) in (0_u16..).zip([
        format!("check      {name}"),
        format!("duration   {took}"),
        String::from("branch     main"),
        String::from("head       9c2b269"),
        String::new(),
        String::from("Changes since v0.1.12"),
        String::from("  • ToastStack, Canvas, SplitTree widgets"),
        String::from("  • Breadcrumbs and TabsDrag"),
        String::from("  • No-feature build fixed"),
    ]) {
        cx.put_str(
            right.x + 2,
            right.y + 2 + row,
            clip_display_prefix(&line, right.width.saturating_sub(4)),
            if row == 5 { fill.fg(accent) } else { fill },
        );
    }
}

fn kh_log(cx: &mut RenderCx<'_>, area: Rect, tick: u32, styles: [Style; 2], colors: [Color; 2]) {
    let [fill, dim] = styles;
    let [accent, _] = colors;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Run log ")
        .style(fill)
        .border_style(dim)
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(area, cx);
    let steps = [
        "checkout main",
        "cargo fmt --check",
        "cargo clippy -D warnings",
        "cargo test --locked",
        "cargo deny check",
        "cargo xtask bench",
        "build release artifacts",
        "sign checksums",
    ];
    let rows = usize::from(area.height.saturating_sub(2));
    let count = usize::try_from(tick / 6).unwrap_or(0) + 1;
    let start = count.saturating_sub(rows);
    for (row, index) in (0_u16..).zip(start..count) {
        let step = steps[index % steps.len()];
        let line = format!(
            "12:{:02}:{:02}  ok   {step}",
            (index * 7 / 60) % 60,
            index * 7 % 60
        );
        cx.put_str(
            area.x + 2,
            area.y + 1 + row,
            clip_display_prefix(&line, area.width.saturating_sub(4)),
            if index + 1 == count { fill } else { dim },
        );
    }
}

fn kh_confirm(cx: &mut RenderCx<'_>, area: Rect, styles: [Style; 2], accent: Color) {
    let [fill, dim] = styles;
    let modal = Rect::new(
        area.x + area.width / 2 - 22,
        area.y + area.height / 2 - 3,
        44,
        6,
    );
    for row in 0..modal.height {
        cx.put_str(modal.x, modal.y + row, &" ".repeat(44), fill);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Publish ")
        .style(fill)
        .border_style(Style::new().fg(accent).bg(fill.bg))
        .title_style(Style::new().fg(accent).bg(fill.bg))
        .render(modal, cx);
    cx.put_str(
        modal.x + 2,
        modal.y + 2,
        "Tag and publish lunatui v0.1.13?",
        fill,
    );
    cx.put_str(
        modal.x + 2,
        modal.y + 3,
        "All eight checks are passing.",
        dim,
    );
}

fn key_hints_baseline(buffer: &mut Buffer, area: RatRect, _theme: &Theme, tick: u32) {
    if let Some(case) = CASES.iter().find(|case| case.name == "key-hints") {
        (case.ratatui)(buffer, area, tick);
    }
}

const OV_SERVICES: [(&str, &str, &str); 8] = [
    ("api", "v0.1.13", "healthy"),
    ("web", "v0.1.13", "healthy"),
    ("jobs", "v0.1.13", "degraded"),
    ("edge", "v0.1.12", "healthy"),
    ("docs", "v0.1.13", "healthy"),
    ("search", "v0.1.13", "healthy"),
    ("billing", "v0.1.12", "healthy"),
    ("auth", "v0.1.13", "healthy"),
];

const OV_ACTIVITY: [&str; 9] = [
    "12:04  deploy   web v0.1.13 finished",
    "12:02  alert    jobs queue depth above 400",
    "11:58  deploy   api v0.1.13 finished",
    "11:51  scale    edge 4 -> 6 instances",
    "11:47  deploy   docs v0.1.13 finished",
    "11:40  rotate   auth signing keys",
    "11:33  deploy   search v0.1.13 finished",
    "11:20  backup   billing snapshot stored",
    "11:02  deploy   billing v0.1.12 finished",
];

fn ov_phase(tick: u32) -> Option<(usize, f32, bool)> {
    let (first, second) = ((24_u32, 100_u32), (124_u32, 204_u32));
    let (which, start, end) = if (first.0..first.1 + 12).contains(&tick) {
        (0, first.0, first.1)
    } else if (second.0..second.1 + 12).contains(&tick) {
        (1, second.0, second.1)
    } else {
        return None;
    };
    let fade = if tick < start + 12 {
        f32::from(u8::try_from(tick - start).unwrap_or(0)) / 12.0
    } else if tick >= end {
        1.0 - f32::from(u8::try_from(tick - end).unwrap_or(12).min(12)) / 12.0
    } else {
        1.0
    };
    let confirm = tick >= start + 46;
    Some((which, fade * 0.6, confirm))
}

fn overlay_native(
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
    cx.put_str(
        area.x,
        area.y,
        &" ".repeat(usize::from(area.width)),
        Style::new()
            .fg(accent)
            .bg(raised)
            .with_attrs(Attrs::new().set_bold(true)),
    );
    cx.put_str(
        area.x + 1,
        area.y,
        "Operations  ·  production",
        Style::new()
            .fg(accent)
            .bg(raised)
            .with_attrs(Attrs::new().set_bold(true)),
    );
    let left = Rect::new(area.x + 2, area.y + 2, 52, area.height.saturating_sub(5));
    let right = Rect::new(
        left.x + left.width + 1,
        left.y,
        area.width.saturating_sub(left.width + 5),
        left.height,
    );
    for (title, pane) in [(" Services ", left), (" Activity ", right)] {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(fill)
            .border_style(dim)
            .title_style(Style::new().fg(accent).bg(surface))
            .render(pane, &mut cx);
    }
    cx.put_str(
        left.x + 2,
        left.y + 1,
        "Name           Version      State",
        dim,
    );
    for (row, (name, version, state)) in (0_u16..).zip(OV_SERVICES) {
        let style = if state == "degraded" {
            fill.fg(theme.resolve(Token::Warn))
        } else {
            fill
        };
        cx.put_str(
            left.x + 2,
            left.y + 3 + row,
            &format!("{name:<15}{version:<13}{state}"),
            style,
        );
    }
    for (row, line) in (0_u16..).zip(OV_ACTIVITY) {
        cx.put_str(
            right.x + 2,
            right.y + 1 + row,
            clip_display_prefix(line, right.width.saturating_sub(4)),
            if row == 0 { fill } else { dim },
        );
    }
    StatusBar::new(" r rollback  ●  d delete previews  ●  q quit")
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
    if let Some((which, alpha, confirm)) = ov_phase(tick) {
        ov_modal(
            &mut cx,
            area,
            which,
            alpha,
            confirm,
            [fill, dim],
            [accent, raised],
        );
    }
}

fn ov_modal(
    cx: &mut RenderCx<'_>,
    area: Rect,
    which: usize,
    alpha: f32,
    confirm: bool,
    styles: [Style; 2],
    colors: [Color; 2],
) {
    let [fill, dim] = styles;
    let [accent, raised] = colors;
    let (width, height, title) = if which == 0 {
        (46_u16, 9_u16, " Roll back web? ")
    } else {
        (60_u16, 11_u16, " Delete preview environments? ")
    };
    let modal = Rect::new(
        area.x + area.width / 2 - width / 2,
        area.y + area.height / 2 - height / 2,
        width,
        height,
    );
    let body = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(fill.bg(raised))
        .border_style(Style::new().fg(accent).bg(raised))
        .title_style(Style::new().fg(accent).bg(raised));
    Overlay::new(body, modal)
        .backdrop(alpha, Color::Rgb(0, 0, 0))
        .shadow(Style::new().bg(Color::Rgb(6, 7, 12)))
        .render(area, cx);
    let lines: &[&str] = if which == 0 {
        &[
            "web goes back from v0.1.13 to v0.1.12.",
            "Open sessions stay connected.",
        ]
    } else {
        &[
            "pr-182  pr-185  pr-191  pr-193",
            "These previews are older than 7 days.",
            "Their databases are removed too.",
        ]
    };
    let content = fill.bg(raised);
    for (row, line) in (0_u16..).zip(lines) {
        cx.put_str(modal.x + 3, modal.y + 2 + row, line, content);
    }
    let (yes, no) = if which == 0 {
        ("Roll back", "Cancel")
    } else {
        ("Delete 4", "Keep")
    };
    let focus = Style::new()
        .fg(raised)
        .bg(accent)
        .with_attrs(Attrs::new().set_bold(true));
    let (yes_style, no_style) = if confirm {
        (focus, dim.bg(raised))
    } else {
        (dim.bg(raised), focus)
    };
    let y = modal.y + modal.height - 3;
    cx.put_str(modal.x + 3, y, &format!(" {yes} "), yes_style);
    cx.put_str(
        modal.x + 5 + u16::try_from(yes.len()).unwrap_or(0) + 3,
        y,
        &format!(" {no} "),
        no_style,
    );
}

fn overlay_baseline(_buffer: &mut Buffer, _area: RatRect, _theme: &Theme, _tick: u32) {}

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
