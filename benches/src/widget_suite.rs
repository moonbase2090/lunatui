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
use lunatui_runtime::keymap::{Command, KeyChord, Keymap};
use lunatui_term::{ColorMode, Encoder, KeyCode, Modifiers};
use lunatui_text::SliceSource;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Axis, Bar, BarChart, BigText, Block, BorderType, Breadcrumbs, Canvas, Chart, Dataset, Date,
    Form, FormField, Gauge, GraphType, KeyHintBar, List, MarkdownView, Marker, Monthly, Paragraph,
    PixelSize, RadioGroup, RenderCx, SearchBox, Slider, Sparkline, SplitAxis, SplitTree, Table,
    Tabs, TextWrap, ToastKind, ToastScope, ToastStack, Toggle, Tooltip, Widget,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint as RatConstraint, Layout as RatLayout, Rect as RatRect};
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

const HELP: &[KeyChord] = &[KeyChord::new(KeyCode::Char('?'), Modifiers::new())];
const QUIT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];

fn key_hint_map() -> &'static Keymap {
    static KEYMAP: OnceLock<Keymap> = OnceLock::new();
    KEYMAP.get_or_init(|| {
        let mut keymap = Keymap::new();
        keymap.register(Command::new("help", "Help", HELP));
        keymap.register(Command::new("quit", "Quit", QUIT));
        keymap
    })
}

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
        name: "split",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no split container; its side lays the same three panes out with Layout constraints and leaves out the divider lines and the one-cell gap lunatui reserves for them",
        ),
        lunatui: split_lunatui,
        ratatui: split_ratatui,
    },
    Case {
        name: "tabs",
        gated: false,
        parity: Parity::Exact,
        lunatui: tabs_lunatui,
        ratatui: tabs_ratatui,
    },
    Case {
        name: "breadcrumbs",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no breadcrumb widget; its side draws the full path as one line and clips it, where lunatui collapses the middle to an ellipsis when the path is wider than the area",
        ),
        lunatui: breadcrumbs_lunatui,
        ratatui: breadcrumbs_ratatui,
    },
    Case {
        name: "key-hints",
        gated: false,
        parity: Parity::Exact,
        lunatui: key_hints_lunatui,
        ratatui: key_hints_ratatui,
    },
    Case {
        name: "toggle",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no toggle switch; its side draws the same knob and track glyphs by hand as styled spans",
        ),
        lunatui: toggle_lunatui,
        ratatui: toggle_ratatui,
    },
    Case {
        name: "tooltip",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no tooltip; its side fills the rectangle Tooltip::area picks and prints the padded hint with a Paragraph",
        ),
        lunatui: tooltip_lunatui,
        ratatui: tooltip_ratatui,
    },
    Case {
        name: "radio-group",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no radio group; its side draws the same glyphs and labels by hand, one paragraph line per option",
        ),
        lunatui: radio_lunatui,
        ratatui: radio_ratatui,
    },
    Case {
        name: "form",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no form container; its side draws the same labels, required markers and messages by hand, one paragraph line each",
        ),
        lunatui: form_lunatui,
        ratatui: form_ratatui,
    },
    Case {
        name: "slider",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no slider; its side draws the same half-cell track glyphs by hand as styled spans",
        ),
        lunatui: slider_lunatui,
        ratatui: slider_ratatui,
    },
    Case {
        name: "search-box",
        gated: false,
        parity: Parity::Documented(
            "ratatui has no search field with a placeholder, match count, clear button, or smart-case filtering; its side draws the prompt, query, and count by hand as styled spans",
        ),
        lunatui: search_box_lunatui,
        ratatui: search_box_ratatui,
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
        name: "canvas",
        gated: false,
        parity: Parity::Documented(
            "ratatui samples every circle at 360 points, which leaves gaps once a circle spans more than about 90 dots; lunatui adds samples so the outline stays unbroken",
        ),
        lunatui: canvas_lunatui,
        ratatui: canvas_ratatui,
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

fn split_share(tick: u32) -> u16 {
    let phase = u16::try_from(tick % 40).unwrap_or(0);
    300 + 10 * if phase <= 20 { phase } else { 40 - phase }
}

fn split_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let (mut tree, left) = SplitTree::new();
    if let Some(right) = tree.split(left, SplitAxis::Horizontal, split_share(tick)) {
        tree.split(right, SplitAxis::Vertical, 500);
    }
    let layout = tree.layout(area);
    for pane in &layout.panes {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Pane {} ", pane.pane.0))
            .style(text_style())
            .border_style(dim_style())
            .render(pane.rect, &mut cx);
    }
    layout.render_dividers(dim_style(), None, accent_style(), &mut cx);
}

fn split_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let columns = RatLayout::horizontal([
        RatConstraint::Percentage(split_share(tick) / 10),
        RatConstraint::Fill(1),
    ])
    .split(area);
    let rows = RatLayout::vertical([RatConstraint::Percentage(50), RatConstraint::Fill(1)])
        .split(columns[1]);
    for (index, rect) in [columns[0], rows[0], rows[1]].into_iter().enumerate() {
        RatBlock::bordered()
            .border_type(RatBorderType::Rounded)
            .title(format!(" Pane {index} "))
            .style(rat_text())
            .border_style(rat_dim())
            .render(rect, buf);
    }
}

const CRUMBS: [&str; 12] = [
    "Home",
    "Projects",
    "Moonbase2090",
    "lunatui",
    "lunatui-widgets",
    "src",
    "markdown",
    "tests",
    "fixtures",
    "commonmark",
    "spec",
    "0.31.2",
];

fn breadcrumbs_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    _tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    Breadcrumbs::new(CRUMBS)
        .style(text_style())
        .current_style(accent_style())
        .separator_style(dim_style())
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
}

fn breadcrumbs_ratatui(buf: &mut Buffer, area: RatRect, _tick: u32) {
    let mut spans = Vec::new();
    for (index, name) in CRUMBS.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" › ", rat_dim()));
        }
        let style = if index + 1 == CRUMBS.len() {
            rat_accent()
        } else {
            rat_text()
        };
        spans.push(Span::styled(*name, style));
    }
    RatParagraph::new(Line::from(spans))
        .style(rat_text())
        .render(RatRect::new(area.x, area.y, area.width, 1), buf);
}

fn rat_color(color: Color) -> RatColor {
    match color {
        Color::Default | Color::Token(_) => RatColor::Reset,
        Color::Indexed(index) => RatColor::Indexed(index),
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
    }
}

fn key_hints_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    _tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    cx.set_style(area, base_style());
    cx.put_str(area.x, area.y, "RELEASE CONSOLE", text_style());
    cx.put_str(
        area.x,
        area.y.saturating_add(2),
        "Deployment v0.1.12 is ready. Review the checks before publishing.",
        dim_style(),
    );
    let hint_area = Rect::new(
        area.x,
        area.y.saturating_add(area.height).saturating_sub(1),
        area.width,
        1,
    );
    KeyHintBar::new(key_hint_map()).render(hint_area, &mut cx);
}

fn key_hints_ratatui(buf: &mut Buffer, area: RatRect, _tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let key = RatStyle::new()
        .fg(rat_color(theme.resolve(lunatui_theme::Token::Accent)))
        .bg(RatColor::Reset);
    let title = RatStyle::new()
        .fg(rat_color(theme.resolve(lunatui_theme::Token::TextPrimary)))
        .bg(RatColor::Reset);
    let header = rat_text();
    let dim = rat_dim();
    RatParagraph::new(Line::from(Span::styled("RELEASE CONSOLE", header)))
        .style(rat_base())
        .render(RatRect::new(area.x, area.y, area.width, 1), buf);
    RatParagraph::new(Line::from(Span::styled(
        "Deployment v0.1.12 is ready. Review the checks before publishing.",
        dim,
    )))
    .style(rat_base())
    .render(
        RatRect::new(area.x, area.y.saturating_add(2), area.width, 1),
        buf,
    );
    RatParagraph::new(Line::from(vec![
        Span::styled("[?]", key),
        Span::styled(" Help  ", title),
        Span::styled("[Q]", key),
        Span::styled(" Quit", title),
    ]))
    .style(rat_base())
    .render(
        RatRect::new(
            area.x,
            area.y.saturating_add(area.height).saturating_sub(1),
            area.width,
            1,
        ),
        buf,
    );
}

const TOGGLES: [&str; 4] = ["Dark mode", "Notifications", "Auto update", "Telemetry"];

fn toggle_on(tick: u32, index: usize) -> bool {
    !(tick / 4 + u32::try_from(index).unwrap_or(0)).is_multiple_of(3)
}

fn toggle_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    for (index, label) in TOGGLES.iter().enumerate() {
        let y = area.y.saturating_add(u16::try_from(index).unwrap_or(0));
        Toggle::new(toggle_on(tick, index))
            .label(*label)
            .render(Rect::new(area.x, y, area.width, 1), &mut cx);
    }
}

fn toggle_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    for (index, label) in TOGGLES.iter().enumerate() {
        let y = area.y.saturating_add(u16::try_from(index).unwrap_or(0));
        let on = toggle_on(tick, index);
        let (rail, rail_style) = if on {
            (
                "━━━",
                RatStyle::new().fg(color(lunatui_theme::Token::Accent)),
            )
        } else {
            (
                "───",
                RatStyle::new()
                    .fg(color(lunatui_theme::Token::TextPrimary))
                    .add_modifier(ratatui::style::Modifier::DIM),
            )
        };
        let knob = RatStyle::new().fg(color(if on {
            lunatui_theme::Token::Ok
        } else {
            lunatui_theme::Token::TextPrimary
        }));
        let line = if on {
            Line::from(vec![
                Span::styled(rail, rail_style),
                Span::styled("●", knob),
            ])
        } else {
            Line::from(vec![
                Span::styled("●", knob),
                Span::styled(rail, rail_style),
            ])
        };
        let mut spans = line.spans;
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            *label,
            RatStyle::new().fg(color(lunatui_theme::Token::TextPrimary)),
        ));
        RatParagraph::new(Line::from(spans)).render(RatRect::new(area.x, y, area.width, 1), buf);
    }
}

const SEARCH_QUERIES: [&str; 4] = ["sl", "tab", "on sw", ""];

fn search_box_query(tick: u32) -> &'static str {
    SEARCH_QUERIES[usize::try_from(tick / 25).unwrap_or(0) % SEARCH_QUERIES.len()]
}

fn search_box_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let query = search_box_query(tick);
    SearchBox::new(query, query.len())
        .placeholder("Filter widgets")
        .matches(None, query.len())
        .render(Rect::new(area.x, area.y, area.width.min(40), 1), &mut cx);
}

fn search_box_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    let base = RatStyle::new()
        .fg(color(lunatui_theme::Token::TextPrimary))
        .bg(color(lunatui_theme::Token::Surface1));
    let accent = RatStyle::new()
        .fg(color(lunatui_theme::Token::Accent))
        .bg(color(lunatui_theme::Token::Surface1));
    let dim = base.add_modifier(ratatui::style::Modifier::DIM);
    let query = search_box_query(tick);
    let width = usize::from(area.width.min(40));
    let shown = if query.is_empty() {
        "Filter widgets"
    } else {
        query
    };
    let count = if query.is_empty() {
        String::new()
    } else {
        format!("{} matches", query.len())
    };
    let used = 3 + shown.chars().count() + count.chars().count() + 3;
    let pad = width.saturating_sub(used);
    let mut spans = vec![
        Span::styled(" / ", accent),
        Span::styled(shown, if query.is_empty() { dim } else { base }),
        Span::styled(" ".repeat(pad), base),
        Span::styled(count, dim),
    ];
    if !query.is_empty() {
        spans.push(Span::styled(" × ", accent));
    }
    RatParagraph::new(Line::from(spans)).render(
        RatRect::new(area.x, area.y, u16::try_from(width).unwrap_or(0), 1),
        buf,
    );
}

const RADIO_OPTIONS: [&str; 4] = ["Small", "Medium", "Large", "Extra large"];

fn radio_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    RadioGroup::new(RADIO_OPTIONS)
        .select(Some(usize::try_from(tick / 4 % 4).unwrap_or(0)))
        .render(area, &mut cx);
}

fn radio_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    let selected = usize::try_from(tick / 4 % 4).unwrap_or(0);
    for (index, label) in RADIO_OPTIONS.iter().enumerate() {
        let y = area.y.saturating_add(u16::try_from(index).unwrap_or(0));
        let (glyph, token) = if index == selected {
            ("◉", lunatui_theme::Token::Accent)
        } else {
            ("○", lunatui_theme::Token::TextPrimary)
        };
        let line = Line::from(vec![
            Span::styled(glyph, RatStyle::new().fg(color(token))),
            Span::raw(" "),
            Span::styled(
                *label,
                RatStyle::new().fg(color(lunatui_theme::Token::TextPrimary)),
            ),
        ]);
        RatParagraph::new(line).render(RatRect::new(area.x, y, area.width, 1), buf);
    }
}

const FORM_LABELS: [&str; 3] = ["Name", "Email", "Notes"];

fn form_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let focus = usize::try_from(tick / 4 % 3).unwrap_or(0);
    let fields = [
        FormField::new(FORM_LABELS[0])
            .required()
            .hint("As on invoices"),
        FormField::new(FORM_LABELS[1]).required().error("Required"),
        FormField::new(FORM_LABELS[2]).hint("Optional"),
    ];
    Form::new(fields).focus(Some(focus)).render(area, &mut cx);
}

fn form_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    let focus = usize::try_from(tick / 4 % 3).unwrap_or(0);
    let messages = ["As on invoices", "✖ Required", "Optional"];
    for (index, label) in FORM_LABELS.iter().enumerate() {
        let y = area.y.saturating_add(u16::try_from(index).unwrap_or(0) * 3);
        let mut name = RatStyle::new().fg(color(if index == focus {
            lunatui_theme::Token::Accent
        } else {
            lunatui_theme::Token::TextPrimary
        }));
        if index == focus {
            name = name.add_modifier(ratatui::style::Modifier::BOLD);
        }
        let mut spans = vec![Span::styled(*label, name)];
        if index < 2 {
            spans.push(Span::styled(
                " *",
                RatStyle::new().fg(color(lunatui_theme::Token::Fail)),
            ));
        }
        RatParagraph::new(Line::from(spans)).render(RatRect::new(area.x, y, area.width, 1), buf);
        let message_style = if index == 1 {
            RatStyle::new().fg(color(lunatui_theme::Token::Fail))
        } else {
            RatStyle::new()
                .fg(color(lunatui_theme::Token::TextPrimary))
                .add_modifier(ratatui::style::Modifier::DIM)
        };
        RatParagraph::new(Line::from(Span::styled(messages[index], message_style))).render(
            RatRect::new(
                area.x.saturating_add(9),
                y.saturating_add(1),
                area.width.saturating_sub(9),
                1,
            ),
            buf,
        );
    }
}

const TOOLBAR: [(&str, &str); 4] = [
    ("Save", "Save the open file"),
    ("Open", "Open a file from disk"),
    ("Find", "Search every open file"),
    ("Close", "Close the current tab"),
];

fn toolbar_slots(area: Rect) -> impl Iterator<Item = (Rect, &'static str, &'static str)> {
    let mut x = area.x;
    TOOLBAR.iter().map(move |&(label, tip)| {
        let width = u16::try_from(label.len()).unwrap_or(0).saturating_add(2);
        let slot = Rect::new(x, area.y, width, 1);
        x = x.saturating_add(width).saturating_add(1);
        (slot, label, tip)
    })
}

fn toolbar_hovered(tick: u32) -> usize {
    usize::try_from(tick / 6).unwrap_or(0) % TOOLBAR.len()
}

fn tooltip_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let hovered = toolbar_hovered(tick);
    let mut shown = None;
    for (index, (slot, label, tip)) in toolbar_slots(area).enumerate() {
        Paragraph::new(label).render(
            Rect::new(slot.x.saturating_add(1), slot.y, slot.width - 2, 1),
            &mut cx,
        );
        if index == hovered {
            shown = Some(Tooltip::new(tip, slot));
        }
    }
    if let Some(tooltip) = shown {
        tooltip.render(area, &mut cx);
    }
}

fn tooltip_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    let screen = Rect::new(area.x, area.y, area.width, area.height);
    let hovered = toolbar_hovered(tick);
    for (index, (slot, label, tip)) in toolbar_slots(screen).enumerate() {
        RatParagraph::new(label).render(
            RatRect::new(slot.x.saturating_add(1), slot.y, slot.width - 2, 1),
            buf,
        );
        if index != hovered {
            continue;
        }
        if let Some(bubble) = Tooltip::new(tip, slot).area(screen) {
            RatParagraph::new(format!(" {tip}"))
                .style(
                    RatStyle::new()
                        .fg(color(lunatui_theme::Token::TextPrimary))
                        .bg(color(lunatui_theme::Token::Surface1)),
                )
                .render(
                    RatRect::new(bubble.x, bubble.y, bubble.width, bubble.height),
                    buf,
                );
        }
    }
}

const SLIDER_ROWS: usize = 4;

fn slider_value(tick: u32, row: usize) -> f64 {
    let step = (u64::from(tick) * 3 + u64::try_from(row).unwrap_or(0) * 17) % 200;
    let step = if step > 100 { 200 - step } else { step };
    f64::from(u32::try_from(step).unwrap_or(0))
}

fn slider_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    for row in 0..SLIDER_ROWS {
        let y = area.y.saturating_add(u16::try_from(row).unwrap_or(0));
        Slider::new(0.0, 100.0)
            .value(slider_value(tick, row))
            .render(Rect::new(area.x, y, area.width, 1), &mut cx);
    }
}

fn slider_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| rat_color(theme.resolve(token));
    let fill = RatStyle::new().fg(color(lunatui_theme::Token::Accent));
    let rest = RatStyle::new()
        .fg(color(lunatui_theme::Token::TextPrimary))
        .add_modifier(ratatui::style::Modifier::DIM);
    let total = usize::from(area.width) * 2;
    for row in 0..SLIDER_ROWS {
        let y = area.y.saturating_add(u16::try_from(row).unwrap_or(0));
        let value = slider_value(tick, row);
        let halves = (0..=total)
            .min_by(|a, b| {
                let want = value / 100.0;
                let da = (f64::from(u32::try_from(*a).unwrap_or(0))
                    / f64::from(u32::try_from(total).unwrap_or(1))
                    - want)
                    .abs();
                let db = (f64::from(u32::try_from(*b).unwrap_or(0))
                    / f64::from(u32::try_from(total).unwrap_or(1))
                    - want)
                    .abs();
                da.total_cmp(&db)
            })
            .unwrap_or(0);
        let spans: Vec<Span<'_>> = (0..usize::from(area.width))
            .map(|cell| {
                let (left, right) = (cell * 2 < halves, cell * 2 + 1 < halves);
                match (left, right) {
                    (true, true) => Span::styled("━", fill),
                    (true, false) => Span::styled("╸", fill),
                    _ => Span::styled("─", rest),
                }
            })
            .collect();
        RatParagraph::new(Line::from(spans)).render(RatRect::new(area.x, y, area.width, 1), buf);
    }
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

fn canvas_angle(tick: u32) -> f64 {
    f64::from(tick) * 0.07
}

fn canvas_lunatui(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    use lunatui_widgets::canvas::{Circle, Line, Points, Rectangle};
    let mut cx = RenderCx::new(grid, clusters, theme);
    let angle = canvas_angle(tick);
    let (sin, cos) = angle.sin_cos();
    let accent = accent_style();
    let text = text_style();
    Canvas::new(move |ctx| {
        ctx.draw(&Rectangle {
            x: -90.0,
            y: -45.0,
            width: 180.0,
            height: 90.0,
            style: dim_style(),
        });
        ctx.draw(&Circle {
            x: 0.0,
            y: 0.0,
            radius: 30.0,
            style: accent,
        });
        ctx.draw(&Circle {
            x: 40.0 * cos,
            y: 20.0 * sin,
            radius: 8.0,
            style: text,
        });
        ctx.draw(&Line {
            x1: -30.0 * cos,
            y1: -30.0 * sin,
            x2: 30.0 * cos,
            y2: 30.0 * sin,
            style: accent,
        });
        ctx.layer();
        ctx.draw(&Points {
            coords: &[
                (0.0, 0.0),
                (15.0 * sin, 15.0 * cos),
                (-60.0, 30.0),
                (60.0, -30.0),
            ],
            style: text,
        });
        ctx.print(-85.0, 40.0, "canvas", text);
    })
    .x_bounds([-100.0, 100.0])
    .y_bounds([-50.0, 50.0])
    .marker(Marker::Braille)
    .background(base_style())
    .render(area, &mut cx);
}

fn canvas_ratatui(buf: &mut Buffer, area: RatRect, tick: u32) {
    use ratatui::text::Line as TextLine;
    use ratatui::widgets::canvas::{Canvas as RatCanvas, Circle, Line, Points, Rectangle};
    let angle = canvas_angle(tick);
    let (sin, cos) = angle.sin_cos();
    let accent = rat_rgb(ACCENT);
    let text = rat_rgb(TEXT);
    let dim = rat_rgb(DIM);
    RatCanvas::default()
        .x_bounds([-100.0, 100.0])
        .y_bounds([-50.0, 50.0])
        .marker(RatMarker::Braille)
        .background_color(rat_rgb(BG))
        .paint(move |ctx| {
            ctx.draw(&Rectangle {
                x: -90.0,
                y: -45.0,
                width: 180.0,
                height: 90.0,
                color: dim,
            });
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 30.0,
                color: accent,
            });
            ctx.draw(&Circle {
                x: 40.0 * cos,
                y: 20.0 * sin,
                radius: 8.0,
                color: text,
            });
            ctx.draw(&Line {
                x1: -30.0 * cos,
                y1: -30.0 * sin,
                x2: 30.0 * cos,
                y2: 30.0 * sin,
                color: accent,
            });
            ctx.layer();
            ctx.draw(&Points {
                coords: &[
                    (0.0, 0.0),
                    (15.0 * sin, 15.0 * cos),
                    (-60.0, 30.0),
                    (60.0, -30.0),
                ],
                color: text,
            });
            ctx.print(-85.0, 40.0, TextLine::styled("canvas", rat_text()));
        })
        .render(area, buf);
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
