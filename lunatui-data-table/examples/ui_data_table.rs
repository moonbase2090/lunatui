// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_data_table::{
    Alignment, Column, Constraint, DataTable, SortDirection, TableState, VecSource,
};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::Role;
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

const SERVICES: [(&str, &str, &str, &str); 12] = [
    ("api", "v2.41.0", "live", "41 ms"),
    ("web", "v8.12.3", "live", "58 ms"),
    ("worker", "v1.9.0", "rolling", "73 ms"),
    ("billing", "v3.2.1", "failed", "-"),
    ("search", "v0.14.0", "live", "96 ms"),
    ("auth", "v5.0.4", "live", "22 ms"),
    ("docs", "v1.3.0", "live", "31 ms"),
    ("mail", "v2.8.8", "live", "47 ms"),
    ("queue", "v4.1.2", "live", "12 ms"),
    ("cache", "v7.0.0", "rolling", "8 ms"),
    ("metrics", "v2.2.5", "live", "19 ms"),
    ("gateway", "v6.3.1", "live", "26 ms"),
];

fn source() -> VecSource {
    let mut source = VecSource::new();
    for (name, version, state, latency) in SERVICES {
        source.push_row(vec![
            name.to_owned(),
            version.to_owned(),
            state.to_owned(),
            latency.to_owned(),
        ]);
    }
    source
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = Style::new().bg(cx.token(Role::Background.token()));
    Block::new().style(backdrop).render(area, &mut cx);
    let title = Style::new()
        .fg(cx.token(Role::Primary.token()))
        .bg(cx.token(Role::Background.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Deploy board")
        .style(title)
        .render(Rect::new(2, 0, area.width.saturating_sub(2), 1), &mut cx);

    let board = Rect::new(
        1,
        2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(3),
    );
    let source = source();
    let mut state = TableState::new();
    if tick >= 120 {
        state.sort(&source, 0, SortDirection::Ascending);
    }
    DataTable::new(&source, table_columns()).sync(board, &mut state);
    state.set_cursor(Some(((tick / 20) as usize) % SERVICES.len()));
    DataTable::new(&source, table_columns())
        .state(&state)
        .render(board, &mut cx);
}

fn table_columns() -> Vec<Column<'static>> {
    vec![
        Column::new("Service", Constraint::Length(12)),
        Column::new("Version", Constraint::Length(10)),
        Column::new("State", Constraint::Length(10)),
        Column::new("p95", Constraint::Fill(1)).align(Alignment::Right),
    ]
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let flag = |name: &str, default: u32| -> u32 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let width = u16::try_from(flag("--width", 80)).unwrap_or(80);
    let height = u16::try_from(flag("--height", 24)).unwrap_or(24);
    let theme = Theme::builtin(Variant::Dark);
    let area = Rect::new(0, 0, width, height);
    let mut clusters = ClusterTable::new();
    let mut next = Grid::new(width, height);
    if args.iter().any(|a| a == "--snapshot") {
        draw(&mut next, &mut clusters, &theme, area, 0);
        print!("{}", next.display_text(&clusters));
        return Ok(());
    }
    let frames = flag("--frames", 270);
    let fps = flag("--fps", 30).max(1);
    let period = Duration::from_secs(1) / fps;
    let mut previous = Grid::new(width, height);
    let mut encoder = Encoder::new();
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let started = Instant::now();
    for tick in 0..frames {
        draw(&mut next, &mut clusters, &theme, area, tick);
        let mut frame = b"\x1b[?2026h".to_vec();
        frame.extend_from_slice(encoder.encode_diff(
            &previous,
            &next,
            &clusters,
            ColorMode::TrueColor,
        ));
        frame.extend_from_slice(b"\x1b[?2026l");
        out.write_all(&frame)?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        let due = started + period * (tick + 1);
        thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    out.flush()
}
