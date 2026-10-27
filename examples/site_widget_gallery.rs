// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::env;
use std::io::{self, Write};
use std::time::Duration;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style, text_display_width};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Alignment, BarChart, BigText, BigTextFont, Block, BorderType, Borders, Constraint, Date, Gauge,
    Lightcycle, LineGauge, List, Monthly, Overlay, Padding, Paragraph, RenderCx, Scrollbar,
    Sparkline, SparklineBar, Spinner, SpinnerPattern, StatusBar, StatusDot, StatusState, Table,
    Tabs, TextInput, TextWrap, TitlePosition, VecSource, WeekStart, Widget,
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

fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

fn bold(color: Color) -> Style {
    style(color).with_attrs(Attrs::new().set_bold(true))
}

fn frame(title: &str, hint: Option<&str>) -> Block {
    let frame = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(Style::new().fg(INK).bg(BG))
        .border_style(style(LINE))
        .title(format!(" ☾ {title} "))
        .title_style(bold(LAV))
        .title_inset(1);
    match hint {
        Some(hint) => frame.title_at(
            format!(" {hint} "),
            TitlePosition::Bottom,
            Alignment::Right,
            style(DIM),
        ),
        None => frame,
    }
}

fn label(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    let width = u16::try_from(text_display_width(text)).unwrap_or_default();
    Paragraph::new(text)
        .style(style)
        .render(Rect::new(x, y, width, 1), cx);
}

fn band(cx: &mut RenderCx<'_>, area: Rect, color: Color) {
    Block::new().style(Style::new().bg(color)).render(area, cx);
}

fn rule(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, color: Color) {
    Block::new()
        .borders(Borders::TOP)
        .border_style(style(color))
        .render(Rect::new(x, y, width, 1), cx);
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
    let mut cx = RenderCx::new(grid, clusters, theme);
    let hint = match name {
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
    };
    frame(
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
        hint,
    )
    .render(Rect::new(0, 0, WIDTH, HEIGHT), &mut cx);
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
                .render(Rect::new(5, 2, 50, 7), &mut cx);
            label(&mut cx, 56, 8, ":12", bold(MOON));
            label(&mut cx, 5, 11, "Tuesday", bold(INK));
            label(&mut cx, 13, 11, "· 27 October 2026", style(MUTED));
            label(
                &mut cx,
                5,
                12,
                "Waning gibbous · 94% illuminated",
                style(DIM),
            );
            rule(&mut cx, 2, 15, 60, LINE);
            let cities = [
                ("UTC", "05:47", "+6h"),
                ("Tokyo", "14:47", "+15h"),
                ("Berlin", "06:47", "+7h"),
                ("San Francisco", "22:47", "-1h"),
            ];
            for (i, (city, time, offset)) in cities.into_iter().enumerate() {
                let x = 4 + u16::try_from(i % 2).unwrap_or_default() * 30;
                let y = 17 + u16::try_from(i / 2).unwrap_or_default() * 2;
                label(&mut cx, x, y, city, style(MUTED));
                label(
                    &mut cx,
                    x + 15,
                    y,
                    time,
                    bold(if i == 0 { LAV } else { INK }),
                );
                let offset_width = u16::try_from(text_display_width(offset)).unwrap_or_default();
                label(&mut cx, x + 25 - offset_width, y, offset, style(DIM));
            }
            label(&mut cx, 4, 21, "▕", style(LINE));
            LineGauge::new(0.74)
                .filled_style(style(LAV2))
                .unfilled_style(style(LINE))
                .render(Rect::new(5, 21, 48, 1), &mut cx);
            label(&mut cx, 54, 21, "74%", style(DIM));
        }
        "block" => {
            label(
                &mut cx,
                5,
                3,
                "A calm frame for terminal content",
                style(MUTED),
            );
            let block = panel("Deployment", LAV).padding(Padding {
                left: 2,
                right: 2,
                top: 1,
                bottom: 1,
            });
            block.render(Rect::new(5, 6, 54, 12), &mut cx);
            label(
                &mut cx,
                9,
                9,
                "lunatui-web  ·  production",
                fg(INK).bg(PANEL_RAISED),
            );
            label(
                &mut cx,
                9,
                11,
                "Build finished in 2.4 seconds",
                fg(MUTED).bg(PANEL_RAISED),
            );
            label(
                &mut cx,
                9,
                13,
                "3 regions healthy",
                fg(GREEN).bg(PANEL_RAISED),
            );
        }
        "calendar" => {
            let months = [
                (
                    date(2_026, 10, 27),
                    "October 2026",
                    bold(MOON),
                    5,
                    [3, 14, 31],
                    true,
                ),
                (
                    date(2_026, 11, 1),
                    "November 2026",
                    bold(MUTED),
                    36,
                    [5, 18, 24],
                    false,
                ),
            ];
            for (day, title, title_style, x, event_days, today) in months {
                let title_width = u16::try_from(text_display_width(title)).unwrap_or_default();
                label(
                    &mut cx,
                    x + 1 + (20 - title_width) / 2,
                    2,
                    title,
                    title_style,
                );
                let events: Vec<_> = event_days
                    .iter()
                    .map(|&event_day| (date(2_026, day.month(), event_day), bold(LAV)))
                    .collect();
                let monthly = Monthly::new(day, events.as_slice())
                    .week_start(WeekStart::Monday)
                    .event_markers(false)
                    .show_weekdays_header(style(DIM))
                    .default_style(style(INK))
                    .weekend_style(style(LAV2));
                let monthly = if today {
                    monthly.selected_style(bold(BG).bg(LAV))
                } else {
                    monthly
                };
                monthly.render(Rect::new(x, 3, 21, 7), &mut cx);
            }
            Block::new()
                .borders(Borders::LEFT)
                .border_style(style(LINE))
                .render(Rect::new(31, 2, 1, 7), &mut cx);
            rule(&mut cx, 2, 10, 60, LINE);
            label(&mut cx, 3, 11, "Upcoming", bold(LAV));
            let events = [
                ("Sat 31 Oct", "●", LAV, "Release v0.9 tagged", "20:00"),
                ("Thu 05 Nov", "●", MOON, "Docs freeze", "09:30"),
                ("Wed 18 Nov", "●", GREEN, "Community call", "17:00"),
                ("Tue 24 Nov", "○", DIM, "Benchmark refresh", "—"),
            ];
            let mut rows = VecSource::new();
            let mut cells = Vec::new();
            for (index, (day, dot, color, title, time)) in events.into_iter().enumerate() {
                rows.push_row(vec![
                    day.to_owned(),
                    dot.to_owned(),
                    title.to_owned(),
                    time.to_owned(),
                ]);
                let selected = index == 0;
                cells.push(vec![
                    fg(MUTED),
                    fg(color),
                    if selected {
                        fg(INK).with_attrs(Attrs::new().set_bold(true))
                    } else {
                        fg(INK)
                    },
                    fg(if selected { LAV } else { DIM }),
                ]);
            }
            Table::new(
                rows,
                vec![
                    Constraint::Length(12),
                    Constraint::Length(2),
                    Constraint::Fill(1),
                    Constraint::Length(5),
                    Constraint::Length(2),
                ],
            )
            .style(style(INK))
            .selected(Some(0))
            .highlight_style(style(INK).bg(FOCUS_BG))
            .highlight_symbol("▌ ", style(LAV).bg(FOCUS_BG))
            .alignments(vec![
                Alignment::Left,
                Alignment::Left,
                Alignment::Left,
                Alignment::Right,
            ])
            .cell_styles(cells)
            .row_gap(1)
            .render(Rect::new(2, 13, 60, 7), &mut cx);
            label(&mut cx, 3, 21, "Today", style(DIM));
            label(&mut cx, 9, 21, "Tue 27 Oct", bold(MOON));
            label(&mut cx, 21, 21, "· 4 upcoming events", style(DIM));
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
            for (index, (name, progress, color, detail)) in stages.into_iter().enumerate() {
                let y = 2 + u16::try_from(index).unwrap_or_default() * 4;
                let focused = index == 0;
                if focused {
                    label(&mut cx, 2, y, "▌", style(LAV));
                }
                label(
                    &mut cx,
                    4,
                    y,
                    name,
                    if focused { bold(INK) } else { style(MUTED) },
                );
                let percent = format!("{:.0}%", progress * 100.0);
                let width = u16::try_from(text_display_width(&percent)).unwrap_or_default();
                label(&mut cx, WIDTH - 4 - width, y, &percent, bold(color));
                Gauge::new(progress)
                    .sub_cell(true)
                    .gauge_style(fg(color).bg(PANEL_RAISED))
                    .track_style(Style::new().bg(PANEL_RAISED))
                    .render(Rect::new(4, y + 1, 56, 1), &mut cx);
                label(&mut cx, 4, y + 2, detail, style(DIM));
            }
            rule(&mut cx, 2, 14, 60, LINE);
            let resources = [
                ("CPU", LAV, 34_u16),
                ("GPU", MOON, 18_u16),
                ("Disk", GREEN, 61_u16),
                ("Net", BLUE, 27_u16),
            ];
            for (index, (name, color, percent)) in resources.into_iter().enumerate() {
                let y = 16 + u16::try_from(index).unwrap_or_default() * 2;
                label(&mut cx, 4, y, name, style(MUTED));
                LineGauge::new(f32::from(percent) / 100.0)
                    .filled_style(style(color))
                    .unfilled_style(style(LINE))
                    .render(Rect::new(10, y, 22, 1), &mut cx);
                label(&mut cx, 33, y, &format!("{percent:>3}%"), style(DIM));
            }
            let week = [
                400_000, 700_000, 550_000, 900_000, 650_000, 300_000, 800_000,
            ];
            let mut day_styles = [fg(LAV2); 7];
            day_styles[3] = fg(LAV);
            BarChart::new(&week)
                .max(1_000_000)
                .bar_width(2)
                .bar_gap(1)
                .style(fg(LAV2))
                .bar_styles(&day_styles)
                .track_style(Style::new().bg(PANEL_RAISED))
                .labels(&["M", "T", "W", "T", "F", "S", "S"])
                .label_style(style(DIM))
                .render(Rect::new(42, 16, 20, 7), &mut cx);
        }
        "lightcycle" => {
            label(&mut cx, 5, 3, "Signal trail", bold(INK));
            label(
                &mut cx,
                5,
                5,
                "Braille detail · 60 frames per second",
                style(MUTED),
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
                "Block",
                "Paragraph",
                "StatusDot",
                "Lightcycle",
                "Borders",
            ];
            let icon_colors = [LAV, LAV, MOON, GREEN, BLUE, LAV, MOON, GREEN, BLUE];
            let mut items = VecSource::new();
            let mut icons = Vec::new();
            let mut marks = Vec::new();
            for (index, name) in names.into_iter().enumerate() {
                items.push_row(vec![name.to_owned()]);
                let color = if index < 13 {
                    [
                        LAV, LAV, MOON, GREEN, BLUE, LAV, MOON, GREEN, BLUE, LAV, MOON, GREEN, BLUE,
                    ][index]
                } else {
                    icon_colors[index % icon_colors.len()]
                };
                icons.push(("◆".to_owned(), fg(color)));
                marks.push(matches!(index, 0 | 3 | 5).then(|| ("✓".to_owned(), fg(GREEN))));
            }
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::new().bg(BG))
                .border_style(style(LINE_HI))
                .title(" Widgets ")
                .title_style(bold(LAV))
                .title_inset(1)
                .render(Rect::new(2, 1, 28, 22), &mut cx);
            List::new(items)
                .style(style(MUTED))
                .selected(Some(3))
                .highlight_style(bold(INK).bg(FOCUS_BG))
                .highlight_symbol("▌ ", fg(LAV))
                .icons(icons)
                .marks(marks, 2)
                .render(Rect::new(3, 3, 25, 13), &mut cx);
            Scrollbar::vertical(names.len(), 13, 0)
                .track_style(style(LINE))
                .thumb_style(style(LAV2))
                .render(Rect::new(28, 3, 1, 13), &mut cx);
            label(&mut cx, 4, 20, "18 items · 3 selected", style(DIM));
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::new().bg(BG))
                .border_style(style(LINE))
                .title(" Preview ")
                .title_style(bold(MUTED))
                .title_inset(1)
                .render(Rect::new(31, 1, 31, 22), &mut cx);
            label(&mut cx, 33, 3, "Gauge", bold(MOON));
            label(&mut cx, 33, 4, "Progress with sub-cell", style(MUTED));
            label(&mut cx, 33, 5, "precision (1/8 cell).", style(MUTED));
            Gauge::new(0.66)
                .sub_cell(true)
                .gauge_style(fg(LAV).bg(PANEL_RAISED))
                .track_style(Style::new().bg(PANEL_RAISED))
                .render(Rect::new(33, 7, 26, 1), &mut cx);
            label(&mut cx, 33, 8, "66%  ·  eased 250ms", style(DIM));
            rule(&mut cx, 32, 10, 29, LINE);
            label(&mut cx, 33, 11, "Options", bold(LAV));
            for (y, key, value) in [
                (13, "label", "\"Compiling\""),
                (14, "ratio", "0.66"),
                (15, "style", "lav · bold"),
                (16, "use_unicode", "true"),
                (18, "Since", "v0.3"),
            ] {
                label(&mut cx, 33, y, key, style(MUTED));
                label(&mut cx, 46, y, value, style(INK));
            }
            label(&mut cx, 33, 20, "↵ open docs", style(DIM));
        }
        "overlay" => {
            let heading = Color::Rgb(74, 79, 128);
            let quiet = Color::Rgb(47, 54, 92);
            label(&mut cx, 3, 2, "Sessions", bold(heading));
            label(&mut cx, 33, 2, "Activity", bold(heading));
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
                label(&mut cx, 3, y, name, style(quiet));
                label(&mut cx, 20, y, time, style(quiet));
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
            let dialog = Rect::new(11, 6, 40, 12);
            Overlay::new(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(PANEL_RAISED))
                    .border_style(fg(LINE_HI).bg(PANEL_RAISED))
                    .title(" Quit Lunatui? ")
                    .title_style(bold(MOON).bg(PANEL_RAISED))
                    .title_inset(1),
                dialog,
            )
            .backdrop(0.0, BG)
            .shadow(Style::new().bg(Color::Rgb(4, 5, 11)))
            .render(Rect::new(1, 1, WIDTH - 2, HEIGHT - 2), &mut cx);
            let raised = |color| style(color).bg(PANEL_RAISED);
            label(&mut cx, 14, 8, "⚠", raised(MOON));
            label(&mut cx, 16, 8, "You have 2 unsaved layouts.", raised(INK));
            label(
                &mut cx,
                16,
                10,
                "Changes since 05:31 will be lost",
                raised(MUTED),
            );
            label(&mut cx, 16, 11, "if you quit now.", raised(MUTED));
            label(&mut cx, 14, 14, "  Save & quit  ", bold(BG).bg(LAV));
            label(&mut cx, 31, 14, "  Quit  ", style(INK).bg(FOCUS_BG));
            label(&mut cx, 41, 14, " Cancel ", style(MUTED).bg(PANEL));
            label(
                &mut cx,
                14,
                16,
                "tab cycle · ↵ confirm · esc close",
                raised(DIM),
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
            label(
                &mut cx,
                9,
                6,
                "A clear view of the work",
                fg(MOON).bg(PANEL_RAISED),
            );
            Paragraph::new("Lunatui keeps terminal content readable as it changes. Wrap long notes, align the details, and leave enough room for the next useful thing. Each line keeps its place as the window grows, so long release notes remain easy to scan.")
                .wrap(TextWrap { trim: true })
                .style(style(INK).bg(PANEL_RAISED))
                .render(Rect::new(9, 8, 46, 8), &mut cx);
            label(&mut cx, 9, 17, "NOTES  /  v0.9.2", fg(DIM).bg(PANEL_RAISED));
            label(
                &mut cx,
                39,
                17,
                "4 lines · 312 chars",
                fg(DIM).bg(PANEL_RAISED),
            );
        }
        "padding" => {
            label(
                &mut cx,
                5,
                3,
                "Comfortable spacing, without wasted space",
                bold(LAV),
            );
            let compact = panel("Compact · 1 cell", LAV).padding(Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            });
            compact.render(Rect::new(5, 7, 26, 7), &mut cx);
            label(&mut cx, 8, 10, "moon-dash", fg(INK).bg(PANEL_RAISED));
            label(
                &mut cx,
                8,
                12,
                "12 dependencies",
                fg(MUTED).bg(PANEL_RAISED),
            );
            let roomy = panel("Roomy · 3 cells", LINE_HI).padding(Padding {
                left: 3,
                right: 3,
                top: 1,
                bottom: 1,
            });
            roomy.render(Rect::new(33, 7, 26, 10), &mut cx);
            label(&mut cx, 38, 11, "moon-dash", fg(INK).bg(PANEL_RAISED));
            label(
                &mut cx,
                38,
                13,
                "Terminal dashboard",
                fg(MUTED).bg(PANEL_RAISED),
            );
            label(&mut cx, 38, 15, "12 dependencies", fg(DIM).bg(PANEL_RAISED));
            label(
                &mut cx,
                5,
                19,
                "Same content, two useful rhythms.",
                style(DIM),
            );
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
            label(&mut cx, 40, 8, "23:47", fg(LAV).bg(PANEL_RAISED));
            label(
                &mut cx,
                40,
                10,
                "Time, at a glance.",
                fg(MUTED).bg(PANEL_RAISED),
            );
            label(&mut cx, 40, 13, "Render size", fg(DIM).bg(PANEL_RAISED));
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
                let focused = title == "Frames / s";
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(BG))
                    .border_style(style(if focused { LINE_HI } else { LINE }))
                    .title(format!(" {title} "))
                    .title_style(bold(if focused { LAV } else { MUTED }))
                    .title_inset(1)
                    .render(Rect::new(x, y, 30, 6), &mut cx);
                label(&mut cx, x + 2, y + 1, value, bold(color));
                let value_width = u16::try_from(text_display_width(value)).unwrap_or_default();
                label(&mut cx, x + 3 + value_width, y + 1, unit, style(DIM));
                Sparkline::new(data)
                    .max(1_000_000)
                    .style(style(color))
                    .render(Rect::new(x + 2, y + 2, 26, 3), &mut cx);
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
                .title(" Throughput · last 60s ")
                .title_style(bold(MUTED))
                .title_inset(1)
                .render(Rect::new(2, 13, 60, 10), &mut cx);
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
            label(&mut cx, 4, 21, "min 0.62k", style(DIM));
            label(&mut cx, 26, 21, "avg 1.18k", style(DIM));
            label(&mut cx, 49, 21, "max 1.31k", style(MOON));
        }
        "spinner" => {
            label(&mut cx, 3, 2, "Styles", bold(LAV));
            let cards = [
                ("Braille dots", SpinnerPattern::BrailleDots, LAV, 1),
                ("Half moon", SpinnerPattern::HalfMoon, MOON, 1),
                ("Quadrant", SpinnerPattern::Quadrant, GREEN, 1),
                ("Braille ring", SpinnerPattern::BrailleRing, BLUE, 1),
                ("Arc", SpinnerPattern::Arc, LAV, 1),
                ("Bars", SpinnerPattern::Bars, MOON, 3),
            ];
            for (index, (name, pattern, color, glyph_width)) in cards.into_iter().enumerate() {
                let x = 3 + u16::try_from(index % 3).unwrap_or_default() * 20;
                let y = 4 + u16::try_from(index / 3).unwrap_or_default() * 3;
                let focused = index == 1;
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(BG))
                    .border_style(style(if focused { LINE_HI } else { LINE }))
                    .render(Rect::new(x, y, 18, 3), &mut cx);
                Spinner::new(0)
                    .pattern(pattern)
                    .style(bold(color))
                    .render(Rect::new(x + 2, y + 1, glyph_width, 1), &mut cx);
                label(
                    &mut cx,
                    x + 3 + glyph_width,
                    y + 1,
                    name,
                    style(if focused { INK } else { MUTED }),
                );
            }
            rule(&mut cx, 2, 11, 60, LINE);
            label(&mut cx, 3, 12, "Tasks", bold(LAV));
            let tasks = [
                ("✓", "Resolve dependencies", GREEN, "0.4s"),
                ("✓", "Compile lunatui-core", GREEN, "2.1s"),
                (
                    Spinner::new(1).pattern(SpinnerPattern::BrailleRing).glyph(),
                    "Compile lunatui-widgets",
                    LAV,
                    "1.3s",
                ),
                ("◐", "Generate docs", MOON, "…"),
                ("·", "Run examples", DIM, "queued"),
                ("·", "Package release", DIM, "queued"),
            ];
            let mut rows = VecSource::new();
            let mut cells = Vec::new();
            for (index, (icon, task, color, time)) in tasks.into_iter().enumerate() {
                rows.push_row(vec![icon.to_owned(), task.to_owned(), time.to_owned()]);
                let selected = index == 2;
                let bold_fg = |color| fg(color).with_attrs(Attrs::new().set_bold(true));
                cells.push(vec![
                    bold_fg(color),
                    if selected {
                        bold_fg(INK)
                    } else {
                        fg(if index < 4 { INK } else { DIM })
                    },
                    fg(if selected { LAV } else { DIM }),
                ]);
            }
            Table::new(
                rows,
                vec![
                    Constraint::Length(2),
                    Constraint::Fill(1),
                    Constraint::Length(6),
                    Constraint::Length(2),
                ],
            )
            .style(style(INK))
            .selected(Some(2))
            .highlight_style(style(INK).bg(FOCUS_BG))
            .highlight_symbol("▌ ", style(LAV).bg(FOCUS_BG))
            .alignments(vec![Alignment::Left, Alignment::Left, Alignment::Right])
            .cell_styles(cells)
            .render(Rect::new(2, 14, 60, 6), &mut cx);
            label(&mut cx, 4, 21, "2 of 6 done", style(MUTED));
            LineGauge::new(2.0 / 6.0)
                .filled_style(style(LAV))
                .unfilled_style(style(LINE))
                .render(Rect::new(17, 21, 24, 1), &mut cx);
            label(&mut cx, 42, 21, "elapsed 3.8s", style(DIM));
        }
        "statusbar" => {
            let source = [
                ("", "use lunatui::prelude::*;", LAV),
                ("", "", INK),
                ("", "fn main() -> Result<()> {", LAV),
                ("    ", "let mut term = Terminal::new()?;", LAV),
                ("    ", "term.draw(|f| ui(f))?;", INK),
                ("    ", "loop {", LAV),
                ("        ", "if quit() { break; }", LAV),
                ("    ", "}", INK),
                ("    ", "Ok(())", MOON),
                ("", "}", INK),
            ];
            band(&mut cx, Rect::new(1, 6, 62, 1), PANEL);
            for (index, (indent, text, keyword_color)) in source.into_iter().enumerate() {
                let y = 2 + u16::try_from(index).unwrap_or_default();
                let current = index == 4;
                let background = if current { PANEL } else { BG };
                label(
                    &mut cx,
                    2,
                    y,
                    &format!("{:>3}", index + 1),
                    style(if current { LAV } else { DIM }).bg(background),
                );
                let rest_color = if current { INK } else { MUTED };
                let keyword_width = text.find([' ', '.', '(']).unwrap_or(text.len());
                let (keyword, rest) = text.split_at(keyword_width);
                let x = 7 + u16::try_from(indent.len()).unwrap_or_default();
                if !keyword.is_empty() {
                    let color = if current { INK } else { keyword_color };
                    label(&mut cx, x, y, keyword, style(color).bg(background));
                }
                if !rest.is_empty() {
                    let keyword_cells =
                        u16::try_from(text_display_width(keyword)).unwrap_or_default();
                    label(
                        &mut cx,
                        x + keyword_cells,
                        y,
                        rest,
                        style(rest_color).bg(background),
                    );
                }
            }
            label(&mut cx, 33, 6, " ", Style::new().fg(BG).bg(LAV));
            for y in 12..19 {
                label(&mut cx, 2, y, "  ~", style(LINE));
            }
            StatusBar::new("")
                .style(Style::new().bg(PANEL))
                .left([
                    (" NORMAL ", bold(BG).bg(LAV)),
                    (" ⎇ main ", fg(INK).bg(FOCUS_BG)),
                    (" src/main.rs ", fg(INK).bg(PANEL)),
                    (" ● ", fg(MOON).bg(PANEL)),
                ])
                .right([
                    (" rust ", fg(MUTED).bg(PANEL)),
                    (" utf-8 ", fg(MUTED).bg(PANEL)),
                    (" 5:23 ", fg(INK).bg(FOCUS_BG)),
                    (" 50% ", fg(BG).bg(MOON)),
                ])
                .render(Rect::new(1, 20, 62, 1), &mut cx);
            label(&mut cx, 2, 21, "✓", style(GREEN));
            label(
                &mut cx,
                4,
                21,
                "cargo check: 0 errors, 0 warnings",
                style(MUTED),
            );
            label(&mut cx, 52, 21, "0.4s ago", style(DIM));
        }
        "statusdot" => {
            label(&mut cx, 7, 5, "Service health", bold(INK));
            label(
                &mut cx,
                7,
                7,
                "Three regions are responding normally.",
                style(MUTED),
            );
            let regions = [
                ("us-west", StatusState::Ok, GREEN),
                ("eu-central", StatusState::Ok, GREEN),
                ("ap-south", StatusState::Idle, INK),
            ];
            for (i, (name, state, color)) in regions.into_iter().enumerate() {
                let y = 11 + u16::try_from(i).unwrap_or_default() * 3;
                StatusDot::new(state).render(Rect::new(12, y, 1, 1), &mut cx);
                label(&mut cx, 16, y, name, bold(color));
                label(
                    &mut cx,
                    38,
                    y,
                    if i == 2 { "standby" } else { "healthy" },
                    style(MUTED),
                );
            }
        }
        "table" => {
            let columns = vec![
                Constraint::Length(22),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Length(8),
                Constraint::Length(9),
            ];
            let marker = ("▌", fg(LAV).bg(FOCUS_BG));
            Table::new(VecSource::new(), columns.clone())
                .highlight_symbol(marker.0, marker.1)
                .header([
                    ("Benchmark", bold(MUTED)),
                    ("Median ▾", bold(LAV)),
                    ("p99", bold(MUTED)),
                    ("Δ", bold(MUTED)),
                    ("Status", bold(MUTED)),
                ])
                .render(Rect::new(2, 2, 60, 1), &mut cx);
            rule(&mut cx, 2, 3, 60, LINE);
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
            let mut source = VecSource::new();
            let mut cells = Vec::new();
            for (index, row) in rows.iter().enumerate() {
                source.push_row(row.iter().map(|cell| (*cell).to_owned()).collect());
                let selected = index == 1;
                let delta = if row[3].starts_with('-') { GREEN } else { RED };
                let status = match row[4] {
                    "● pass" => GREEN,
                    "◐ watch" => MOON,
                    _ => RED,
                };
                cells.push(vec![
                    if selected {
                        fg(INK).with_attrs(Attrs::new().set_bold(true))
                    } else {
                        fg(INK)
                    },
                    fg(if selected { LAV } else { MUTED }),
                    fg(MUTED),
                    fg(delta),
                    fg(status),
                ]);
            }
            Table::new(source, columns)
                .style(style(INK))
                .selected(Some(1))
                .highlight_style(style(INK).bg(FOCUS_BG))
                .highlight_symbol(marker.0, marker.1)
                .odd_row_style(Style::new().bg(PANEL))
                .cell_styles(cells)
                .render(Rect::new(2, 4, 60, 9), &mut cx);
            rule(&mut cx, 2, 14, 60, LINE);
            label(&mut cx, 3, 15, "Selected", style(DIM));
            label(&mut cx, 12, 15, "diff render", bold(LAV));
            for (y, key, value) in [
                (17, "Samples", "10,000"),
                (18, "Grid", "120 × 40"),
                (19, "Changed", "~6% of cells"),
            ] {
                label(&mut cx, 3, y, key, style(MUTED));
                label(&mut cx, 15, y, value, style(INK));
            }
            label(&mut cx, 34, 16, "distribution", style(DIM));
            let distribution = [
                100_000, 300_000, 700_000, 1_000_000, 800_000, 450_000, 250_000, 120_000, 60_000,
                30_000,
            ];
            let mut bar_styles = [fg(LAV2); 10];
            bar_styles[3] = fg(LAV);
            BarChart::new(&distribution)
                .max(1_000_000)
                .bar_width(2)
                .bar_gap(0)
                .style(fg(LAV2))
                .bar_styles(&bar_styles)
                .render(Rect::new(34, 17, 20, 4), &mut cx);
            label(&mut cx, 3, 21, "9 rows", style(DIM));
            label(&mut cx, 12, 21, "· sorted by median ↓", style(DIM));
        }
        "tabs" => {
            Tabs::new(["1 Overview", "2 Logs", "3 Metrics", "4 Settings"])
                .select(2)
                .style(style(MUTED))
                .highlight_style(bold(LAV))
                .boxed(style(LINE_HI), style(LINE))
                .join_parent_border(true)
                .indent(2)
                .render(Rect::new(0, 1, WIDTH, 3), &mut cx);
            label(&mut cx, 3, 5, "Metrics", bold(MOON));
            label(&mut cx, 11, 5, "· lunatui-demo · uptime 3h 12m", style(DIM));
            let stats = [
                ("Frames", "1.25k/s", LAV),
                ("Draw", "0.81 ms", MOON),
                ("Cells Δ", "6.1%", GREEN),
                ("Alloc", "0 B/f", BLUE),
            ];
            for (index, (name, value, color)) in stats.into_iter().enumerate() {
                let x = 3 + u16::try_from(index).unwrap_or_default() * 15;
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .style(Style::new().bg(BG))
                    .border_style(style(LINE))
                    .render(Rect::new(x, 7, 14, 4), &mut cx);
                label(&mut cx, x + 2, 8, name, style(MUTED));
                label(&mut cx, x + 2, 9, value, bold(color));
            }
            label(&mut cx, 3, 12, "Frame time (ms)", style(MUTED));
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
                label(&mut cx, 3, row, value, style(DIM));
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
            rule(&mut cx, 5, 19, 56, LINE);
            label(&mut cx, 5, 20, "-60s", style(DIM));
            label(&mut cx, 30, 20, "-30s", style(DIM));
            label(&mut cx, 57, 20, "now", style(DIM));
        }
        "textinput" => {
            label(&mut cx, 3, 2, "Create project", bold(MOON));
            label(&mut cx, 3, 3, "Scaffold a new Lunatui app.", style(MUTED));
            let fields = [
                ("Name", "moon-dash", Some(("✓ available", GREEN)), true),
                ("Template", "dashboard  ▾", None, false),
                (
                    "Description",
                    "A tiny system monitor…",
                    Some(("optional", DIM)),
                    false,
                ),
            ];
            for (index, (name, value, note, focused)) in fields.into_iter().enumerate() {
                let y = 5 + u16::try_from(index).unwrap_or_default() * 4;
                label(
                    &mut cx,
                    4,
                    y,
                    name,
                    if focused { bold(LAV) } else { style(MUTED) },
                );
                if let Some((text, color)) = note {
                    let width = u16::try_from(text_display_width(text)).unwrap_or_default();
                    label(&mut cx, WIDTH - 4 - width, y, text, style(color));
                }
                let field_bg = if focused { PANEL_RAISED } else { PANEL };
                let value_color = if index == 2 { DIM } else { INK };
                TextInput::new(value, value.len())
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .style(Style::new().bg(BG))
                            .border_style(style(if focused { LINE_HI } else { LINE })),
                    )
                    .field_style(Style::new().bg(field_bg))
                    .padding(2)
                    .style(style(value_color).bg(field_bg))
                    .cursor_glyph(" ")
                    .cursor_style(Style::new().fg(BG).bg(LAV))
                    .show_cursor(focused)
                    .render(Rect::new(3, y + 1, 58, 3), &mut cx);
            }
            label(&mut cx, 4, 18, "[", style(DIM));
            label(&mut cx, 5, 18, "✓", style(LAV));
            label(&mut cx, 6, 18, "]", style(DIM));
            label(&mut cx, 8, 18, "Initialize git repository", style(MUTED));
            label(&mut cx, 4, 19, "[ ]", style(DIM));
            label(&mut cx, 8, 19, "Add example tests", style(MUTED));
            label(&mut cx, 4, 21, "name: moon-dash", style(DIM));
            label(&mut cx, 37, 21, " Cancel ", style(MUTED).bg(PANEL));
            label(&mut cx, 47, 21, "  Create ↵  ", bold(BG).bg(LAV));
        }
        "borders" => {
            label(
                &mut cx,
                6,
                4,
                "Soft corners keep structure quiet.",
                style(MUTED),
            );
            let examples = [
                ("Rounded", BorderType::Rounded, LAV),
                ("Plain", BorderType::Plain, LINE),
                ("Double", BorderType::Double, MOON),
            ];
            for (i, (name, kind, color)) in examples.into_iter().enumerate() {
                let x = 5 + u16::try_from(i).unwrap_or_default() * 19;
                let block = Block::bordered()
                    .border_type(kind)
                    .border_style(style(color))
                    .title(name)
                    .title_style(bold(color));
                block.render(Rect::new(x, 9, 17, 8), &mut cx);
                label(&mut cx, x + 3, 13, "Content", fg(INK).bg(PANEL_RAISED));
            }
        }
        _ => return false,
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
