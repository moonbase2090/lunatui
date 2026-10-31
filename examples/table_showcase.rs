// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, LineGauge, Paragraph, RenderCx, RowSource, SortDirection, StatusBar, Table,
    TableState, VecSource, Widget,
};

const SERVICES: [(&str, &str, &str, u32, u32); 16] = [
    ("api-gateway", "edge", "eu-west", 182, 12),
    ("auth", "identity", "us-east", 96, 4),
    ("billing", "payments", "us-east", 341, 31),
    ("catalog", "commerce", "eu-west", 128, 7),
    ("checkout", "payments", "eu-west", 412, 58),
    ("email", "growth", "ap-south", 74, 2),
    ("images", "media", "us-west", 233, 9),
    ("inventory", "commerce", "us-east", 157, 6),
    ("ledger", "payments", "eu-west", 265, 14),
    ("notifications", "growth", "us-west", 88, 3),
    ("orders", "commerce", "ap-south", 301, 22),
    ("pricing", "commerce", "us-west", 119, 5),
    ("recommend", "growth", "eu-west", 377, 11),
    ("search", "discovery", "us-east", 143, 8),
    ("sessions", "identity", "ap-south", 61, 1),
    ("shipping", "logistics", "eu-west", 208, 17),
];

const COLUMNS: [&str; 6] = ["Service", "Team", "Region", "p95 ms", "Errors", "Deploys"];

#[derive(Clone, Copy)]
enum Step {
    Down(u32),
    Point(u32, u16, u16),
    SortClick(u32),
    Extend(u32),
    Press(u32),
    Drag(u32, u16),
    Release(u32),
}

const STEPS: [Step; 17] = [
    Step::Down(10),
    Step::Down(16),
    Step::Down(22),
    Step::Down(28),
    Step::Point(60, 3, 0),
    Step::SortClick(66),
    Step::Down(100),
    Step::Extend(112),
    Step::Extend(120),
    Step::Extend(128),
    Step::Point(165, 0, 0),
    Step::Press(170),
    Step::Drag(178, 4),
    Step::Drag(186, 8),
    Step::Drag(194, 12),
    Step::Release(200),
    Step::Point(250, 0, 0),
];

struct App {
    state: TableState,
    key: &'static str,
    note: String,
    pointer: Option<(u16, u16, bool)>,
}

fn latency(base: u32, row: usize, tick: u32) -> u32 {
    let wave = (tick / 10 + u32::try_from(row).unwrap_or(0) * 7) % 13;
    base + wave * base / 40
}

fn rows(tick: u32) -> VecSource {
    VecSource::from_rows(
        SERVICES
            .iter()
            .enumerate()
            .map(|(row, (name, team, region, p95, errors))| {
                let errors = errors + (tick / 30 + u32::try_from(row).unwrap_or(0)) % 3;
                vec![
                    (*name).to_owned(),
                    (*team).to_owned(),
                    (*region).to_owned(),
                    latency(*p95, row, tick).to_string(),
                    format!("{}.{}%", errors / 10, errors % 10),
                    ((row * 5 + 3) % 17).to_string(),
                ]
            })
            .collect(),
    )
}

fn columns() -> Vec<Constraint> {
    vec![
        Constraint::Length(15),
        Constraint::Length(11),
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Fill(1),
    ]
}

fn replay(source: &VecSource, rows_shown: usize, tick: u32) -> App {
    let mut app = App {
        state: TableState::new(),
        key: "",
        note: String::from("16 services, live"),
        pointer: None,
    };
    app.state.fit(source.len(), rows_shown);
    app.state.set_cursor(Some(0));
    for step in STEPS {
        match step {
            Step::Down(at) if tick >= at => {
                app.key = "↓";
                app.state.down(1, false);
            }
            Step::Point(at, column, row) if tick + 25 >= at => {
                app.pointer = Some((column, row, false));
            }
            Step::SortClick(at) if tick >= at => {
                app.key = "click";
                app.state.sort(&rows(at), 3, SortDirection::Descending);
                app.note = String::from("Sorted by p95, slowest first");
            }
            Step::Extend(at) if tick >= at => {
                app.key = "Shift+↓";
                app.state.down(1, true);
                if let Some(range) = app.state.selection() {
                    app.note = format!("{} services selected", range.end() - range.start() + 1);
                }
            }
            Step::Press(at) if tick >= at => {
                app.key = "drag";
                app.state.start_resize(0, 15, 0);
                app.note = String::from("Dragging the Service divider");
            }
            Step::Drag(at, dx) if tick >= at => app.state.drag_resize(dx),
            Step::Release(at) if tick >= at => {
                app.state.end_resize();
                app.note = String::from("Service column widened by drag");
            }
            _ => {}
        }
    }
    app
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let text = theme.resolve(Token::TextPrimary);
    let fill = Style::new().fg(text).bg(theme.resolve(Token::Surface0));
    let raised = Style::new().fg(text).bg(theme.resolve(Token::Surface1));
    let accent = theme.resolve(Token::Accent);
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(fill).render(area, &mut cx);
    if area.width < 100 || area.height < 30 {
        return;
    }
    let seconds = 20 + tick / 30;
    Paragraph::new(format!(
        " acme · service inventory{:>width$}",
        format!("16:08:{seconds:02} "),
        width = usize::from(area.width) - 26
    ))
    .style(raised.fg(accent).with_attrs(Attrs::new().set_bold(true)))
    .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);

    let pane = Rect::new(area.x + 1, area.y + 2, area.width - 34, area.height - 4);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Services ")
        .style(fill)
        .border_style(fill.fg(theme.resolve(Token::FocusRing)))
        .title_style(
            fill.fg(theme.resolve(Token::FocusRing))
                .with_attrs(Attrs::new().set_bold(true)),
        );
    block.render(pane, &mut cx);
    let inner = block.inner(pane);
    let table_area = Rect::new(inner.x + 1, inner.y, inner.width - 2, inner.height);
    let source = rows(tick);
    let shown = Table::new(&source, columns())
        .header(COLUMNS.map(|title| (title, Style::new())))
        .data_rows(table_area);
    let mut app = replay(&source, shown, tick);
    app.state.fit(source.len(), shown);
    let warn = theme.resolve(Token::Warn);
    let fail = theme.resolve(Token::Fail);
    let ok = theme.resolve(Token::Ok);
    let cursor = app.state.cursor();
    let highlight = |row: usize, column: usize| -> Style {
        if Some(row) == cursor {
            return Style::new();
        }
        let source_row = app.state.source_row(row);
        let p95 = source
            .cell(source_row, 3)
            .and_then(|cell| cell.parse::<u32>().ok())
            .unwrap_or(0);
        let errors = source
            .cell(source_row, 4)
            .and_then(|cell| cell.trim_end_matches('%').parse::<f32>().ok())
            .unwrap_or(0.0);
        match column {
            3 if p95 >= 350 => Style::new()
                .fg(fail)
                .with_attrs(Attrs::new().set_bold(true)),
            3 if p95 >= 250 => Style::new().fg(warn),
            4 if errors >= 3.0 => Style::new()
                .fg(fail)
                .with_attrs(Attrs::new().set_bold(true)),
            4 if errors >= 1.5 => Style::new().fg(warn),
            4 => Style::new().fg(ok),
            _ => Style::new(),
        }
    };
    let header_style = fill.fg(accent).with_attrs(Attrs::new().set_bold(true));
    let table = Table::new(app.state.view(&source), columns())
        .header(COLUMNS.map(|title| (title, header_style)))
        .alignments(vec![
            lunatui_widgets::Alignment::Left,
            lunatui_widgets::Alignment::Left,
            lunatui_widgets::Alignment::Left,
            lunatui_widgets::Alignment::Right,
            lunatui_widgets::Alignment::Right,
            lunatui_widgets::Alignment::Right,
        ])
        .state(&app.state)
        .style(fill)
        .highlight_style(
            Style::new()
                .fg(theme.resolve(Token::Surface0))
                .bg(theme.resolve(Token::FocusRing))
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .selection_style(Style::new().bg(theme.resolve(Token::Surface1)))
        .highlight_symbol("▸ ", Style::new())
        .cell_styles(highlight);
    table.render(table_area, &mut cx);

    let side = Rect::new(
        pane.x + pane.width + 1,
        pane.y,
        area.width - pane.width - 3,
        pane.height,
    );
    details(&mut cx, &app, &source, side, fill, theme, tick);

    status(&mut cx, &app, area, raised, accent, theme);
    pointer(&mut cx, &app, &table, table_area, fill, theme, tick);
}

fn status(
    cx: &mut RenderCx<'_>,
    app: &App,
    area: Rect,
    raised: Style,
    accent: Color,
    theme: &Theme,
) {
    StatusBar::new("")
        .style(raised)
        .left([
            (
                if app.key.is_empty() {
                    String::new()
                } else {
                    format!(" key {} ", app.key)
                },
                raised.fg(theme.resolve(Token::FocusRing)),
            ),
            (format!(" {} ", app.note), raised.fg(accent)),
        ])
        .right([(
            String::from(" ↑↓ move  Shift+↓ select  click sort  drag resize "),
            raised.with_attrs(Attrs::new().set_dim(true)),
        )])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            cx,
        );
}

fn pointer<S: RowSource, C: lunatui_widgets::CellStyle>(
    cx: &mut RenderCx<'_>,
    app: &App,
    table: &Table<S, C>,
    table_area: Rect,
    fill: Style,
    theme: &Theme,
    tick: u32,
) {
    if let Some((column, _, _)) = app
        .pointer
        .filter(|_| (35..80).contains(&tick) || (140..215).contains(&tick))
    {
        let target = if tick < 165 {
            let header = table.column_areas(table_area)[usize::from(column)];
            (header.x + 3, table_area.y)
        } else {
            let first = table.column_areas(table_area)[0];
            (first.x + first.width - 1, table_area.y)
        };
        let pressed = (66..70).contains(&tick) || (170..200).contains(&tick);
        Paragraph::new(if pressed { "●" } else { "↖" })
            .style(
                fill.fg(theme.resolve(Token::FocusRing))
                    .with_attrs(Attrs::new().set_bold(true)),
            )
            .render(Rect::new(target.0, target.1, 1, 1), cx);
    }
}

fn details(
    cx: &mut RenderCx<'_>,
    app: &App,
    source: &VecSource,
    pane: Rect,
    fill: Style,
    theme: &Theme,
    tick: u32,
) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Details ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let Some(cursor) = app.state.cursor() else {
        return;
    };
    let row = app.state.source_row(cursor);
    let cell = |column: usize| source.cell(row, column).unwrap_or("");
    let line = |cx: &mut RenderCx<'_>, y: u16, text: String, style: Style| {
        Paragraph::new(text)
            .style(style)
            .render(Rect::new(inner.x + 1, inner.y + y, inner.width - 2, 1), cx);
    };
    line(
        cx,
        0,
        cell(0).to_owned(),
        fill.fg(theme.resolve(Token::Accent))
            .with_attrs(Attrs::new().set_bold(true)),
    );
    line(cx, 1, format!("{} · {}", cell(1), cell(2)), dim);
    line(cx, 3, format!("p95      {} ms", cell(3)), fill);
    line(cx, 4, format!("errors   {}", cell(4)), fill);
    line(cx, 5, format!("deploys  {} this week", cell(5)), fill);
    let p95 = cell(3).parse::<f32>().unwrap_or(0.0);
    line(cx, 7, String::from("latency budget 400 ms"), dim);
    let tone: Color = if p95 >= 350.0 {
        theme.resolve(Token::Fail)
    } else if p95 >= 250.0 {
        theme.resolve(Token::Warn)
    } else {
        theme.resolve(Token::Ok)
    };
    LineGauge::new((p95 / 400.0).min(1.0))
        .filled_style(fill.fg(tone))
        .unfilled_style(dim)
        .render(Rect::new(inner.x + 1, inner.y + 8, inner.width - 2, 1), cx);
    if let Some(range) = app
        .state
        .selection()
        .filter(|range| range.end() > range.start())
    {
        line(
            cx,
            10,
            format!("{} selected", range.end() - range.start() + 1),
            fill.fg(theme.resolve(Token::FocusRing)),
        );
        for (offset, view) in range.enumerate() {
            let name = source.cell(app.state.source_row(view), 0).unwrap_or("");
            line(
                cx,
                11 + u16::try_from(offset).unwrap_or(0),
                format!("  {name}"),
                dim,
            );
        }
    }
    let refresh = (tick % 30) * 100 / 29;
    line(cx, inner.height - 2, format!("refresh {refresh:>3}%"), dim);
    LineGauge::new(f32::from(u8::try_from(refresh).unwrap_or(100)) / 100.0)
        .filled_style(fill.fg(theme.resolve(Token::Accent)))
        .unfilled_style(dim)
        .render(
            Rect::new(inner.x + 1, inner.y + inner.height - 1, inner.width - 2, 1),
            cx,
        );
}
