// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! A release console built from Lunatui widgets.
//!
//! Run `cargo run --example release_console_app` for the live screen. The site draws its widget
//! scenes from this program: `--snapshot` prints one plain-text frame, `--snapshot-styled` prints
//! the same frame as runs of text with a class per color, and `--scene <name>` picks one widget
//! scene (`form`, `progress`, `logview`, `calendar`, `sparkline`, `barchart`, `accordion`,
//! `bigtext`). `--width <cells>` sets the console width (112 by default; 46 for a narrow
//! terminal).

use std::env;
use std::io::{self, Write};

use lunatui_core::{Attrs, CellWidth, ClusterTable, Color, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, RunOptions, is_quit, render_once, run};
use lunatui_term::Event;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Accordion, AccordionSection, Alignment, Bar, BarChart, BigText, Block, BorderType, Date, Form,
    FormField, LineGauge, LogBuffer, LogEntry, LogLevel, LogView, Monthly, PixelSize, ProgressBar,
    ProgressTone, RadioFlow, RadioGroup, RenderCx, Sparkline, Toggle, Widget, WidgetState,
};

const WIDE: u16 = 112;
const HEIGHT: u16 = 30;

/// Moonstone palette. Surfaces, text and the four tones are theme tokens; `DIM` and `LINE` are
/// the two extra grays the console uses for hints and borders.
const BG: Color = Color::Rgb(0x17, 0x1c, 0x34);
const BAR: Color = Color::Rgb(0x26, 0x2c, 0x4c);
const FG: Color = Color::Rgb(0xd6, 0xdc, 0xf5);
const ACCENT: Color = Color::Rgb(0xf2, 0xcf, 0x7e);
const OK: Color = Color::Rgb(0x86, 0xd6, 0xc6);
const WARN: Color = Color::Rgb(0xf7, 0xa2, 0x5e);
const FAIL: Color = Color::Rgb(0xff, 0x7a, 0x8c);
const FOCUS: Color = Color::Rgb(0xc9, 0xbc, 0xff);
const DIM: Color = Color::Rgb(0x8c, 0x94, 0xb8);
const LINE: Color = Color::Rgb(0x6c, 0x75, 0x9c);

fn theme() -> Theme {
    Theme::with_overrides(
        Variant::Dark,
        &[
            (Token::Surface0, BG),
            (Token::Surface1, BAR),
            (Token::TextPrimary, FG),
            (Token::Accent, ACCENT),
            (Token::Ok, OK),
            (Token::Warn, WARN),
            (Token::Fail, FAIL),
            (Token::FocusRing, FOCUS),
        ],
    )
}

fn style(fg: Color, bg: Color) -> Style {
    Style::new().fg(fg).bg(bg)
}

struct Console {
    scene: Option<String>,
    width: u16,
}

impl App for Console {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }

    fn render(&mut self, grid: &mut Grid, _area: Rect, clusters: &mut ClusterTable) {
        draw(grid, clusters, self.scene.as_deref(), self.width);
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, scene: Option<&str>, width: u16) {
    let theme = theme();
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, &theme);
    cx.set_style(area, style(FG, BG));
    match scene {
        None => console(&mut cx, area, width),
        Some("form") => form(&mut cx, area),
        Some("progress") => progress(&mut cx, area),
        Some("logview") => logview(&mut cx, area),
        Some("calendar") => calendar(&mut cx, area),
        Some("sparkline") => sparkline(&mut cx, area),
        Some("barchart") => barchart(&mut cx, area),
        Some("accordion") => accordion(&mut cx, area),
        Some("bigtext") => bigtext(&mut cx, area),
        Some(_) => {}
    }
}

fn frame(title: &str, right: Option<(&str, Color)>) -> Block {
    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {title} "))
        .style(style(FG, BG))
        .border_style(style(LINE, BG))
        .title_style(style(ACCENT, BG));
    if let Some((text, _)) = right {
        block = block.title_top_aligned(format!(" {text} "), Alignment::Right);
    }
    block
}

fn console(cx: &mut RenderCx<'_>, area: Rect, width: u16) {
    let w = width.min(area.width);
    cx.fill(Rect::new(0, 0, w, 1), style(FG, BAR), ' ');
    cx.put_str(1, 0, "\u{25cf}", style(OK, BAR));
    cx.put_str(
        3,
        0,
        "lunar-deploy",
        style(FG, BAR).with_attrs(Attrs::new().set_bold(true)),
    );
    if w >= 70 {
        cx.put_str(18, 0, "production", style(DIM, BAR));
        cx.put_str(
            w.saturating_sub(28),
            0,
            "release v0.4.2   14:02:06",
            style(DIM, BAR),
        );
    } else {
        cx.put_str(w.saturating_sub(10), 0, "14:02:06", style(DIM, BAR));
    }
    let tabs: &[&str] = if w >= 70 {
        &["Overview", "Services", "Logs", "Settings"]
    } else {
        &["Overview", "Services", "Logs"]
    };
    let mut x = 1_u16;
    for (index, tab) in tabs.iter().enumerate() {
        let label = format!(" {tab} ");
        let len = u16::try_from(label.chars().count()).unwrap_or(0);
        let tab_style = if index == 0 {
            style(ACCENT, BAR).with_attrs(Attrs::new().set_bold(true))
        } else {
            style(DIM, BG)
        };
        cx.put_str(x, 1, &label, tab_style);
        x += len + 1;
    }
    let body_h = area.height.saturating_sub(4);
    let left_w = if w >= 70 { 40 } else { w };
    let left = Rect::new(0, 3, left_w, body_h);
    left_column(cx, left);
    if w >= 70 {
        let right = Rect::new(left_w + 2, 3, w - left_w - 2, body_h);
        right_column(cx, right);
    }
    let status = area.height.saturating_sub(1);
    cx.fill(Rect::new(0, status, w, 1), style(FG, BAR), ' ');
    let mut x = 1_u16;
    let hints: &[(&str, &str)] = if w >= 70 {
        &[
            ("tab", " views   "),
            ("/", " filter   "),
            ("r", " rerun stage   "),
            ("q", " quit"),
        ]
    } else {
        &[("tab", " views  "), ("q", " quit")]
    };
    for (key, label) in hints {
        cx.put_str(x, status, key, style(ACCENT, BAR));
        x += u16::try_from(key.chars().count()).unwrap_or(0);
        cx.put_str(x, status, label, style(FG, BAR));
        x += u16::try_from(label.chars().count()).unwrap_or(0);
    }
    cx.put_str(
        w.saturating_sub(13),
        status,
        "\u{25cf} connected",
        style(OK, BAR),
    );
}

fn left_column(cx: &mut RenderCx<'_>, area: Rect) {
    let services = Rect::new(area.x, area.y, area.width, 8);
    let block = frame("Services", Some(("5", DIM)));
    block.render(services, cx);
    let inner = block.inner(services);
    cx.put_str(
        inner.x + 1,
        inner.y,
        "service      state    p50",
        style(DIM, BG),
    );
    let rows = [
        ("api", OK, "ok", "12 ms"),
        ("worker", WARN, "warn", "40 ms"),
        ("edge", OK, "ok", "9 ms"),
        ("billing", OK, "ok", "18 ms"),
        ("search", FAIL, "down", "\u{2014}"),
    ];
    for (index, (name, tone, state, p50)) in rows.iter().enumerate() {
        let y = inner.y + 1 + u16::try_from(index).unwrap_or(0);
        let selected = index == 0;
        let bg = if selected { BAR } else { BG };
        if selected {
            cx.fill(Rect::new(inner.x, y, inner.width, 1), style(FG, BAR), ' ');
            cx.put_str(inner.x, y, "\u{258c}", style(ACCENT, BAR));
        }
        cx.put_str(inner.x + 1, y, name, style(FG, bg));
        cx.put_str(inner.x + 14, y, "\u{25cf}", style(*tone, bg));
        cx.put_str(inner.x + 16, y, state, style(FG, bg));
        cx.put_str(
            inner.x + 23,
            y,
            p50,
            style(if *p50 == "\u{2014}" { DIM } else { FG }, bg),
        );
    }

    let capacity = Rect::new(area.x, area.y + 8, area.width, 5);
    let block = frame("Capacity", None);
    block.render(capacity, cx);
    let inner = block.inner(capacity);
    for (index, (label, ratio, tone, text)) in [
        ("cpu", 0.48_f32, ACCENT, " 48%"),
        ("mem", 0.81, WARN, " 81%"),
        ("disk", 0.96, FAIL, " 96%"),
    ]
    .iter()
    .enumerate()
    {
        let y = inner.y + u16::try_from(index).unwrap_or(0);
        cx.put_str(inner.x + 1, y, label, style(DIM, BG));
        let gauge_w = inner.width.saturating_sub(14);
        LineGauge::new(*ratio)
            .filled_style(style(*tone, BG))
            .unfilled_style(style(BAR, BG))
            .render(Rect::new(inner.x + 7, y, gauge_w, 1), cx);
        cx.put_str(inner.x + 7 + gauge_w, y, text, style(FG, BG));
    }

    let release = Rect::new(
        area.x,
        area.y + 13,
        area.width,
        area.height.saturating_sub(13),
    );
    let block = frame("Release v0.4.2", None);
    block.render(release, cx);
    let inner = block.inner(release);
    let stages: [(&str, &str, Color); 4] = [
        ("checkout", "done", DIM),
        ("compile", "72%", FG),
        ("test", "1 failed", FAIL),
        ("upload", "queued", DIM),
    ];
    for (index, (name, status, tone)) in stages.iter().enumerate() {
        let y = inner.y + u16::try_from(index).unwrap_or(0) * 2;
        cx.put_str(inner.x + 1, y, name, style(FG, BG));
        let status_w = u16::try_from(status.chars().count()).unwrap_or(0);
        cx.put_str(
            inner.x + inner.width.saturating_sub(status_w + 1),
            y,
            status,
            style(*tone, BG),
        );
        let bar = Rect::new(inner.x + 1, y + 1, inner.width.saturating_sub(2), 1);
        let mut progress = ProgressBar::new().style(style(OK, BG));
        progress = match index {
            0 => progress.ratio(1.0),
            1 => progress.ratio(0.72).style(style(ACCENT, BG)),
            2 => progress
                .segment(0.81, ProgressTone::Ok)
                .segment(0.08, ProgressTone::Warn)
                .segment(0.03, ProgressTone::Fail),
            _ => progress.ratio(0.0),
        };
        progress.render(bar, cx);
    }
}

fn requests() -> Vec<u64> {
    vec![
        12, 14, 13, 17, 16, 20, 19, 23, 22, 18, 16, 15, 18, 21, 25, 29, 33, 31, 28, 24, 22, 20, 23,
        26, 24, 21, 19, 17, 18, 20, 22, 25, 27, 30, 34, 37, 35, 32, 29, 27, 25, 22, 20, 21, 23, 26,
        28, 26, 24, 23, 25, 28, 31, 30, 27, 24, 22, 21, 19, 20, 22, 24, 27, 29, 31, 33,
    ]
}

fn right_column(cx: &mut RenderCx<'_>, area: Rect) {
    let top = Rect::new(area.x, area.y, area.width, 8);
    let block = frame("Requests per minute", Some(("p50 12 ms", DIM)));
    block.render(top, cx);
    let inner = block.inner(top);
    let samples = requests();
    Sparkline::new(&samples)
        .max(40)
        .style(style(ACCENT, BG))
        .render(
            Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 5),
            cx,
        );
    cx.put_str(inner.x + 1, inner.y + 5, "60 min ago", style(DIM, BG));
    let now = "now  2,418";
    let now_w = u16::try_from(now.chars().count()).unwrap_or(0);
    cx.put_str(
        inner.x + inner.width.saturating_sub(now_w + 1),
        inner.y + 5,
        now,
        style(FG, BG).with_attrs(Attrs::new().set_bold(true)),
    );

    let bottom = Rect::new(
        area.x,
        area.y + 8,
        area.width,
        area.height.saturating_sub(8),
    );
    let block = frame("Log", Some(("following", OK)));
    block.render(bottom, cx);
    let log = logs();
    LogView::new(&log)
        .target_width(8)
        .follow(true)
        .render(block.inner(bottom), cx);
}

fn logs() -> LogBuffer {
    let mut log = LogBuffer::new(500);
    for (stamp, level, target, message) in [
        (
            "14:02:01",
            LogLevel::Info,
            "deploy",
            "pulled image lunar-api:0.4.2",
        ),
        ("14:02:02", LogLevel::Info, "deploy", "2 of 6 pods ready"),
        (
            "14:02:03",
            LogLevel::Debug,
            "cache",
            "warmed 1,284 keys in 210 ms",
        ),
        (
            "14:02:04",
            LogLevel::Warn,
            "probe",
            "/healthz answered in 412 ms (budget 250)",
        ),
        ("14:02:05", LogLevel::Info, "deploy", "4 of 6 pods ready"),
        (
            "14:02:05",
            LogLevel::Error,
            "search",
            "connection refused on :9200",
        ),
        (
            "14:02:06",
            LogLevel::Info,
            "search",
            "restarting, attempt 2 of 5",
        ),
        ("14:02:06", LogLevel::Info, "deploy", "6 of 6 pods ready"),
        ("14:02:06", LogLevel::Info, "test", "1,086 passed, 1 failed"),
        (
            "14:02:06",
            LogLevel::Warn,
            "test",
            "render::wide_cluster failed on windows",
        ),
        (
            "14:02:06",
            LogLevel::Info,
            "upload",
            "waiting for test stage",
        ),
        (
            "14:02:06",
            LogLevel::Debug,
            "edge",
            "rotated 3 tls sessions",
        ),
        ("14:02:06", LogLevel::Info, "deploy", "traffic shifted 25%"),
    ] {
        log.push(LogEntry::new(level, target, message).stamp(stamp));
    }
    log
}

fn form(cx: &mut RenderCx<'_>, area: Rect) {
    let form = Form::new([
        FormField::new("Name")
            .required()
            .hint("As it appears on invoices"),
        FormField::new("Email")
            .required()
            .error("Enter a full email address"),
        FormField::new("Plan"),
        FormField::new("Updates"),
    ])
    .focus(Some(0));
    form.render(area, cx);
    let fields = form.layout(area);
    if let Some(plan) = fields.get(2) {
        RadioGroup::new(["Free", "Pro", "Team"])
            .flow(RadioFlow::Horizontal)
            .gap(3)
            .render(plan.input, cx);
    }
    if let Some(updates) = fields.get(3) {
        Toggle::new(true)
            .label("Product news")
            .render(updates.input, cx);
    }
}

fn progress(cx: &mut RenderCx<'_>, area: Rect) {
    let stages: [(&str, &str, Color); 4] = [
        ("checkout", "done", DIM),
        ("compile", "72%", FG),
        ("test", "1 failed", FAIL),
        ("upload", "queued", DIM),
    ];
    for (index, (name, status, tone)) in stages.iter().enumerate() {
        let y = area.y + u16::try_from(index).unwrap_or(0) * 2;
        cx.put_str(area.x, y, name, style(FG, BG));
        let status_w = u16::try_from(status.chars().count()).unwrap_or(0);
        cx.put_str(
            area.x + area.width.saturating_sub(status_w),
            y,
            status,
            style(*tone, BG),
        );
        let bar = Rect::new(area.x, y + 1, area.width, 1);
        let base = ProgressBar::new().style(style(OK, BG));
        match index {
            0 => base.ratio(1.0),
            1 => base.ratio(0.72).style(style(ACCENT, BG)),
            2 => base
                .segment(0.86, ProgressTone::Ok)
                .segment(0.07, ProgressTone::Warn)
                .segment(0.03, ProgressTone::Fail),
            _ => base.ratio(0.0),
        }
        .render(bar, cx);
    }
}

fn logview(cx: &mut RenderCx<'_>, area: Rect) {
    let log = logs();
    LogView::new(&log)
        .target_width(0)
        .follow(true)
        .render(area, cx);
}

fn calendar(cx: &mut RenderCx<'_>, area: Rect) {
    let day = |d| Date::from_ymd(2026, 11, d).unwrap_or(Date::UNIX_EPOCH);
    let events = [
        (day(3), style(WARN, BG)),
        (day(12), style(OK, BG)),
        (day(24), style(ACCENT, BG)),
    ];
    Monthly::new(day(1), events.as_slice())
        .state(WidgetState::Focused)
        .render(area, cx);
}

fn sparkline(cx: &mut RenderCx<'_>, area: Rect) {
    let samples = requests();
    Sparkline::new(&samples)
        .max(40)
        .style(style(ACCENT, BG))
        .render(Rect::new(area.x, area.y, area.width.min(44), 3), cx);
    let errors: Vec<u64> = samples.iter().map(|value| value % 7).collect();
    Sparkline::new(&errors)
        .style(style(FAIL, BG))
        .render(Rect::new(area.x, area.y + 5, area.width.min(44), 1), cx);
}

fn barchart(cx: &mut RenderCx<'_>, area: Rect) {
    let bars = [
        Bar::new(34).label("Mon"),
        Bar::new(41).label("Tue"),
        Bar::new(38).label("Wed"),
        Bar::new(52).label("Thu"),
        Bar::new(61).label("Fri"),
        Bar::new(18).label("Sat"),
        Bar::new(12).label("Sun"),
    ];
    BarChart::from_bars(&bars)
        .bar_width(4)
        .bar_gap(2)
        .style(style(ACCENT, BG))
        .render(area, cx);
}

fn accordion(cx: &mut RenderCx<'_>, area: Rect) {
    let accordion = Accordion::new(vec![
        AccordionSection::new("Build", 2)
            .open(true)
            .badge("2 steps"),
        AccordionSection::new("Package", 2).badge("2 steps"),
        AccordionSection::new("Publish", 1)
            .open(true)
            .badge("1 step"),
        AccordionSection::new("Notify", 0).badge("off"),
    ])
    .focus(Some(0));
    accordion.render(area, cx);
}

fn bigtext(cx: &mut RenderCx<'_>, area: Rect) {
    BigText::new("14:02")
        .pixel_size(PixelSize::HalfHeight)
        .alignment(Alignment::Left)
        .style(style(ACCENT, BG))
        .render(Rect::new(area.x, area.y, area.width, 4), cx);
}

/// The class the site gives a cell color.
fn class_of(color: Color) -> &'static str {
    match color {
        c if c == BG => "bg",
        c if c == BAR => "sel",
        c if c == FG => "",
        c if c == ACCENT => "a",
        c if c == OK => "o",
        c if c == WARN => "w",
        c if c == FAIL => "f",
        c if c == FOCUS => "fc",
        c if c == DIM => "d",
        c if c == LINE => "ln",
        _ => "x",
    }
}

fn styled(grid: &Grid, clusters: &ClusterTable) -> String {
    let mut out = String::new();
    let text = grid.display_text(clusters);
    for (row, line) in text.lines().enumerate() {
        let row = u16::try_from(row).unwrap_or(0);
        let mut chars = line.chars();
        let mut current: Option<(String, String)> = None;
        let mut col = 0_u16;
        while col < grid.width() {
            let Some(cell) = grid.get(col, row) else {
                break;
            };
            let width = cell.meta().width;
            if width == CellWidth::Continuation {
                col += 1;
                continue;
            }
            let Some(ch) = chars.next() else {
                break;
            };
            let st = cell.style();
            let fg = class_of(st.fg);
            let bg = class_of(st.bg);
            let bold = st.attrs.bold();
            let class = [
                fg,
                if bg == "bg" { "" } else { bg },
                if bold { "b" } else { "" },
            ]
            .iter()
            .filter(|part| !part.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
            match current.as_mut() {
                Some((open, text)) if *open == class => text.push(ch),
                _ => {
                    if let Some((open, text)) = current.take() {
                        out.push_str(&format!("{open}\t{text}\n"));
                    }
                    current = Some((class, ch.to_string()));
                }
            }
            col += if width == CellWidth::Two { 2 } else { 1 };
        }
        if let Some((open, text)) = current.take() {
            out.push_str(&format!("{open}\t{text}\n"));
        }
        out.push_str("--\n");
    }
    out
}

fn option(name: &str) -> Option<String> {
    let args: Vec<String> = env::args().collect();
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1).cloned())
}

/// Runs the release console.
///
/// # Errors
///
/// Returns terminal setup, rendering, or output errors.
pub fn main() -> io::Result<()> {
    let scene = option("--scene");
    let width = option("--width")
        .and_then(|value| value.parse().ok())
        .unwrap_or(WIDE);
    let mut app = Console { scene, width };
    let plain = env::args().any(|arg| arg == "--snapshot");
    let styled_out = env::args().any(|arg| arg == "--snapshot-styled");
    if plain || styled_out {
        let (cols, rows) = match app.scene.as_deref() {
            None => (width, if width >= 70 { HEIGHT } else { 27 }),
            Some("form") => (46, 11),
            Some("progress") => (46, 8),
            Some("logview") => (48, 10),
            Some("calendar") => (25, 8),
            Some("sparkline") => (46, 7),
            Some("barchart") => (46, 10),
            Some("accordion") => (46, 7),
            Some(_) => (46, 8),
        };
        let (grid, clusters) = render_once(&mut app, cols, rows);
        let text = if styled_out {
            styled(&grid, &clusters)
        } else {
            grid.display_text(&clusters)
        };
        io::stdout().write_all(text.as_bytes())?;
        return Ok(());
    }
    run(app, RunOptions::new(width, HEIGHT))
}
