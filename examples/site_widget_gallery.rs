// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::env;
use std::io::{self, Write};
use std::time::Duration;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style, text_display_width};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Alignment, BigText, BigTextFont, Block, BorderType, Constraint, Date, Gauge, Lightcycle, List,
    Monthly, Padding, Paragraph, RenderCx, Scrollbar, Sparkline, SparklineBar, Spinner,
    SpinnerPattern, StatusBar, StatusDot, StatusState, Table, Tabs, TextInput, TextWrap, VecSource,
    WeekStart, Widget,
};

const WIDTH: u16 = 64;
const HEIGHT: u16 = 24;
const BG: Color = Color::Rgb(10, 13, 28);
const PANEL: Color = Color::Rgb(17, 22, 49);
const PANEL_RAISED: Color = Color::Rgb(22, 28, 61);
const LINE: Color = Color::Rgb(46, 53, 96);
const LINE_HI: Color = Color::Rgb(143, 124, 224);
const LAV: Color = Color::Rgb(185, 167, 240);
const LAV2: Color = Color::Rgb(143, 124, 224);
const INK: Color = Color::Rgb(236, 231, 218);
const MUTED: Color = Color::Rgb(170, 176, 204);
const DIM: Color = Color::Rgb(90, 97, 120);
const MOON: Color = Color::Rgb(243, 226, 176);
const GREEN: Color = Color::Rgb(155, 227, 168);
const RED: Color = Color::Rgb(242, 160, 160);
const BLUE: Color = Color::Rgb(143, 184, 240);
const FOCUS_BG: Color = Color::Rgb(42, 35, 87);

fn theme() -> Theme {
    Theme::with_overrides(
        Variant::Dark,
        &[
            (Token::Surface0, BG),
            (Token::Surface1, PANEL),
            (Token::TextPrimary, INK),
            (Token::Accent, LAV),
            (Token::Ok, GREEN),
            (Token::Warn, MOON),
            (Token::Fail, RED),
            (Token::FocusRing, LINE_HI),
        ],
    )
}

fn style(color: Color) -> Style {
    Style::new().fg(color).bg(BG)
}

fn bold(color: Color) -> Style {
    style(color).with_attrs(Attrs::new().set_bold(true))
}

fn put(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, color: Color) {
    cx.put_str(x, y, text, style(color));
}

fn put_bold(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, color: Color) {
    cx.put_str(x, y, text, bold(color));
    let width = u16::try_from(text_display_width(text)).unwrap_or_default();
    cx.set_style(Rect::new(x, y, width, 1), bold(color));
}

fn put_on(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, color: Color, background: Color) {
    cx.put_str(x, y, text, Style::new().fg(color).bg(background));
}

fn put_on_bold(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, color: Color, background: Color) {
    let style = Style::new()
        .fg(color)
        .bg(background)
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(x, y, text, style);
    let width = u16::try_from(text_display_width(text)).unwrap_or_default();
    cx.set_style(Rect::new(x, y, width, 1), style);
}

fn fill(cx: &mut RenderCx<'_>, rect: Rect, color: Color) {
    let blank = " ".repeat(usize::from(rect.width));
    for y in rect.y..rect.y.saturating_add(rect.height) {
        cx.put_str(rect.x, y, &blank, Style::new().bg(color));
    }
}

fn spark_glyph(level: f32) -> char {
    if level >= 8.0 {
        '█'
    } else if level >= 7.0 {
        '▇'
    } else if level >= 6.0 {
        '▆'
    } else if level >= 5.0 {
        '▅'
    } else if level >= 4.0 {
        '▄'
    } else if level >= 3.0 {
        '▃'
    } else if level >= 2.0 {
        '▂'
    } else if level >= 1.0 {
        '▁'
    } else {
        ' '
    }
}

fn framed(cx: &mut RenderCx<'_>, title: &str) -> Rect {
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(Style::new().fg(INK).bg(BG))
        .border_style(Style::new().fg(LINE).bg(BG));
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    frame.render(area, cx);
    put_bold(cx, 2, 0, &format!(" ☾ {title} "), LAV);
    frame.inner(area)
}

fn panel(title: &str, accent: Color) -> Block {
    let title = if title.is_empty() {
        String::new()
    } else {
        format!("─ {title} ─")
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(Style::new().bg(BG))
        .border_style(style(accent))
        .title(title)
        .title_style(style(accent))
}

#[expect(
    clippy::expect_used,
    reason = "The fixed gallery dates are valid calendar examples."
)]
fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_ymd(year, month, day).expect("gallery date constants are valid")
}

#[expect(
    clippy::too_many_lines,
    reason = "The gallery dispatch keeps each isolated widget scene together."
)]
fn render_case(name: &str, grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme) -> bool {
    grid.fill(
        Rect::new(0, 0, WIDTH, HEIGHT),
        Style::new().fg(INK).bg(BG),
        ' ',
        clusters,
    );
    let mut cx = RenderCx::new(grid, clusters, theme);
    let _body = framed(
        &mut cx,
        match name {
            "bigtext" => "BigText",
            "block" => "Block",
            "calendar" => "Calendar",
            "gauge" => "Gauge",
            "lightcycle" => "Lightcycle",
            "list" => "List",
            "overlay" => "Overlay",
            "paragraph" => "Paragraph",
            "padding" => "Padding",
            "scrollbar" => "Scrollbar",
            "sparkline" => "Sparkline",
            "spinner" => "Spinner",
            "statusbar" => "Status bar",
            "statusdot" => "StatusDot",
            "table" => "Table",
            "tabs" => "Tabs",
            "textinput" => "Text input",
            "borders" => "Borders",
            _ => return false,
        },
    );
    match name {
        "bigtext" => {
            BigText::new("23:47")
                .font(BigTextFont::FiveBySeven)
                .row_styles(&[
                    style(Color::Rgb(220, 210, 249)),
                    style(Color::Rgb(208, 195, 246)),
                    style(Color::Rgb(196, 181, 243)),
                    style(Color::Rgb(185, 167, 240)),
                    style(Color::Rgb(170, 151, 234)),
                    style(Color::Rgb(156, 136, 229)),
                    style(Color::Rgb(143, 124, 224)),
                ])
                .alignment(Alignment::Left)
                .render(Rect::new(5, 2, 56, 7), &mut cx);
            put_bold(&mut cx, 56, 8, ":12", MOON);
            put_bold(&mut cx, 5, 11, "Tuesday", INK);
            put(&mut cx, 13, 11, "· 27 October 2026", MUTED);
            put(&mut cx, 5, 12, "Waning gibbous · 94% illuminated", DIM);
            put(&mut cx, 2, 15, &"─".repeat(60), LINE);
            let cities = [
                ("UTC", "05:47", "+6h"),
                ("Tokyo", "14:47", "+15h"),
                ("Berlin", "06:47", "+7h"),
                ("San Francisco", "22:47", "-1h"),
            ];
            for (i, (city, time, offset)) in cities.into_iter().enumerate() {
                let x = 4 + u16::try_from(i % 2).unwrap_or_default() * 30;
                let y = 17 + u16::try_from(i / 2).unwrap_or_default() * 2;
                put(&mut cx, x, y, city, MUTED);
                put_bold(&mut cx, x + 15, y, time, if i == 0 { LAV } else { INK });
                let offset_width = u16::try_from(offset.chars().count()).unwrap_or_default();
                put(&mut cx, x + 25 - offset_width, y, offset, DIM);
            }
            put(&mut cx, 4, 21, "▕", LINE);
            put(&mut cx, 5, 21, &"━".repeat(35), LAV2);
            put(&mut cx, 40, 21, &"─".repeat(13), LINE);
            put(&mut cx, 54, 21, "74%", DIM);
        }
        "block" => {
            put(&mut cx, 5, 3, "A calm frame for terminal content", MUTED);
            let block = panel("Deployment", LAV).padding(Padding {
                left: 2,
                right: 2,
                top: 1,
                bottom: 1,
            });
            block.render(Rect::new(5, 6, 54, 12), &mut cx);
            put_on(
                &mut cx,
                9,
                9,
                "lunatui-web  ·  production",
                INK,
                PANEL_RAISED,
            );
            put_on(
                &mut cx,
                9,
                11,
                "Build finished in 2.4 seconds",
                MUTED,
                PANEL_RAISED,
            );
            put_on(&mut cx, 9, 13, "3 regions healthy", GREEN, PANEL_RAISED);
        }
        "calendar" => {
            let october_style = bold(MOON);
            let november_style = bold(MUTED);
            let weekday_style = style(DIM);
            let day_style = style(INK);
            let october_events = [
                (date(2_026, 10, 4), style(LAV2)),
                (date(2_026, 10, 10), style(LAV2)),
                (date(2_026, 10, 11), style(LAV2)),
                (date(2_026, 10, 17), style(LAV2)),
                (date(2_026, 10, 18), style(LAV2)),
                (date(2_026, 10, 24), style(LAV2)),
                (date(2_026, 10, 25), style(LAV2)),
                (date(2_026, 10, 27), bold(BG).bg(LAV)),
                (date(2_026, 10, 3), bold(LAV)),
                (date(2_026, 10, 14), bold(LAV)),
                (date(2_026, 10, 31), bold(LAV)),
            ];
            Monthly::new(date(2_026, 10, 27), &october_events[..])
                .week_start(WeekStart::Monday)
                .event_markers(false)
                .show_month_header(october_style)
                .show_weekdays_header(weekday_style)
                .default_style(day_style)
                .render(Rect::new(5, 2, 21, 8), &mut cx);
            let november_events = [
                (date(2_026, 11, 1), style(LAV2)),
                (date(2_026, 11, 7), style(LAV2)),
                (date(2_026, 11, 8), style(LAV2)),
                (date(2_026, 11, 14), style(LAV2)),
                (date(2_026, 11, 15), style(LAV2)),
                (date(2_026, 11, 21), style(LAV2)),
                (date(2_026, 11, 22), style(LAV2)),
                (date(2_026, 11, 28), style(LAV2)),
                (date(2_026, 11, 29), style(LAV2)),
                (date(2_026, 11, 5), bold(LAV)),
                (date(2_026, 11, 18), bold(LAV)),
                (date(2_026, 11, 24), bold(LAV)),
            ];
            Monthly::new(date(2_026, 11, 1), &november_events[..])
                .week_start(WeekStart::Monday)
                .event_markers(false)
                .show_month_header(november_style)
                .show_weekdays_header(weekday_style)
                .default_style(day_style)
                .render(Rect::new(36, 2, 21, 8), &mut cx);
            put_bold(&mut cx, 10, 2, "October 2026", MOON);
            put(&mut cx, 9, 2, " ", BG);
            put_bold(&mut cx, 40, 2, "November 2026", MUTED);
            put_on_bold(&mut cx, 9, 8, "27", BG, LAV);
            fill(&mut cx, Rect::new(8, 8, 1, 1), LAV);
            fill(&mut cx, Rect::new(11, 8, 1, 1), LAV);
            put_bold(&mut cx, 40, 8, "24", LAV);
            for y in 2..9 {
                put(&mut cx, 31, y, "│", LINE);
            }
            put(&mut cx, 2, 10, &"─".repeat(60), LINE);
            put_bold(&mut cx, 3, 11, "Upcoming", LAV);
            let events = [
                ("Sat 31 Oct", "●", LAV, "Release v0.9 tagged", "20:00"),
                ("Thu 05 Nov", "●", MOON, "Docs freeze", "09:30"),
                ("Wed 18 Nov", "●", GREEN, "Community call", "17:00"),
                ("Tue 24 Nov", "○", DIM, "Benchmark refresh", "—"),
            ];
            for (index, (day, dot, color, title, time)) in events.into_iter().enumerate() {
                let y = 13 + u16::try_from(index).unwrap_or_default() * 2;
                let selected = index == 0;
                if selected {
                    fill(&mut cx, Rect::new(2, y, 60, 1), FOCUS_BG);
                    put_on(&mut cx, 2, y, "▌", LAV, FOCUS_BG);
                }
                let background = if selected { FOCUS_BG } else { BG };
                put_on(&mut cx, 4, y, day, MUTED, background);
                put_on(&mut cx, 16, y, dot, color, background);
                if selected {
                    put_on_bold(&mut cx, 18, y, title, INK, background);
                } else {
                    put_on(&mut cx, 18, y, title, INK, background);
                }
                let time_x = WIDTH
                    .saturating_sub(4)
                    .saturating_sub(u16::try_from(time.chars().count()).unwrap_or_default());
                put_on(
                    &mut cx,
                    time_x,
                    y,
                    time,
                    if selected { LAV } else { DIM },
                    background,
                );
            }
            put(&mut cx, 3, 21, "Today", DIM);
            put_bold(&mut cx, 9, 21, "Tue 27 Oct", MOON);
            put(&mut cx, 21, 21, "· 4 upcoming events", DIM);
        }
        "gauge" => {
            let stages = [
                (
                    "Compiling lunatui",
                    0.72,
                    LAV,
                    "crate 37/51 · lunatui-widgets · 4.1s elapsed",
                ),
                (
                    "Downloading assets",
                    0.43,
                    MOON,
                    "18.6 MiB of 43.2 MiB · 6.2 MiB/s",
                ),
                ("Tests passed", 0.96, GREEN, "482 of 502 · 20 remaining"),
            ];
            for (index, (label, progress, color, detail)) in stages.into_iter().enumerate() {
                let y = 2 + u16::try_from(index).unwrap_or_default() * 4;
                if index == 0 {
                    put(&mut cx, 2, y, "▌", LAV);
                }
                if index == 0 {
                    put_bold(&mut cx, 4, y, label, INK);
                } else {
                    put(&mut cx, 4, y, label, MUTED);
                }
                put_bold(
                    &mut cx,
                    56,
                    y,
                    &format!("{:>3.0}%", progress * 100.0),
                    color,
                );
                fill(&mut cx, Rect::new(4, y + 1, 56, 1), PANEL_RAISED);
                Gauge::new(progress)
                    .gauge_style(style(color).bg(PANEL_RAISED))
                    .render(Rect::new(4, y + 1, 56, 1), &mut cx);
                let ticks = progress * 56.0;
                let whole = u16::try_from(
                    (0_u16..56)
                        .take_while(|cell| f32::from(*cell + 1) <= ticks)
                        .count(),
                )
                .unwrap_or_default();
                let partial = if ticks.fract() == 0.0 {
                    0
                } else {
                    (1_u8..=7)
                        .take_while(|eighth| f32::from(*eighth) / 8.0 <= ticks.fract())
                        .last()
                        .unwrap_or(1)
                };
                if partial > 0 {
                    let glyphs = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"];
                    put_on(
                        &mut cx,
                        4 + whole,
                        y + 1,
                        glyphs[usize::from(partial - 1)],
                        color,
                        PANEL_RAISED,
                    );
                }
                put(&mut cx, 4, y + 2, detail, DIM);
            }
            put(&mut cx, 2, 14, &"─".repeat(60), LINE);
            let resources = [
                ("CPU", LAV, 34_u16),
                ("GPU", MOON, 18_u16),
                ("Disk", GREEN, 61_u16),
                ("Net", BLUE, 27_u16),
            ];
            for (index, (label, color, percent)) in resources.into_iter().enumerate() {
                let y = 16 + u16::try_from(index).unwrap_or_default() * 2;
                put(&mut cx, 4, y, &format!("{label:<5}"), MUTED);
                let filled = usize::from((percent * 22).div_ceil(100));
                put(&mut cx, 10, y, &"━".repeat(filled), color);
                put(
                    &mut cx,
                    10 + u16::try_from(filled).unwrap_or_default(),
                    y,
                    &"─".repeat(22 - filled),
                    LINE,
                );
                put(&mut cx, 33, y, &format!("{percent:>3}%"), DIM);
            }
            let week = [0.4_f32, 0.7, 0.55, 0.9, 0.65, 0.3, 0.8];
            for (index, value) in week.into_iter().enumerate() {
                let height = value * 6.0 * 8.0;
                let x = 42 + u16::try_from(index).unwrap_or_default() * 3;
                let color = if index == 3 { LAV } else { LINE_HI };
                for row in 0..6 {
                    let level = height - f32::from(5 - row) * 8.0;
                    let glyph = spark_glyph(level);
                    put_on(
                        &mut cx,
                        x,
                        16 + row,
                        &glyph.to_string().repeat(2),
                        color,
                        PANEL_RAISED,
                    );
                }
            }
            put(&mut cx, 42, 22, "M  T  W  T  F  S  S", DIM);
        }
        "lightcycle" => {
            put_bold(&mut cx, 5, 3, "Signal trail", INK);
            put(
                &mut cx,
                5,
                5,
                "Braille detail · 60 frames per second",
                MUTED,
            );
            for (elapsed, head, tail) in [(0, LAV, BLUE), (1_412, MOON, LAV), (2_824, GREEN, BLUE)]
            {
                Lightcycle::new(Duration::from_millis(elapsed))
                    .period(Duration::from_secs(4))
                    .trail(bold(head), style(tail))
                    .render(Rect::new(20, 9, 24, 10), &mut cx);
            }
        }
        "list" => {
            let names = [
                "Calendar",
                "BigText",
                "Sparkline",
                "Gauge",
                "Table",
                "List",
                "Tabs",
                "Text input",
                "Spinner",
                "Status bar",
                "Overlay",
                "Canvas",
                "Scrollbar",
            ];
            let source = VecSource::from_rows(
                names
                    .into_iter()
                    .map(|name| vec![name.to_owned()])
                    .collect(),
            );
            panel("Widgets", LINE_HI).render(Rect::new(2, 1, 28, 22), &mut cx);
            for index in 0..names.len() {
                let y = 3 + u16::try_from(index).unwrap_or_default();
                if index == 3 {
                    fill(&mut cx, Rect::new(3, y, 26, 1), FOCUS_BG);
                    put_on(&mut cx, 3, y, "▌", LAV, FOCUS_BG);
                }
            }
            List::new(source)
                .selected(Some(3))
                .style(style(MUTED))
                .highlight_style(Style::new().fg(INK).bg(FOCUS_BG))
                .render(Rect::new(7, 3, 18, 13), &mut cx);
            let icons = [
                LAV, LAV, MOON, GREEN, BLUE, LAV, MOON, GREEN, BLUE, LAV, MOON, GREEN, BLUE,
            ];
            for (index, color) in icons.into_iter().enumerate() {
                let y = 3 + u16::try_from(index).unwrap_or_default();
                let background = if index == 3 { FOCUS_BG } else { BG };
                put_on(&mut cx, 5, y, "◆", color, background);
                if matches!(index, 0 | 3 | 5) {
                    put_on(&mut cx, 25, y, "✓", GREEN, background);
                }
                if index == 3 {
                    cx.set_style(
                        Rect::new(7, y, 5, 1),
                        Style::new()
                            .fg(INK)
                            .bg(FOCUS_BG)
                            .with_attrs(Attrs::new().set_bold(true)),
                    );
                }
            }
            put_bold(&mut cx, 5, 1, "Widgets", LAV);
            put_bold(&mut cx, 34, 1, "Preview", MUTED);
            put(&mut cx, 3, 1, "─", LINE_HI);
            put(&mut cx, 32, 1, "─", LINE);
            put(&mut cx, 12, 1, " ", BG);
            put(&mut cx, 41, 1, " ", BG);
            Scrollbar::vertical(13, 0, 0)
                .track_style(style(LINE))
                .thumb_style(style(LAV2))
                .render(Rect::new(28, 4, 1, 16), &mut cx);
            cx.set_style(Rect::new(28, 6, 1, 1), Style::new().fg(LINE).bg(FOCUS_BG));
            put(&mut cx, 32, 10, &"─".repeat(29), LINE);
            put(&mut cx, 28, 20, "│", LINE);
            put_on(&mut cx, 4, 20, "13 items · 3 selected", DIM, BG);
            panel("Preview", LINE).render(Rect::new(31, 1, 31, 22), &mut cx);
            put_on_bold(&mut cx, 33, 3, "Gauge", MOON, BG);
            put_on(&mut cx, 33, 4, "Progress with sub-cell", MUTED, BG);
            put_on(&mut cx, 33, 5, "precision (1/8 cell).", MUTED, BG);
            fill(&mut cx, Rect::new(33, 7, 26, 1), PANEL_RAISED);
            Gauge::new(0.66)
                .gauge_style(style(LAV).bg(PANEL_RAISED))
                .render(Rect::new(33, 7, 26, 1), &mut cx);
            put_on(&mut cx, 50, 7, "▋", LAV, PANEL_RAISED);
            put_on(&mut cx, 33, 8, "66%  ·  eased 250ms", DIM, BG);
            put_on_bold(&mut cx, 33, 11, "Options", LAV, BG);
            put_on(&mut cx, 33, 13, "label", MUTED, BG);
            put_on(&mut cx, 46, 13, "\"Compiling\"", INK, BG);
            put_on(&mut cx, 33, 14, "ratio", MUTED, BG);
            put_on(&mut cx, 46, 14, "0.66", INK, BG);
            put_on(&mut cx, 33, 15, "style", MUTED, BG);
            put_on(&mut cx, 46, 15, "lav · bold", INK, BG);
            put_on(&mut cx, 33, 16, "use_unicode", MUTED, BG);
            put_on(&mut cx, 46, 16, "true", INK, BG);
            put_on(&mut cx, 33, 18, "Since", MUTED, BG);
            put_on(&mut cx, 46, 18, "v0.3", INK, BG);
            put_on(&mut cx, 33, 20, "↵ open docs", DIM, BG);
            put_bold(&mut cx, 34, 1, "Preview", MUTED);
            cx.set_style(Rect::new(28, 6, 1, 1), Style::new().fg(LINE).bg(FOCUS_BG));
            put(&mut cx, 32, 10, &"─".repeat(29), LINE);
            put(&mut cx, 28, 20, "│", LINE);
        }
        "overlay" => {
            put_bold(&mut cx, 3, 2, "Sessions", Color::Rgb(74, 79, 128));
            put_bold(&mut cx, 33, 2, "Activity", Color::Rgb(74, 79, 128));
            let sessions = [
                ("render-bench", "0.81 ms"),
                ("diff-bench", "0.19 ms"),
                ("scroll-10k", "1.44 ms"),
                ("layout-solve", "0.07 ms"),
                ("unicode-wide", "0.58 ms"),
                ("table-5k", "0.96 ms"),
                ("sparkline-1k", "0.22 ms"),
                ("resize-storm", "0.88 ms"),
                ("image-fallback", "2.31 ms"),
            ];
            for (index, (name, time)) in sessions.into_iter().enumerate() {
                let y = 4 + u16::try_from(index).unwrap_or_default() * 2;
                put(&mut cx, 3, y, name, Color::Rgb(47, 54, 92));
                put(&mut cx, 20, y, time, Color::Rgb(47, 54, 92));
            }
            let activity = [
                414_425, 475_604, 541_255, 633_338, 660_078, 732_902, 578_594, 582_700, 705_330,
                723_068, 795_785, 654_778, 636_098, 562_052, 572_259, 576_221, 433_479, 371_699,
                333_640, 451_441, 491_340, 369_777, 436_310, 326_216, 368_020, 285_884, 206_786,
                369_986,
            ];
            Sparkline::new(&activity)
                .max(1_000_000)
                .style(style(Color::Rgb(36, 42, 76)))
                .render(Rect::new(33, 10, 28, 8), &mut cx);
            fill(&mut cx, Rect::new(12, 7, 40, 12), Color::Rgb(4, 5, 11));
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::new().bg(PANEL_RAISED))
                .border_style(Style::new().fg(LINE_HI).bg(PANEL_RAISED))
                .render(Rect::new(11, 6, 40, 12), &mut cx);
            fill(&mut cx, Rect::new(12, 7, 38, 10), PANEL_RAISED);
            put_on(&mut cx, 12, 6, "─", LINE_HI, PANEL_RAISED);
            put_on_bold(&mut cx, 13, 6, " Quit Lunatui? ", MOON, PANEL_RAISED);
            put_on(&mut cx, 14, 8, "⚠", MOON, PANEL_RAISED);
            put_on(
                &mut cx,
                16,
                8,
                "You have 2 unsaved layouts.",
                INK,
                PANEL_RAISED,
            );
            put_on(
                &mut cx,
                16,
                10,
                "Changes since 05:31 will be lost",
                MUTED,
                PANEL_RAISED,
            );
            put_on(&mut cx, 16, 11, "if you quit now.", MUTED, PANEL_RAISED);
            fill(&mut cx, Rect::new(14, 14, 15, 1), LAV);
            put_on_bold(&mut cx, 14, 14, "  Save & quit  ", BG, LAV);
            fill(&mut cx, Rect::new(31, 14, 8, 1), FOCUS_BG);
            put_on(&mut cx, 31, 14, "  Quit  ", INK, FOCUS_BG);
            fill(&mut cx, Rect::new(41, 14, 8, 1), PANEL);
            put_on(&mut cx, 41, 14, " Cancel ", MUTED, PANEL);
            put_on(
                &mut cx,
                14,
                16,
                "tab cycle · ↵ confirm · esc close",
                DIM,
                PANEL_RAISED,
            );
        }
        "paragraph" => {
            let story = panel("Release notes", LINE_HI).padding(Padding {
                left: 3,
                right: 3,
                top: 1,
                bottom: 1,
            });
            story.render(Rect::new(5, 3, 54, 17), &mut cx);
            put_on(
                &mut cx,
                9,
                6,
                "A clear view of the work",
                MOON,
                PANEL_RAISED,
            );
            Paragraph::new("Lunatui keeps terminal content readable as it changes. Wrap long notes, align the details, and leave enough room for the next useful thing. Each line keeps its place as the window grows, so long release notes remain easy to scan.")
                .wrap(TextWrap { trim: true })
                .style(style(INK).bg(PANEL_RAISED))
                .render(Rect::new(9, 8, 46, 8), &mut cx);
            put_on(&mut cx, 9, 17, "NOTES  /  v0.9.2", DIM, PANEL_RAISED);
            put_on(&mut cx, 39, 17, "4 lines · 312 chars", DIM, PANEL_RAISED);
        }
        "padding" => {
            put_bold(
                &mut cx,
                5,
                3,
                "Comfortable spacing, without wasted space",
                LAV,
            );
            let compact = panel("Compact · 1 cell", LAV).padding(Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            });
            compact.render(Rect::new(5, 7, 26, 7), &mut cx);
            put_on(&mut cx, 8, 10, "moon-dash", INK, PANEL_RAISED);
            put_on(&mut cx, 8, 12, "12 dependencies", MUTED, PANEL_RAISED);
            let roomy = panel("Roomy · 3 cells", LINE_HI).padding(Padding {
                left: 3,
                right: 3,
                top: 1,
                bottom: 1,
            });
            roomy.render(Rect::new(33, 7, 26, 10), &mut cx);
            put_on(&mut cx, 38, 11, "moon-dash", INK, PANEL_RAISED);
            put_on(&mut cx, 38, 13, "Terminal dashboard", MUTED, PANEL_RAISED);
            put_on(&mut cx, 38, 15, "12 dependencies", DIM, PANEL_RAISED);
            put(&mut cx, 5, 19, "Same content, two useful rhythms.", DIM);
        }
        "scrollbar" => {
            let rows = [
                "Releases",
                "Render",
                "Themes",
                "Widgets",
                "Calendar",
                "BigText",
                "Sparkline",
                "Gauge",
                "Table",
                "Input",
                "Overlay",
                "Status",
            ];
            let source =
                VecSource::from_rows(rows.into_iter().map(|name| vec![name.to_owned()]).collect());
            panel("Component index", LINE_HI).render(Rect::new(4, 4, 31, 16), &mut cx);
            List::new(source)
                .offset(3)
                .selected(Some(5))
                .style(Style::new().fg(MUTED).bg(PANEL_RAISED))
                .highlight_style(Style::new().fg(INK).bg(FOCUS_BG))
                .render(Rect::new(6, 6, 25, 14), &mut cx);
            Scrollbar::vertical(24, 14, 8)
                .track_style(style(LINE))
                .thumb_style(style(LAV))
                .render(Rect::new(32, 6, 1, 14), &mut cx);
            panel("Selected · BigText", LAV).render(Rect::new(37, 4, 23, 16), &mut cx);
            put_on(&mut cx, 40, 8, "23:47", LAV, PANEL_RAISED);
            put_on(&mut cx, 40, 10, "Time, at a glance.", MUTED, PANEL_RAISED);
            put_on(&mut cx, 40, 13, "Render size", DIM, PANEL_RAISED);
            Gauge::new(0.66)
                .gauge_style(style(LAV).bg(PANEL_RAISED))
                .render(Rect::new(40, 15, 17, 1), &mut cx);
        }
        "sparkline" => {
            let frames = [
                713_223, 835_835, 898_375, 814_368, 805_422, 778_661, 797_904, 833_354, 680_749,
                548_490, 641_521, 603_807, 652_807, 503_643, 507_318, 580_490, 520_115, 658_423,
                754_724, 623_920, 542_017, 618_840, 777_185, 761_454, 716_036, 736_116, 655_672,
                642_736, 680_185, 712_731, 663_171, 617_522, 571_750, 586_778, 546_667, 445_795,
                564_632, 576_325, 603_729, 514_723,
            ];
            let render = [
                550_322, 596_779, 440_514, 325_861, 385_078, 396_911, 383_778, 296_150, 290_689,
                286_015, 279_327, 194_402, 196_085, 198_443, 275_998, 395_434, 480_849, 469_919,
                451_515, 410_644, 340_670, 292_541, 346_178, 351_578, 362_284, 464_563, 453_664,
                442_561, 358_994, 247_316, 219_525, 153_977, 176_386, 285_741, 296_065, 206_109,
                289_471, 334_844, 362_513, 426_176,
            ];
            let memory = [
                490_730, 473_097, 435_157, 425_880, 417_883, 355_244, 307_863, 361_448, 347_819,
                344_383, 428_118, 444_981, 504_298, 520_133, 556_601, 540_618, 582_953, 638_454,
                642_657, 664_545, 667_499, 601_259, 615_024, 597_520, 545_368, 471_108, 492_703,
                460_137, 455_535, 464_290, 452_567, 466_401, 427_458, 447_174, 433_725, 483_264,
                522_991, 483_577, 470_276, 478_374,
            ];
            let events = [
                253_602, 109_961, 128_271, 80_000, 80_000, 80_000, 323_579, 479_958, 586_201,
                453_601, 495_794, 428_664, 345_165, 262_672, 248_051, 520_186, 665_875, 751_379,
                799_464, 579_893, 461_842, 495_329, 550_863, 630_434, 689_881, 410_138, 420_588,
                455_587, 417_773, 266_419, 287_170, 157_856, 416_153, 580_239, 578_398, 487_460,
                674_851, 676_842, 805_115, 881_398,
            ];
            let tiles = [
                (2, 1, "Frames / s", "1,248", "fps", &frames[14..], LAV),
                (32, 1, "Render time", "0.81", "ms", &render[14..], MOON),
                (2, 7, "Memory", "14.2", "MiB", &memory[14..], GREEN),
                (32, 7, "Events", "312", "/s", &events[14..], BLUE),
            ];
            for (x, y, title, value, unit, data, color) in tiles {
                let accent = if title == "Frames / s" { LAV } else { MUTED };
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(BG))
                    .border_style(style(if title == "Frames / s" { LINE_HI } else { LINE }))
                    .render(Rect::new(x, y, 30, 6), &mut cx);
                put(
                    &mut cx,
                    x + 2,
                    y,
                    "─",
                    if title == "Frames / s" { LINE_HI } else { LINE },
                );
                put(&mut cx, x + 2, y, " ", BG);
                put_on_bold(&mut cx, x + 3, y, title, accent, BG);
                put_on_bold(&mut cx, x + 2, y + 1, value, color, BG);
                put_on(
                    &mut cx,
                    x + 2 + u16::try_from(value.len()).unwrap_or_default() + 1,
                    y + 1,
                    unit,
                    DIM,
                    BG,
                );
                Sparkline::new(data)
                    .max(1_000_000)
                    .style(style(color))
                    .render(Rect::new(x + 2, y + 2, 26, 3), &mut cx);
                let exact_rows: [&str; 3] = match title {
                    "Frames / s" => [
                        "    ▂   ▂▂▁▁  █▁          ",
                        "▄▅▄▇█▆▅▆████▇▇██▇▆▅▆▅▂▅▅▆▄",
                        "██████████████████████████",
                    ],
                    "Render time" => [
                        "                          ",
                        " ▁▃▃▂▁█ ███▃▂▂█        ██▂",
                        "▆██████▇███████▅▅▃▄▆▇▄▆███",
                    ],
                    "Memory" => [
                        "      █                   ",
                        "▅▄▅▇▇▇█▆▆▆▅▃▃▃▂▃▂▃▂▂▂▃▄▃▃▃",
                        "██████████████████████████",
                    ],
                    _ => [
                        "   ▂▃     █           ██▃▅",
                        " ▄▇██▅▃▃▅▇█▁▂▂▂   ▁▅▅▃████",
                        "▅██████████████▆▆▃████████",
                    ],
                };
                for (row, cells) in exact_rows.iter().enumerate() {
                    put(
                        &mut cx,
                        x + 2,
                        y + 2 + u16::try_from(row).unwrap_or_default(),
                        cells,
                        color,
                    );
                }
                let title_end = x + 3 + u16::try_from(title.chars().count()).unwrap_or_default();
                put(&mut cx, title_end, y, " ", BG);
            }
            let throughput = [
                575_988, 507_029, 608_622, 503_326, 563_269, 546_601, 433_340, 478_463, 358_150,
                382_047, 279_765, 206_019, 248_126, 394_278, 284_286, 238_467, 331_637, 499_321,
                513_897, 479_830, 640_577, 485_116, 630_297, 570_573, 492_760, 435_768, 455_981,
                622_625, 546_145, 607_808, 660_861, 608_953, 614_911, 462_705, 344_411, 295_489,
                394_944, 382_016, 334_575, 381_049, 374_850, 328_132, 449_896, 514_657, 433_215,
                485_947, 519_023, 657_699, 720_557, 639_961, 796_944, 651_447, 640_181, 731_844,
                609_722, 618_299,
            ];
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::new().bg(BG))
                .border_style(style(LINE))
                .render(Rect::new(2, 13, 60, 10), &mut cx);
            put(&mut cx, 3, 13, "─", LINE);
            put(&mut cx, 4, 13, " ", BG);
            put_on_bold(&mut cx, 5, 13, "Throughput · last 60s", MUTED, BG);
            put(&mut cx, 26, 13, " ", BG);
            Sparkline::new(&throughput)
                .max(1_000_000)
                .style(style(LAV))
                .row_styles(&[
                    style(LAV),
                    style(LAV),
                    style(LAV),
                    style(LAV2),
                    style(LAV2),
                    style(LAV2),
                    style(LAV2),
                ])
                .render(Rect::new(4, 14, 56, 7), &mut cx);
            let exact_rows = [
                "                                                        ",
                "                                                █ ▄  █  ",
                "█ ▂                 ▃ ▃    ▂ ▂▅▂▂              ▄█▃█▄▃█▂▂",
                "█▄█▄▇▆█▂         ▃▄▂█▃█▇▃█▁█▆████▁        ▁▄█▃▅█████████",
                "████████▄▅   ▆  ▂█████████████████▃█▆▅▂▅▄▂██████████████",
                "██████████▇▃▅█▇▅████████████████████████████████████████",
                "████████████████████████████████████████████████████████",
            ];
            for (row, cells) in exact_rows.iter().enumerate() {
                put(
                    &mut cx,
                    4,
                    14 + u16::try_from(row).unwrap_or_default(),
                    cells,
                    if row > 2 { LAV2 } else { LAV },
                );
            }
            put_on(&mut cx, 4, 21, "min 0.62k", DIM, BG);
            put_on(&mut cx, 26, 21, "avg 1.18k", DIM, BG);
            put_on(&mut cx, 49, 21, "max 1.31k", MOON, BG);
        }
        "spinner" => {
            put_bold(&mut cx, 3, 2, "Styles", LAV);
            let labels = [
                "Braille dots",
                "Half moon",
                "Quadrant",
                "Braille ring",
                "Arc",
                "Bars",
            ];
            for (index, label) in labels.into_iter().enumerate() {
                let x = 3 + u16::try_from(index % 3).unwrap_or_default() * 20;
                let y = 4 + u16::try_from(index / 3).unwrap_or_default() * 3;
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(BG))
                    .border_style(style(if index == 1 { LINE_HI } else { LINE }))
                    .render(Rect::new(x, y, 18, 3), &mut cx);
                Spinner::new(0)
                    .pattern(
                        [
                            SpinnerPattern::BrailleDots,
                            SpinnerPattern::HalfMoon,
                            SpinnerPattern::Quadrant,
                            SpinnerPattern::BrailleRing,
                            SpinnerPattern::Arc,
                            SpinnerPattern::Bars,
                        ][index],
                    )
                    .style(style([LAV, MOON, GREEN, BLUE, LAV, MOON][index]))
                    .render(
                        Rect::new(x + 2, y + 1, if index == 5 { 3 } else { 1 }, 1),
                        &mut cx,
                    );
                let glyph_width = if index == 5 { 3 } else { 1 };
                cx.set_style(
                    Rect::new(x + 2, y + 1, glyph_width, 1),
                    bold([LAV, MOON, GREEN, BLUE, LAV, MOON][index]),
                );
                let label_x = if index == 5 { x + 6 } else { x + 4 };
                put(
                    &mut cx,
                    label_x,
                    y + 1,
                    label,
                    if index == 1 { INK } else { MUTED },
                );
            }
            put(&mut cx, 2, 11, &"─".repeat(60), LINE);
            put_bold(&mut cx, 3, 12, "Tasks", LAV);
            let tasks = [
                ("✓", "Resolve dependencies", GREEN, "0.4s"),
                ("✓", "Compile lunatui-core", GREEN, "2.1s"),
                ("⣽", "Compile lunatui-widgets", LAV, "1.3s"),
                ("◐", "Generate docs", MOON, "…"),
                ("·", "Run examples", DIM, "queued"),
                ("·", "Package release", DIM, "queued"),
            ];
            for (index, (icon, task, color, time)) in tasks.into_iter().enumerate() {
                let y = 14 + u16::try_from(index).unwrap_or_default();
                let selected = index == 2;
                if selected {
                    fill(&mut cx, Rect::new(2, y, 60, 1), FOCUS_BG);
                    put_on(&mut cx, 2, y, "▌", LAV, FOCUS_BG);
                }
                let background = if selected { FOCUS_BG } else { BG };
                if selected {
                    Spinner::new(1)
                        .pattern(SpinnerPattern::BrailleRing)
                        .style(Style::new().fg(color).bg(background))
                        .render(Rect::new(4, y, 1, 1), &mut cx);
                    cx.set_style(Rect::new(4, y, 1, 1), bold(color).bg(background));
                } else {
                    put_on_bold(&mut cx, 4, y, icon, color, background);
                }
                if selected {
                    put_on_bold(&mut cx, 6, y, task, INK, background);
                } else {
                    put_on(
                        &mut cx,
                        6,
                        y,
                        task,
                        if index < 4 { INK } else { DIM },
                        background,
                    );
                }
                let time_x = WIDTH
                    .saturating_sub(4)
                    .saturating_sub(u16::try_from(time.chars().count()).unwrap_or_default());
                put_on(
                    &mut cx,
                    time_x,
                    y,
                    time,
                    if selected { LAV } else { DIM },
                    background,
                );
            }
            put(&mut cx, 4, 21, "2 of 6 done", MUTED);
            put(&mut cx, 17, 21, &"━".repeat(8), LAV);
            put(&mut cx, 25, 21, &"─".repeat(16), LINE);
            put(&mut cx, 42, 21, "elapsed 3.8s", DIM);
        }
        "statusbar" => {
            let source = [
                "use lunatui::prelude::*;",
                "",
                "fn main() -> Result<()> {",
                "    let mut term = Terminal::new()?;",
                "    term.draw(|f| ui(f))?;",
                "    loop {",
                "        if quit() { break; }",
                "    }",
                "    Ok(())",
                "}",
            ];
            for (index, text) in source.into_iter().enumerate() {
                let y = 2 + u16::try_from(index).unwrap_or_default();
                let current = index == 4;
                if current {
                    fill(&mut cx, Rect::new(1, y, 62, 1), PANEL);
                }
                put_on(
                    &mut cx,
                    2,
                    y,
                    &format!("{:>3}", index + 1),
                    if current { LAV } else { DIM },
                    if current { PANEL } else { BG },
                );
                put_on(
                    &mut cx,
                    7,
                    y,
                    text,
                    if current { INK } else { MUTED },
                    if current { PANEL } else { BG },
                );
                let (prefix, prefix_color) = match index {
                    0 => ("use", LAV),
                    2 => ("fn", LAV),
                    3 => ("    let", LAV),
                    5 => ("    loop", LAV),
                    6 => ("        if", LAV),
                    7 => ("    }", INK),
                    8 => ("    Ok", MOON),
                    9 => ("}", INK),
                    _ => ("", INK),
                };
                if !prefix.is_empty() {
                    let background = if current { PANEL } else { BG };
                    put_on(&mut cx, 7, y, prefix, prefix_color, background);
                    put_on(
                        &mut cx,
                        7 + u16::try_from(prefix.len()).unwrap_or_default(),
                        y,
                        &text[prefix.len()..],
                        if current { INK } else { MUTED },
                        background,
                    );
                }
            }
            let cursor_x =
                7 + u16::try_from("    term.draw(|f| ui(f))?;".len()).unwrap_or_default();
            fill(&mut cx, Rect::new(cursor_x, 6, 1, 1), LAV);
            put_on(&mut cx, cursor_x, 6, " ", BG, LAV);
            for y in 12..19 {
                put(&mut cx, 2, y, "  ~", LINE);
            }
            fill(&mut cx, Rect::new(1, 20, 62, 1), PANEL);
            StatusBar::new(" NORMAL ")
                .style(bold(BG).bg(LAV))
                .render(Rect::new(1, 20, 8, 1), &mut cx);
            cx.set_style(Rect::new(2, 20, 6, 1), bold(BG).bg(LAV));
            StatusBar::new(" ⎇ main ")
                .style(style(INK).bg(FOCUS_BG))
                .render(Rect::new(9, 20, 8, 1), &mut cx);
            StatusBar::new(" src/main.rs ")
                .style(style(INK).bg(PANEL))
                .render(Rect::new(17, 20, 13, 1), &mut cx);
            StatusBar::new(" ● ")
                .style(style(MOON).bg(PANEL))
                .render(Rect::new(30, 20, 3, 1), &mut cx);
            StatusBar::new(" rust ")
                .style(style(MUTED).bg(PANEL))
                .render(Rect::new(39, 20, 6, 1), &mut cx);
            StatusBar::new(" utf-8 ")
                .style(style(MUTED).bg(PANEL))
                .render(Rect::new(45, 20, 7, 1), &mut cx);
            StatusBar::new(" 5:23 ")
                .style(style(INK).bg(FOCUS_BG))
                .render(Rect::new(52, 20, 6, 1), &mut cx);
            StatusBar::new(" 50% ")
                .style(style(BG).bg(MOON))
                .render(Rect::new(58, 20, 5, 1), &mut cx);
            put_on(&mut cx, 2, 21, "✓", GREEN, BG);
            put(&mut cx, 4, 21, "cargo check: 0 errors, 0 warnings", MUTED);
            put(&mut cx, 52, 21, "0.4s ago", DIM);
        }
        "statusdot" => {
            put_bold(&mut cx, 7, 5, "Service health", INK);
            put(
                &mut cx,
                7,
                7,
                "Three regions are responding normally.",
                MUTED,
            );
            let regions = [
                ("us-west", StatusState::Ok, GREEN),
                ("eu-central", StatusState::Ok, GREEN),
                ("ap-south", StatusState::Idle, MOON),
            ];
            for (i, (name, state, color)) in regions.into_iter().enumerate() {
                let y = 11 + u16::try_from(i).unwrap_or_default() * 3;
                StatusDot::new(state).render(Rect::new(12, y, 1, 1), &mut cx);
                cx.set_style(Rect::new(12, y, 1, 1), style(color));
                put_bold(&mut cx, 16, y, name, color);
                put(
                    &mut cx,
                    38,
                    y,
                    if i == 2 { "standby" } else { "healthy" },
                    MUTED,
                );
            }
        }
        "table" => {
            let headers = [
                (3, "Benchmark"),
                (25, "Median ▾"),
                (35, "p99"),
                (44, "Δ"),
                (52, "Status"),
            ];
            for (x, title) in headers {
                put_bold(
                    &mut cx,
                    x,
                    2,
                    title,
                    if title == "Median ▾" { LAV } else { MUTED },
                );
            }
            put(&mut cx, 2, 3, &"─".repeat(60), LINE);
            let rows = [
                ["full redraw 120×40", "0.81 ms", "1.02", "-12%", "● pass"],
                ["diff render", "0.19 ms", "0.27", "-31%", "● pass"],
                ["scroll 10k lines", "1.44 ms", "1.90", "-4%", "● pass"],
                ["table 5k rows", "0.96 ms", "1.31", "+2%", "◐ watch"],
                ["unicode wide chars", "0.58 ms", "0.74", "-9%", "● pass"],
                ["sparkline 1k pts", "0.22 ms", "0.29", "-18%", "● pass"],
                ["layout solve", "0.07 ms", "0.11", "-6%", "● pass"],
                ["image fallback", "2.31 ms", "3.05", "+11%", "○ slow"],
                ["resize storm", "0.88 ms", "1.40", "-3%", "● pass"],
            ];
            let table_rows: Vec<Vec<String>> = rows
                .iter()
                .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
                .collect();
            fill(&mut cx, Rect::new(2, 5, 60, 1), FOCUS_BG);
            Table::new(
                VecSource::from_rows(table_rows),
                vec![
                    Constraint::Length(22),
                    Constraint::Length(10),
                    Constraint::Length(9),
                    Constraint::Length(8),
                    Constraint::Length(9),
                ],
            )
            .selected(Some(1))
            .style(style(MUTED))
            .highlight_style(Style::new().fg(INK).bg(FOCUS_BG))
            .render(Rect::new(3, 4, 58, 9), &mut cx);
            put_on(&mut cx, 2, 5, "▌", LAV, FOCUS_BG);
            for y in [7, 9, 11] {
                fill(&mut cx, Rect::new(2, y, 60, 1), PANEL);
            }
            for (index, row) in rows.iter().enumerate() {
                let y = 4 + u16::try_from(index).unwrap_or_default();
                let background = if index == 1 {
                    FOCUS_BG
                } else if [3, 5, 7].contains(&index) {
                    PANEL
                } else {
                    BG
                };
                if index == 1 {
                    put_on_bold(&mut cx, 3, y, row[0], INK, background);
                } else {
                    put_on(&mut cx, 3, y, row[0], INK, background);
                }
                put_on(
                    &mut cx,
                    25,
                    y,
                    row[1],
                    if index == 1 { LAV } else { MUTED },
                    background,
                );
                put_on(&mut cx, 35, y, row[2], MUTED, background);
                let delta_color = if row[3].starts_with('-') { GREEN } else { RED };
                put_on(&mut cx, 44, y, row[3], delta_color, background);
                let status_color = match row[4] {
                    "● pass" => GREEN,
                    "◐ watch" => MOON,
                    _ => RED,
                };
                put_on(&mut cx, 52, y, row[4], status_color, background);
            }
            put(&mut cx, 2, 14, &"─".repeat(60), LINE);
            put(&mut cx, 3, 15, "Selected", DIM);
            put_bold(&mut cx, 12, 15, "diff render", LAV);
            put(&mut cx, 3, 17, "Samples", MUTED);
            put(&mut cx, 15, 17, "10,000", INK);
            put(&mut cx, 3, 18, "Grid", MUTED);
            put(&mut cx, 15, 18, "120 × 40", INK);
            put(&mut cx, 3, 19, "Changed", MUTED);
            put(&mut cx, 15, 19, "~6% of cells", INK);
            put(&mut cx, 34, 16, "distribution", DIM);
            let distribution = [0.1_f32, 0.3, 0.7, 1.0, 0.8, 0.45, 0.25, 0.12, 0.06, 0.03];
            for (index, value) in distribution.into_iter().enumerate() {
                let height = value * 4.0 * 8.0;
                let x = 34 + u16::try_from(index).unwrap_or_default() * 2;
                for row in 0..4 {
                    let level = height - f32::from(3 - row) * 8.0;
                    let glyph = spark_glyph(level);
                    put(
                        &mut cx,
                        x,
                        17 + row,
                        &glyph.to_string().repeat(2),
                        if index == 3 { LAV } else { LAV2 },
                    );
                }
            }
            put(&mut cx, 3, 21, "9 rows", DIM);
            put(&mut cx, 12, 21, "· sorted by median ↓", DIM);
        }
        "tabs" => {
            put(&mut cx, 1, 3, &"─".repeat(62), LINE);
            put(&mut cx, 0, 3, "├", LINE);
            put(&mut cx, 63, 3, "┤", LINE);
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::new().bg(BG))
                .border_style(style(LINE_HI))
                .render(Rect::new(29, 1, 13, 3), &mut cx);
            Tabs::new([
                "  1 Overview ",
                "   2 Logs ",
                "   3 Metrics ",
                "   4 Settings ",
            ])
            .select(2)
            .style(style(MUTED))
            .highlight_style(bold(LAV))
            .render(Rect::new(3, 2, 58, 1), &mut cx);
            put(&mut cx, 29, 2, "│", LINE_HI);
            put(&mut cx, 41, 2, "│", LINE_HI);
            put(&mut cx, 29, 3, "╯", LINE_HI);
            put(&mut cx, 41, 3, "╰", LINE_HI);
            fill(&mut cx, Rect::new(30, 3, 11, 1), BG);
            put_bold(&mut cx, 31, 2, "3", LAV);
            put_bold(&mut cx, 33, 2, "Metrics", LAV);
            put_bold(&mut cx, 3, 5, "Metrics", MOON);
            put(&mut cx, 11, 5, "· lunatui-demo · uptime 3h 12m", DIM);
            let stats = [
                ("Frames", "1.25k/s", LAV),
                ("Draw", "0.81 ms", MOON),
                ("Cells Δ", "6.1%", GREEN),
                ("Alloc", "0 B/f", BLUE),
            ];
            for (index, (label, value, color)) in stats.into_iter().enumerate() {
                let x = 3 + u16::try_from(index).unwrap_or_default() * 15;
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(BG))
                    .border_style(style(LINE))
                    .render(Rect::new(x, 7, 14, 4), &mut cx);
                put(&mut cx, x + 2, 8, label, MUTED);
                put_bold(&mut cx, x + 2, 9, value, color);
            }
            put(&mut cx, 3, 12, "Frame time (ms)", MUTED);
            let data = [
                266_667, 291_265, 418_906, 389_714, 391_815, 426_476, 348_413, 394_706, 467_743,
                569_953, 448_720, 422_656, 342_337, 482_362, 540_538, 389_734, 537_149, 624_567,
                587_933, 541_524, 372_147, 206_981, 230_753, 114_379, 80_000, 80_000, 80_000,
                82_019, 145_488, 313_091, 350_820, 421_697, 441_458, 508_696, 504_059, 453_894,
                620_800, 734_929, 766_546, 744_574, 610_401, 482_324, 398_443, 267_613, 360_767,
                314_676, 400_357, 326_825, 433_282, 477_418, 274_521, 196_840, 346_514, 337_527,
                484_695, 435_423,
            ];
            for (row, value) in [(13, "2"), (15, "1"), (18, "0")] {
                put(&mut cx, 3, row, value, DIM);
            }
            let bars: Vec<_> = data
                .iter()
                .copied()
                .map(|value| {
                    SparklineBar::value(value).style(style(if value < 750_000 {
                        LAV
                    } else {
                        MOON
                    }))
                })
                .collect();
            Sparkline::new(&bars)
                .max(1_000_000)
                .render(Rect::new(5, 13, 56, 6), &mut cx);
            put(&mut cx, 5, 19, &"─".repeat(56), LINE);
            put(&mut cx, 5, 20, "-60s", DIM);
            put(&mut cx, 30, 20, "-30s", DIM);
            put(&mut cx, 57, 20, "now", DIM);
        }
        "textinput" => {
            put_bold(&mut cx, 3, 2, "Create project", MOON);
            put(&mut cx, 3, 3, "Scaffold a new Lunatui app.", MUTED);
            put_bold(&mut cx, 4, 5, "Name", LAV);
            put(&mut cx, 49, 5, "✓ available", GREEN);
            TextInput::new("moon-dash", 10)
                .block(panel("", LINE_HI).padding(Padding {
                    left: 2,
                    right: 2,
                    top: 0,
                    bottom: 0,
                }))
                .style(style(INK).bg(PANEL_RAISED))
                .cursor_style(Style::new().fg(BG).bg(LAV))
                .cursor_glyph(" ")
                .render(Rect::new(3, 6, 58, 3), &mut cx);
            fill(&mut cx, Rect::new(4, 7, 56, 1), PANEL_RAISED);
            put_on(&mut cx, 6, 7, "moon-dash", INK, PANEL_RAISED);
            put_on(&mut cx, 15, 7, " ", BG, LAV);
            put(&mut cx, 4, 9, "Template", MUTED);
            TextInput::new("dashboard  ▾", "dashboard  ▾".len())
                .block(
                    panel("", LINE)
                        .style(Style::new().bg(PANEL))
                        .padding(Padding {
                            left: 2,
                            right: 2,
                            top: 0,
                            bottom: 0,
                        }),
                )
                .style(style(MUTED).bg(PANEL))
                .cursor_style(Style::new().fg(PANEL).bg(PANEL))
                .render(Rect::new(3, 10, 58, 3), &mut cx);
            put_on(&mut cx, 18, 11, " ", PANEL, PANEL);
            put_on(&mut cx, 6, 11, "dashboard  ▾", INK, PANEL);
            put(&mut cx, 4, 13, "Description", MUTED);
            put(&mut cx, 52, 13, "optional", DIM);
            TextInput::new("A tiny system monitor…", "A tiny system monitor…".len())
                .block(
                    panel("", LINE)
                        .style(Style::new().bg(PANEL))
                        .padding(Padding {
                            left: 2,
                            right: 2,
                            top: 0,
                            bottom: 0,
                        }),
                )
                .style(style(DIM).bg(PANEL))
                .cursor_style(Style::new().fg(PANEL).bg(PANEL))
                .render(Rect::new(3, 14, 58, 3), &mut cx);
            put_on(&mut cx, 28, 15, " ", PANEL, PANEL);
            put(&mut cx, 4, 18, "[", DIM);
            put(&mut cx, 5, 18, "✓", LAV);
            put(&mut cx, 6, 18, "]", DIM);
            put(&mut cx, 8, 18, "Initialize git repository", MUTED);
            put(&mut cx, 4, 19, "[ ]", DIM);
            put(&mut cx, 8, 19, "Add example tests", MUTED);
            put(&mut cx, 4, 21, "name: moon-dash", DIM);
            fill(&mut cx, Rect::new(37, 21, 8, 1), PANEL);
            put_on(&mut cx, 37, 21, " Cancel ", MUTED, PANEL);
            fill(&mut cx, Rect::new(45, 21, 2, 1), BG);
            put_on_bold(&mut cx, 47, 21, "  Create ↵  ", BG, LAV);
            cx.set_style(Rect::new(49, 21, 8, 1), bold(BG).bg(LAV));
        }
        "borders" => {
            put(&mut cx, 6, 4, "Soft corners keep structure quiet.", MUTED);
            let examples = [
                ("Rounded", BorderType::Rounded, LAV),
                ("Plain", BorderType::Plain, LINE),
                ("Double", BorderType::Double, MOON),
            ];
            for (i, (label, kind, color)) in examples.into_iter().enumerate() {
                let x = 5 + u16::try_from(i).unwrap_or_default() * 19;
                let block = Block::bordered()
                    .border_type(kind)
                    .border_style(style(color))
                    .title(label)
                    .title_style(bold(color));
                block.render(Rect::new(x, 9, 17, 8), &mut cx);
                put_on(&mut cx, x + 3, 13, "Content", INK, PANEL_RAISED);
            }
        }
        _ => return false,
    }
    if let Some(hint) = match name {
        "bigtext" => Some("24h · local time"),
        "calendar" => Some("←→ month · ↵ select"),
        "gauge" => Some("tab focus · space pause"),
        "list" => Some("↑↓ move · ↵ open · x toggle"),
        "sparkline" => Some("1s refresh · q quit"),
        "spinner" => Some("12 styles · 80ms frame"),
        "table" => Some("↑↓ select · s sort · / filter"),
        "tabs" => Some("←→ / 1-4 switch"),
        "textinput" => Some("tab next · ↵ submit · esc cancel"),
        _ => None,
    } {
        let x = WIDTH
            .saturating_sub(u16::try_from(hint.chars().count()).unwrap_or_default())
            .saturating_sub(3);
        put(&mut cx, x, HEIGHT - 1, &format!(" {hint} "), DIM);
    }
    true
}

pub fn run() -> io::Result<()> {
    let Some(name) = env::args()
        .skip(1)
        .find_map(|arg| arg.strip_prefix("--site-widget=").map(str::to_owned))
    else {
        eprintln!("usage: gallery --site-widget=<widget>");
        std::process::exit(2);
    };
    let theme = theme();
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    if !render_case(&name, &mut grid, &mut clusters, &theme) {
        eprintln!("unknown widget: {name}");
        std::process::exit(2);
    }
    let mut encoder = Encoder::new();
    let previous = Grid::new(WIDTH, HEIGHT);
    let output = encoder.encode_diff(&previous, &grid, &clusters, ColorMode::TrueColor);
    io::stdout().write_all(output.as_ref())
}
