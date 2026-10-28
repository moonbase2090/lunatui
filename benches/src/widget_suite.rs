// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::cell::Cell;
use std::io::{self, LineWriter, Write};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;

use crate::scenes::ratatui::{ColorOutput, set_color_output};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_term::{ColorMode, Encoder};
use lunatui_text::SliceSource;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Axis, Bar, BarChart, BigText, Block, BorderType, Chart, Dataset, Date, Gauge, GraphType, List,
    MarkdownView, Marker, Monthly, Paragraph, PixelSize, RenderCx, Sparkline, Table, Tabs,
    TextWrap, ToastKind, ToastScope, ToastStack, Widget,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint as RatConstraint, Rect as RatRect};
use ratatui::style::{Color as RatColor, Style as RatStyle};
use ratatui::symbols::Marker as RatMarker;
use ratatui::text::{Line, Span};
use ratatui::widgets::calendar::{DateStyler, Monthly as RatMonthly};
use ratatui::widgets::{
    Axis as RatAxis, Bar as RatBar, BarChart as RatBarChart, BarGroup as RatBarGroup,
    Block as RatBlock, BorderType as RatBorderType, Chart as RatChart, Clear as RatClear,
    Dataset as RatDataset, Gauge as RatGauge, GraphType as RatGraphType, List as RatList, ListItem,
    Paragraph as RatParagraph, Row, Sparkline as RatSparkline, Table as RatTable, Tabs as RatTabs,
    Widget as RatWidget, Wrap,
};
use ratatui::{Terminal, TerminalOptions, Viewport};
use time::{Date as TimeDate, Month};

/// Terminal sizes every case runs at.
pub const SIZES: [(u16, u16); 2] = [(80, 24), (200, 60)];

const BG: (u8, u8, u8) = (0x1a, 0x1b, 0x26);
const TEXT: (u8, u8, u8) = (0xc0, 0xca, 0xf5);
const DIM: (u8, u8, u8) = (0x56, 0x5f, 0x89);
const ACCENT: (u8, u8, u8) = (0x7a, 0xa2, 0xf7);
const ROWS: usize = 500;
const LOG_LINES: usize = 200;
const TABS: [&str; 4] = ["Overview", "Lints", "Tests", "Deps"];

/// Whether a case's two sides draw the same cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    /// Both sides produce identical cells; the case is timed and gated.
    Exact,
    /// The sides differ for the stated reason; the case is reported but not gated.
    Documented(&'static str),
}

/// Draws one widget on the lunatui side for `tick` into `area`.
pub type LunatuiDraw = fn(&mut Grid, &mut ClusterTable, &Theme, Rect, u32);

/// Draws the equivalent ratatui widget for `tick` into `area`.
pub type RatatuiDraw = fn(&mut Buffer, RatRect, u32);

/// One widget compared with its ratatui equivalent on identical data.
pub struct Case {
    /// Short stable name used in reports and bench ids.
    pub name: &'static str,
    /// Whether the two sides are cell-identical.
    pub parity: Parity,
    /// Whether the performance gate fails the build when this case is over the bar. Widgets that
    /// predate the gate are tracked and reported until they meet it.
    pub gated: bool,
    /// The lunatui drawing function.
    pub lunatui: LunatuiDraw,
    /// The ratatui drawing function.
    pub ratatui: RatatuiDraw,
}

/// Every widget case, in report order.
pub static CASES: &[Case] = &[
    Case {
        name: "block",
        gated: false,
        parity: Parity::Exact,
        lunatui: block_lunatui,
        ratatui: block_ratatui,
    },
    Case {
        name: "paragraph",
        gated: false,
        parity: Parity::Exact,
        lunatui: paragraph_lunatui,
        ratatui: paragraph_ratatui,
    },
    Case {
        name: "table",
        gated: false,
        parity: Parity::Exact,
        lunatui: table_lunatui,
        ratatui: table_ratatui,
    },
    Case {
        name: "list",
        gated: false,
        parity: Parity::Exact,
        lunatui: list_lunatui,
        ratatui: list_ratatui,
    },
    Case {
        name: "tabs",
        gated: false,
        parity: Parity::Exact,
        lunatui: tabs_lunatui,
        ratatui: tabs_ratatui,
    },
    Case {
        name: "calendar",
        gated: true,
        parity: Parity::Exact,
        lunatui: calendar_lunatui,
        ratatui: calendar_ratatui,
    },
    Case {
        name: "gauge",
        gated: false,
        parity: Parity::Documented(
            "ratatui paints a reversed blank at the label position once the bar passes the middle, even for an empty label; lunatui's Gauge has no label",
        ),
        lunatui: gauge_lunatui,
        ratatui: gauge_ratatui,
    },
    Case {
        name: "sparkline",
        gated: true,
        parity: Parity::Exact,
        lunatui: sparkline_lunatui,
        ratatui: sparkline_ratatui,
    },
    Case {
        name: "barchart",
        gated: true,
        parity: Parity::Exact,
        lunatui: barchart_lunatui,
        ratatui: barchart_ratatui,
    },
    Case {
        name: "markdown",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no Markdown widget; its side shows the same source in a wrapped Paragraph",
        ),
        lunatui: markdown_lunatui,
        ratatui: markdown_ratatui,
    },
    Case {
        name: "toast",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no toast widget; its side draws the same three boxes by hand, in plain colors where lunatui uses theme tokens",
        ),
        lunatui: toast_lunatui,
        ratatui: toast_ratatui,
    },
    Case {
        name: "chart",
        gated: true,
        parity: Parity::Exact,
        lunatui: chart_lunatui,
        ratatui: chart_ratatui,
    },
    Case {
        name: "bigtext",
        gated: true,
        parity: Parity::Exact,
        lunatui: bigtext_lunatui,
        ratatui: bigtext_ratatui,
    },
];

fn rgb((red, green, blue): (u8, u8, u8)) -> Color {
    Color::Rgb(red, green, blue)
}

fn rat_rgb((red, green, blue): (u8, u8, u8)) -> RatColor {
    RatColor::Rgb(red, green, blue)
}

/// Background style both sides paint the whole area with before drawing.
#[must_use]
pub fn base_style() -> Style {
    Style::new().bg(rgb(BG))
}

fn rat_base() -> RatStyle {
    RatStyle::new().bg(rat_rgb(BG))
}

fn text_style() -> Style {
    base_style().fg(rgb(TEXT))
}

fn rat_text() -> RatStyle {
    rat_base().fg(rat_rgb(TEXT))
}

fn dim_style() -> Style {
    base_style().fg(rgb(DIM))
}

fn rat_dim() -> RatStyle {
    rat_base().fg(rat_rgb(DIM))
}

fn accent_style() -> Style {
    base_style().fg(rgb(ACCENT))
}

fn rat_accent() -> RatStyle {
    rat_base().fg(rat_rgb(ACCENT))
}

fn log_text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        (0..LOG_LINES)
            .map(|i| {
                format!(
                    "[{i:03}] lint: clippy::needless_borrow in src/lib.rs line {} - consider removing the borrow ",
                    i * 3
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn table_rows() -> &'static [Vec<String>] {
    static DATA: OnceLock<Vec<Vec<String>>> = OnceLock::new();
    DATA.get_or_init(|| {
        (0..ROWS)
            .map(|i| {
                vec![
                    format!("check-{i:04}"),
                    if i % 7 == 0 { "FAIL" } else { "pass" }.to_owned(),
                    format!("{}ms", (i * 37) % 900),
                    format!("src/module_{}/file_{i}.rs", i % 13),
                ]
            })
            .collect()
    })
}

fn row_slices() -> &'static [&'static [String]] {
    static SLICES: OnceLock<Vec<&'static [String]>> = OnceLock::new();
    SLICES.get_or_init(|| table_rows().iter().map(Vec::as_slice).collect())
}

fn offset(tick: u32) -> usize {
    usize::try_from(tick % 100).unwrap_or(0)
}

fn block_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" Block {} ", tick % 10))
        .style(base_style())
        .title_style(dim_style())
        .border_style(dim_style())
        .render(area, &mut cx);
}

fn block_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    RatBlock::bordered()
        .border_type(RatBorderType::Rounded)
        .title(format!(" Block {} ", tick % 10))
        .style(rat_base())
        .border_style(rat_dim())
        .render(area, buf);
}

fn paragraph_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    cx.set_style(area, text_style());
    Paragraph::new(log_text())
        .wrap(TextWrap { trim: true })
        .scroll((u16::try_from(tick % 50).unwrap_or(0), 0))
        .style(text_style())
        .render(area, &mut cx);
}

fn paragraph_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    RatParagraph::new(log_text())
        .wrap(Wrap { trim: true })
        .scroll((u16::try_from(tick % 50).unwrap_or(0), 0))
        .style(rat_text())
        .render(area, buf);
}

fn table_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    cx.set_style(area, text_style());
    Table::new(
        SliceSource::new(row_slices()),
        vec![
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Fill(1),
        ],
    )
    .style(text_style())
    .offset(offset(tick))
    .render(area, &mut cx);
}

fn table_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let rows = table_rows()
        .iter()
        .skip(offset(tick))
        .take(usize::from(area.height))
        .map(|row| Row::new(row.iter().map(String::as_str)));
    RatTable::new(
        rows,
        [
            RatConstraint::Length(12),
            RatConstraint::Length(6),
            RatConstraint::Length(7),
            RatConstraint::Fill(1),
        ],
    )
    .column_spacing(0)
    .style(rat_text())
    .render(area, buf);
}

fn list_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    cx.set_style(area, text_style());
    List::new(SliceSource::new(row_slices()))
        .style(text_style())
        .offset(offset(tick))
        .render(area, &mut cx);
}

fn list_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let items = table_rows()
        .iter()
        .skip(offset(tick))
        .take(usize::from(area.height))
        .map(|row| ListItem::new(row[0].as_str()));
    RatWidget::render(RatList::new(items).style(rat_text()), area, buf);
}

fn tabs_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    cx.set_style(Rect::new(area.x, area.y, area.width, 1), text_style());
    Tabs::new(TABS)
        .select(usize::try_from(tick % 4).unwrap_or(0))
        .style(text_style())
        .highlight_style(accent_style())
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
}

fn tabs_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let titles = TABS.map(|title| Line::from(Span::raw(title)));
    RatTabs::new(titles)
        .select(usize::try_from(tick % 4).unwrap_or(0))
        .style(rat_text())
        .highlight_style(rat_accent())
        .padding("", "")
        .divider(" ")
        .render(RatRect::new(area.x, area.y, area.width, 1), buf);
}

fn calendar_month(tick: u32) -> u8 {
    u8::try_from((tick % 12) + 1).unwrap_or(1)
}

fn calendar_date(tick: u32) -> Date {
    Date::from_ymd(2024, calendar_month(tick), 1).unwrap_or(Date::UNIX_EPOCH)
}

fn rat_month(tick: u32) -> Month {
    match tick % 12 {
        0 => Month::January,
        1 => Month::February,
        2 => Month::March,
        3 => Month::April,
        4 => Month::May,
        5 => Month::June,
        6 => Month::July,
        7 => Month::August,
        8 => Month::September,
        9 => Month::October,
        10 => Month::November,
        _ => Month::December,
    }
}

fn rat_calendar_date(tick: u32) -> TimeDate {
    TimeDate::from_calendar_date(2024, rat_month(tick), 1).unwrap_or(TimeDate::MIN)
}

struct DayThirteen;

impl DateStyler for DayThirteen {
    fn get_style(&self, date: TimeDate) -> RatStyle {
        if date.day() == 13 {
            rat_accent()
        } else {
            RatStyle::new()
        }
    }
}

fn calendar_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let date = calendar_date(tick);
    let event = Date::from_ymd(date.year(), date.month(), 13).unwrap_or(date);
    let events = [(event, accent_style())];
    Monthly::new(date, events.as_slice())
        .default_style(text_style())
        .show_month_header(accent_style())
        .show_weekdays_header(dim_style())
        .show_surrounding(dim_style())
        .render(area, &mut cx);
}

fn calendar_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let date = rat_calendar_date(tick);
    RatMonthly::new(date, DayThirteen)
        .default_style(rat_text())
        .show_month_header(rat_accent())
        .show_weekdays_header(rat_dim())
        .show_surrounding(rat_dim())
        .render(area, buf);
}

fn ratio(width: u16, tick: u32) -> f32 {
    let columns = u32::from(width.saturating_sub(2)).max(1);
    let filled = tick % (columns + 1);
    f32::from(u16::try_from(filled).unwrap_or(0)) / f32::from(u16::try_from(columns).unwrap_or(1))
}

fn gauge_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    Gauge::new(ratio(area.width, tick))
        .block(
            Block::bordered()
                .title(" Progress ")
                .style(base_style())
                .title_style(dim_style())
                .border_style(dim_style()),
        )
        .gauge_style(accent_style())
        .render(Rect::new(area.x, area.y, area.width, 3), &mut cx);
    cx.set_style(
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1),
        accent_style(),
    );
}

const SPARK_LEN: usize = 256;

const SPARK: [u64; SPARK_LEN] = {
    let mut values = [0; SPARK_LEN];
    let mut index = 0;
    while index < SPARK_LEN {
        values[index] = spark_at(index);
        index += 1;
    }
    values
};

const fn spark_at(index: usize) -> u64 {
    let index = index as u64;
    4 + (index * 17) % 48 + (index % 5) * 3
}

fn spark_window(tick: u32, width: u16) -> &'static [u64] {
    let start = usize::try_from(tick % 40).unwrap_or(0);
    let columns = usize::from(width).min(SPARK_LEN.saturating_sub(start));
    &SPARK[start..start + columns]
}

fn sparkline_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let height = area.height.min(8);
    Sparkline::new(spark_window(tick, area.width))
        .style(accent_style())
        .render(Rect::new(area.x, area.y, area.width, height), &mut cx);
}

fn sparkline_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let height = area.height.min(8);
    RatSparkline::default()
        .data(spark_window(tick, area.width))
        .style(rat_accent())
        .render(RatRect::new(area.x, area.y, area.width, height), buf);
}

const MARKDOWN: &str = "# Release notes\n\nThe **widgets** crate gained a `MarkdownView`: headings, lists, quotes, code, and tables, drawn from a parsed document.\n\n## Checklist\n\n- [x] Parse the source once\n- [x] Lay the rows out at the width\n- [ ] Scroll by rows\n  - nested items indent\n\n> A quote keeps its bar on every row, even when the text wraps to a second or third line.\n\n```rust\nlet view = MarkdownView::new(&document);\nview.render(area, &mut cx);\n```\n\n| widget | state |\n|:--|--:|\n| table | ready |\n| list | ready |\n\n---\n\nMore text follows so the view has something to scroll through when the area is short. Another paragraph with *emphasis*, ~~strikethrough~~, and a [link](https://example.com).\n";

fn markdown_document() -> &'static lunatui_text::markdown::Document {
    static DOCUMENT: OnceLock<lunatui_text::markdown::Document> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        lunatui_text::markdown::parse(MARKDOWN, lunatui_text::markdown::Options::gfm())
    })
}

fn markdown_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    MarkdownView::new(markdown_document())
        .background(text_style())
        .scroll(usize::try_from(tick % 20).unwrap_or(0))
        .render(area, &mut cx);
}

fn markdown_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    RatParagraph::new(MARKDOWN)
        .wrap(Wrap { trim: true })
        .scroll((u16::try_from(tick % 20).unwrap_or(0), 0))
        .style(rat_text())
        .render(area, buf);
}

const TOAST_TEXTS: [&str; 3] = ["Saved", "Build finished", "Disk almost full"];

fn toast_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let mut stack = ToastStack::new();
    let now = Duration::from_millis(u64::from(tick) * 100);
    for (index, text) in (0_u64..).zip(TOAST_TEXTS) {
        stack.push(
            now,
            Some(Duration::from_secs(60 + index)),
            ToastScope::Pane(0),
            ToastKind::Info,
            text,
        );
    }
    stack.render(now, area, |_| Some(area), &mut cx);
}

fn toast_ratatui(buf: &mut Buffer, area: RatRect, _tick: u32) {
    let bottom = area.y + area.height;
    for (depth, text) in TOAST_TEXTS.iter().rev().enumerate() {
        let wide = (u16::try_from(text.len()).unwrap_or(u16::MAX) + 4).min(area.width);
        let lift = u16::try_from(depth).unwrap_or(0) * 3;
        let Some(top) = bottom.checked_sub(3 + lift) else {
            break;
        };
        let rect = RatRect::new(area.x + area.width - wide, top, wide, 3);
        RatClear.render(rect, buf);
        RatBlock::bordered()
            .border_type(RatBorderType::Rounded)
            .style(rat_text())
            .render(rect, buf);
        RatParagraph::new(*text).style(rat_text()).render(
            RatRect::new(rect.x + 2, rect.y + 1, wide.saturating_sub(4), 1),
            buf,
        );
    }
}

const BAR_COUNT: usize = 64;

fn bar_values(tick: u32, width: u16) -> Vec<u64> {
    let columns = usize::from(width).div_ceil(2).min(BAR_COUNT);
    (0..columns)
        .map(|index| {
            let phase = u64::try_from(index).unwrap_or(0) * 3 + u64::from(tick);
            (phase * 37 % 101) + (phase * 11 % 29)
        })
        .collect()
}

fn barchart_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let height = area.height.min(12);
    let bars: Vec<Bar<'_>> = bar_values(tick, area.width)
        .into_iter()
        .map(Bar::new)
        .collect();
    BarChart::from_bars(&bars)
        .background(base_style())
        .max(130)
        .style(accent_style())
        .render(Rect::new(area.x, area.y, area.width, height), &mut cx);
}

fn barchart_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let height = area.height.min(12);
    let bars: Vec<RatBar<'_>> = bar_values(tick, area.width)
        .into_iter()
        .map(|value| RatBar::default().value(value).text_value(String::new()))
        .collect();
    RatBarChart::default()
        .data(RatBarGroup::default().bars(&bars))
        .bar_width(1)
        .bar_gap(1)
        .max(130)
        .bar_style(rat_accent())
        .render(RatRect::new(area.x, area.y, area.width, height), buf);
}

const CHART_POINTS: usize = 96;
const CHART_LABELS: [&str; 3] = ["0", "5", "9"];

fn chart_series(tick: u32, phase: f64) -> Vec<(f64, f64)> {
    (0..CHART_POINTS)
        .map(|index| {
            let x = f64::from(u32::try_from(index).unwrap_or(0)) * 0.25;
            let shift = f64::from(tick) * 0.15 + phase;
            (
                x,
                5.0 + 3.5 * (x * 0.55 + shift).sin() + 1.2 * (x * 1.7 + shift).cos(),
            )
        })
        .collect()
}

fn chart_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let first = chart_series(tick, 0.0);
    let second = chart_series(tick, 2.0);
    let sets = [
        Dataset::new(&first)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(accent_style()),
        Dataset::new(&second)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(text_style()),
    ];
    Chart::new(&sets)
        .background(base_style())
        .x_axis(
            Axis::new()
                .bounds([0.0, 24.0])
                .labels(&CHART_LABELS)
                .style(dim_style())
                .label_style(base_style()),
        )
        .y_axis(
            Axis::new()
                .bounds([0.0, 10.0])
                .labels(&CHART_LABELS)
                .style(dim_style())
                .label_style(base_style()),
        )
        .render(area, &mut cx);
}

fn chart_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let first = chart_series(tick, 0.0);
    let second = chart_series(tick, 2.0);
    let sets = vec![
        RatDataset::default()
            .marker(RatMarker::Braille)
            .graph_type(RatGraphType::Line)
            .style(rat_accent())
            .data(&first),
        RatDataset::default()
            .marker(RatMarker::Braille)
            .graph_type(RatGraphType::Line)
            .style(rat_text())
            .data(&second),
    ];
    RatChart::new(sets)
        .style(rat_base())
        .x_axis(
            RatAxis::default()
                .bounds([0.0, 24.0])
                .labels(CHART_LABELS)
                .style(rat_dim()),
        )
        .y_axis(
            RatAxis::default()
                .bounds([0.0, 10.0])
                .labels(CHART_LABELS)
                .style(rat_dim()),
        )
        .render(area, buf);
}

fn gauge_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    RatGauge::default()
        .ratio(f64::from(ratio(area.width, tick)))
        .label("")
        .use_unicode(false)
        .gauge_style(rat_accent())
        .block(
            RatBlock::bordered()
                .title(" Progress ")
                .style(rat_base())
                .border_style(rat_dim()),
        )
        .render(RatRect::new(area.x, area.y, area.width, 3), buf);
}

fn big_label(tick: u32) -> &'static str {
    const LABELS: [&str; 8] = [
        "REQ 1840", "REQ 1841", "REQ 1842", "REQ 1843", "REQ 1844", "REQ 1845", "REQ 1846",
        "REQ 1847",
    ];
    LABELS[usize::try_from(tick % 8).unwrap_or(0)]
}

fn bigtext_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    BigText::new(big_label(tick))
        .pixel_size(PixelSize::Quadrant)
        .style(accent_style())
        .render(area, &mut cx);
}

fn bigtext_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let text = BigText::new(big_label(tick))
        .pixel_size(PixelSize::Quadrant)
        .style(accent_style());
    let (_width, height) = PixelSize::Quadrant.cells();
    let rows = height.min(area.height);
    for row in 0..rows {
        let mut glyphs = [' '; 64];
        let columns = text.row_glyphs(row, &mut glyphs);
        let y = area.y.saturating_add(row);
        let limit = columns.min(glyphs.len()).min(usize::from(area.width));
        for (col, glyph) in glyphs.iter().take(limit).enumerate() {
            let x = area.x.saturating_add(u16::try_from(col).unwrap_or(0));
            if x >= area.x.saturating_add(area.width) || y >= area.y.saturating_add(area.height) {
                break;
            }
            buf[(x, y)].set_char(*glyph).set_style(rat_accent());
        }
    }
}

/// Shared byte and `write()` counters for the frame sink.
#[derive(Clone, Default)]
pub struct Counters {
    bytes: Rc<Cell<usize>>,
    writes: Rc<Cell<usize>>,
}

impl Counters {
    /// Bytes written so far.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.bytes.get()
    }

    /// `write()` calls so far.
    #[must_use]
    pub fn writes(&self) -> usize {
        self.writes.get()
    }
}

struct CountingSink(Counters);

impl Write for CountingSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.bytes.set(self.0.bytes.get() + buf.len());
        self.0.writes.set(self.0.writes.get() + 1);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The lunatui side of one case at one size, reusing its buffers across frames.
pub struct LunatuiRun {
    case: &'static Case,
    area: Rect,
    theme: Theme,
    prev: Grid,
    next: Grid,
    clusters: ClusterTable,
    encoder: Encoder,
    sink: CountingSink,
    /// Frame counter passed to the case.
    pub tick: u32,
}

impl LunatuiRun {
    /// Sets up `case` at `width`×`height`.
    #[must_use]
    pub fn new(case: &'static Case, width: u16, height: u16) -> Self {
        Self {
            case,
            area: Rect::new(0, 0, width, height),
            theme: Theme::builtin(Variant::Dark),
            prev: Grid::new(width, height),
            next: Grid::new(width, height),
            clusters: ClusterTable::new(),
            encoder: Encoder::new(),
            sink: CountingSink(Counters::default()),
            tick: 0,
        }
    }

    /// Byte and write counters of the sink.
    #[must_use]
    pub fn counters(&self) -> Counters {
        self.sink.0.clone()
    }

    /// Clears and draws the next tick into the reused grid.
    pub fn render(&mut self) {
        self.next
            .fill(self.area, base_style(), ' ', &mut self.clusters);
        (self.case.lunatui)(
            &mut self.next,
            &mut self.clusters,
            &self.theme,
            self.area,
            self.tick,
        );
        self.tick = self.tick.wrapping_add(1);
    }

    /// Draws, diffs against the previous frame, encodes and writes it.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the sink fails.
    pub fn frame(&mut self) -> io::Result<()> {
        self.render();
        let payload =
            self.encoder
                .encode_diff(&self.prev, &self.next, &self.clusters, ColorMode::TrueColor);
        self.sink.write_all(payload)?;
        std::mem::swap(&mut self.prev, &mut self.next);
        Ok(())
    }
}

type RatTerminal = Terminal<CrosstermBackend<LineWriter<CountingSink>>>;

/// The ratatui side of one case at one size, using a real `Terminal` for frames.
pub struct RatatuiRun {
    case: &'static Case,
    area: RatRect,
    buffer: Buffer,
    terminal: RatTerminal,
    counters: Counters,
    /// Frame counter passed to the case.
    pub tick: u32,
}

impl RatatuiRun {
    /// Sets up `case` at `width`×`height`.
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal cannot be created.
    pub fn new(case: &'static Case, width: u16, height: u16) -> io::Result<Self> {
        let area = RatRect::new(0, 0, width, height);
        let counters = Counters::default();
        let backend = CrosstermBackend::new(LineWriter::new(CountingSink(counters.clone())));
        let terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(area),
            },
        )?;
        Ok(Self {
            case,
            area,
            buffer: Buffer::empty(area),
            terminal,
            counters,
            tick: 0,
        })
    }

    /// Byte and write counters of the sink.
    #[must_use]
    pub fn counters(&self) -> Counters {
        self.counters.clone()
    }

    /// Clears and draws the next tick into the reused buffer.
    pub fn render(&mut self) {
        self.buffer.reset();
        self.buffer.set_style(self.area, rat_base());
        (self.case.ratatui)(&mut self.buffer, self.area, self.tick);
        self.tick = self.tick.wrapping_add(1);
    }

    /// Draws one frame through `Terminal::draw`, which diffs and writes it.
    ///
    /// # Errors
    ///
    /// Returns an error if drawing fails.
    pub fn frame(&mut self) -> io::Result<()> {
        let case = self.case;
        let area = self.area;
        let tick = self.tick;
        self.terminal.draw(|frame| {
            frame.buffer_mut().set_style(area, rat_base());
            (case.ratatui)(frame.buffer_mut(), area, tick);
        })?;
        self.tick = self.tick.wrapping_add(1);
        Ok(())
    }
}

/// Renders `case` at `tick` on both sides and returns the lunatui grid and ratatui buffer.
#[must_use]
pub fn render_both(
    case: &'static Case,
    width: u16,
    height: u16,
    tick: u32,
) -> (Grid, ClusterTable, Buffer) {
    let theme = Theme::builtin(Variant::Dark);
    let area = Rect::new(0, 0, width, height);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    grid.fill(area, base_style(), ' ', &mut clusters);
    (case.lunatui)(&mut grid, &mut clusters, &theme, area, tick);
    let rat_area = RatRect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(rat_area);
    buffer.set_style(rat_area, rat_base());
    (case.ratatui)(&mut buffer, rat_area, tick);
    (grid, clusters, buffer)
}

/// Frame-ratio limit the gate uses when `--max-ratio` is not given.
pub const DEFAULT_MAX_RATIO: f64 = 1.05;

/// Makes ratatui's crossterm backend write colors whatever `NO_COLOR` says, so both sides encode
/// truecolor and the byte counts do not depend on the environment.
pub fn pin_color_output() {
    set_color_output(ColorOutput::Enabled);
}

/// Reads `--max-ratio N` from `args`; absent means [`DEFAULT_MAX_RATIO`].
///
/// # Errors
///
/// Returns a message when the flag has no value or the value is not a finite number above zero.
pub fn parse_max_ratio(args: &[String]) -> Result<f64, String> {
    let Some(position) = args.iter().position(|arg| arg == "--max-ratio") else {
        return Ok(DEFAULT_MAX_RATIO);
    };
    let value = args
        .get(position + 1)
        .ok_or_else(|| "--max-ratio needs a value".to_owned())?;
    match value.parse::<f64>() {
        Ok(ratio) if ratio.is_finite() && ratio > 0.0 => Ok(ratio),
        _ => Err(format!(
            "--max-ratio must be a finite number above zero, got {value}"
        )),
    }
}

/// Reasons a gated case is over the bar; empty when it passes. A ratio that is not a number
/// counts as over the bar.
#[must_use]
pub fn gate_failures(
    label: &str,
    frame_ratio: f64,
    limit: f64,
    lunatui_allocs: f64,
    ratatui_allocs: f64,
) -> Vec<String> {
    let mut failures = Vec::new();
    if frame_ratio
        .partial_cmp(&limit)
        .is_none_or(std::cmp::Ordering::is_gt)
    {
        failures.push(format!("{label} frame ratio {frame_ratio:.2} > {limit:.2}"));
    }
    if lunatui_allocs
        .partial_cmp(&ratatui_allocs)
        .is_none_or(std::cmp::Ordering::is_gt)
    {
        failures.push(format!(
            "{label} allocations {lunatui_allocs:.1} > {ratatui_allocs:.1}"
        ));
    }
    failures
}
