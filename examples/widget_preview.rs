// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::widget_suite::{CASES, Case, pin_color_output};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, clip_display_prefix};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Profile, Theme, Token, Variant};
use lunatui_widgets::{Block, BorderType, Date, Monthly, RenderCx, Widget, WidgetState};
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

const PREVIEWS: &[Preview] = &[Preview {
    name: "calendar",
    native: calendar_native,
    baseline: calendar_baseline,
}];

fn preview(name: &str) -> Option<&'static Preview> {
    PREVIEWS.iter().find(|preview| preview.name == name)
}

fn state_at(tick: u32) -> WidgetState {
    match (tick / 80) % 3 {
        0 => WidgetState::Normal,
        1 => WidgetState::Focused,
        _ => WidgetState::Disabled,
    }
}

fn state_name(state: WidgetState) -> &'static str {
    if state == WidgetState::Focused {
        "focused"
    } else if state == WidgetState::Disabled {
        "disabled"
    } else {
        "normal"
    }
}

fn border_for(state: WidgetState) -> BorderType {
    if state == WidgetState::Focused {
        BorderType::Thick
    } else {
        BorderType::Rounded
    }
}

fn month_on(year: i32, month: u8, day: u8) -> Date {
    Date::from_ymd(year, month, day).unwrap_or(Date::UNIX_EPOCH)
}

fn shift_month(month: u8, delta: i32) -> (i32, u8) {
    let shifted = i32::from(month) - 1 + delta;
    let year = 2026 + shifted.div_euclid(12);
    let index = shifted.rem_euclid(12);
    (year, u8::try_from(index).unwrap_or(0) + 1)
}

fn paint_month(
    cx: &mut RenderCx<'_>,
    area: Rect,
    year: i32,
    month: u8,
    day: u8,
    state: WidgetState,
    title: &str,
) {
    if area.width < 8 || area.height < 6 {
        return;
    }
    let text = cx.token(Token::TextPrimary);
    let accent = cx.token(Token::Accent);
    let surface = cx.token(Token::Surface0);
    let raised = cx.token(Token::Surface1);
    let focus = cx.token(Token::FocusRing);
    let ok = cx.token(Token::Ok);
    let warn = cx.token(Token::Warn);
    let ground = Style::new().fg(text).bg(surface);
    let tone = match state {
        WidgetState::Focused => focus,
        WidgetState::Disabled => raised,
        _ => accent,
    };
    let frame = Block::bordered()
        .border_type(border_for(state))
        .title(format!(" {title} "))
        .style(ground)
        .border_style(Style::new().fg(tone).bg(surface))
        .title_style(Style::new().fg(tone).bg(surface));
    frame.render(area, cx);
    let inner = frame.inner(area);
    let events = [
        (month_on(year, month, 3), Style::new().fg(warn)),
        (month_on(year, month, 14), Style::new().fg(ok)),
        (month_on(year, month, 26), Style::new().fg(accent)),
    ];
    Monthly::new(month_on(year, month, day), events.as_slice())
        .state(state)
        .render(inner, cx);
}

fn calendar_native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let text = cx.token(Token::TextPrimary);
    let surface = cx.token(Token::Surface0);
    let ground = Style::new().fg(text).bg(surface);
    cx.set_style(area, ground);
    let live = state_at(tick);
    let day = u8::try_from((tick / 4) % 28).unwrap_or(0) + 1;
    let (year, month) = (2026, 10);
    if area.width < 70 || area.height < 20 {
        paint_month(&mut cx, area, year, month, day, live, state_name(live));
        return;
    }
    let main_h = 16.min(area.height.saturating_sub(14));
    paint_month(
        &mut cx,
        Rect::new(1, 0, 36, main_h),
        year,
        month,
        day,
        live,
        &format!("October  {day}  {}", state_name(live)),
    );
    let note = format!("release window   day {day:>2}   on-call rotates");
    cx.put_str(
        40,
        2,
        clip_display_prefix(&note, area.width.saturating_sub(42)),
        Style::new().fg(text).bg(surface),
    );
    let states = [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ];
    let top = main_h.saturating_add(1);
    let panel_h = area.height.saturating_sub(top).saturating_sub(1);
    let gap = 2_u16;
    let panel_w = area.width.saturating_sub(2).saturating_sub(gap * 2) / 3;
    for (index, state) in states.iter().enumerate() {
        let (panel_year, panel_month) = shift_month(month, i32::try_from(index).unwrap_or(0) - 1);
        let x = 1 + u16::try_from(index).unwrap_or(0) * (panel_w + gap);
        paint_month(
            &mut cx,
            Rect::new(x, top, panel_w, panel_h),
            panel_year,
            panel_month,
            day,
            *state,
            state_name(*state),
        );
    }
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
