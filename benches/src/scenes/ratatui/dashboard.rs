// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Gauge, Paragraph, Row, Table, Tabs, Widget, Wrap};

const ROWS: usize = 500;
const LOG_LINES: usize = 200;
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// The dashboard's state: the animation tick, the scroll offset, 500 check rows and a 200-line log.
#[derive(Clone, Debug)]
pub struct Model {
    /// Animation tick. It moves the rail highlight, the selected tab, the gauge and the spinner.
    pub tick: u32,
    /// Rows scrolled off the top of the table and the log.
    pub scroll: usize,
    rows: Vec<[String; 4]>,
    log: String,
}

impl Model {
    /// Builds the model at tick 0 and scroll 0.
    #[must_use]
    pub fn new() -> Self {
        let rows = (0..ROWS)
            .map(|i| {
                let state = if i.is_multiple_of(7) { "FAIL" } else { "pass" };
                [
                    format!("check-{i:04}"),
                    state.to_owned(),
                    format!("{}ms", (i * 37) % 900),
                    format!("src/module_{}/file_{}.rs", i % 13, i),
                ]
            })
            .collect();
        let log = (0..LOG_LINES)
            .map(|i| {
                format!(
                    "[{i:03}] lint: clippy::needless_borrow in src/lib.rs line {} — consider removing the borrow ",
                    i * 3
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            tick: 0,
            scroll: 0,
            rows,
            log,
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders the dashboard for `model` into `area` of `buf`.
pub fn render(model: &Model, area: Rect, buf: &mut Buffer) {
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    buf.set_style(area, Style::new().bg(bg));
    let [rail, main] = Layout::horizontal([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    for y in rail.top()..rail.bottom() {
        let color = if (u32::from(y) + model.tick).is_multiple_of(12) {
            accent
        } else {
            dim
        };
        buf[(rail.x + 1, y)].set_symbol("│").set_fg(color);
    }
    let [tabs, body, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(main);
    let titles = ["● Overview", "○ Lints", "● Tests", "○ Deps"].map(|title| {
        Line::from(vec![
            Span::styled(&title[..3], Style::new().fg(Color::Green)),
            Span::raw(&title[3..]),
        ])
    });
    Tabs::new(titles)
        .select(selected_tab(model.tick))
        .highlight_style(Style::new().fg(accent).bold())
        .render(tabs, buf);
    let [left, right] = Layout::horizontal([Constraint::Percentage(60), Constraint::Fill(1)])
        .spacing(1)
        .areas(body);
    render_checks(model, left, dim, buf);
    let [log, gauge] = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(right);
    Paragraph::new(model.log.as_str())
        .wrap(Wrap { trim: true })
        .scroll((u16::try_from(model.scroll).unwrap_or(u16::MAX), 0))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Log "),
        )
        .render(log, buf);
    Gauge::default()
        .ratio(f64::from(model.tick % 100) / 100.0)
        .gauge_style(Style::new().fg(accent))
        .block(Block::bordered().title(" Progress "))
        .render(gauge, buf);
    let spinner = SPINNER[spinner_frame(model.tick)];
    Line::from(vec![
        spinner.cyan(),
        " scorecard • 3 failing • ".into(),
        format!("frame {}", model.tick).dim(),
    ])
    .render(status, buf);
}

fn render_checks(model: &Model, area: Rect, dim: Color, buf: &mut Buffer) {
    let rows = model
        .rows
        .iter()
        .skip(model.scroll)
        .take(usize::from(area.height))
        .map(|row| {
            let state_style = if row[1] == "FAIL" {
                Style::new().fg(Color::Red).bold()
            } else {
                Style::new().fg(Color::Green)
            };
            Row::new(vec![
                Cell::from(row[0].as_str()),
                Span::styled(row[1].as_str(), state_style).into(),
                row[2].as_str().into(),
                row[3].as_str().into(),
            ])
        });
    Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Fill(1),
        ],
    )
    .header(Row::new(vec!["check", "state", "time", "file"]).add_modifier(Modifier::BOLD))
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Checks ")
            .border_style(Style::new().fg(dim)),
    )
    .render(area, buf);
}

fn selected_tab(tick: u32) -> usize {
    usize::try_from(tick / 30 % 4).unwrap_or(0)
}

fn spinner_frame(tick: u32) -> usize {
    usize::try_from(tick % 10).unwrap_or(0)
}

/// Frames of the dashboard used to measure the diff and the encoder, built as in ratatui-perf.
#[derive(Clone, Debug)]
pub struct Frames {
    /// An empty buffer, the previous frame of a first draw.
    pub empty: Buffer,
    /// The dashboard at tick 0, scroll 0.
    pub first: Buffer,
    /// The next tick: only the rail, tabs, gauge, spinner and frame counter change.
    pub tick: Buffer,
    /// The tick frame scrolled by one row: the table and log move too.
    pub scroll: Buffer,
}

impl Frames {
    /// Renders the four frames at `area`.
    #[must_use]
    pub fn new(area: Rect) -> Self {
        let mut model = Model::new();
        let first = rendered(&model, area);
        model.tick += 1;
        let tick = rendered(&model, area);
        model.scroll += 1;
        let scroll = rendered(&model, area);
        Self {
            empty: Buffer::empty(area),
            first,
            tick,
            scroll,
        }
    }
}

fn rendered(model: &Model, area: Rect) -> Buffer {
    let mut buf = Buffer::empty(area);
    render(model, area, &mut buf);
    buf
}
