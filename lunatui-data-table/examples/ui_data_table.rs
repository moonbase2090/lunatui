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

const SERVICES: [(&str, &str, &str, u32); 40] = [
    ("api", "v2.41.0", "live", 41),
    ("web", "v8.12.3", "live", 58),
    ("worker", "v1.9.0", "rolling", 73),
    ("billing", "v3.2.1", "failed", 0),
    ("search", "v0.14.0", "live", 96),
    ("auth", "v5.0.4", "live", 22),
    ("docs", "v1.3.0", "live", 31),
    ("mail", "v2.8.8", "live", 47),
    ("queue", "v4.1.2", "live", 12),
    ("cache", "v7.0.0", "rolling", 8),
    ("metrics", "v2.2.5", "live", 19),
    ("gateway", "v6.3.1", "live", 26),
    ("ledger", "v1.4.7", "live", 35),
    ("images", "v0.9.2", "live", 88),
    ("notify", "v3.0.1", "live", 29),
    ("export", "v2.5.0", "queued", 0),
    ("import", "v2.5.0", "live", 64),
    ("reports", "v1.1.9", "live", 52),
    ("audit", "v4.4.0", "live", 17),
    ("backup", "v1.0.8", "rolling", 43),
    ("search-api", "v0.14.1", "live", 91),
    ("cdn", "v5.2.2", "live", 15),
    ("hooks", "v2.0.0", "failed", 0),
    ("admin", "v3.7.3", "live", 38),
    ("mobile", "v6.1.0", "live", 55),
    ("feeds", "v1.8.4", "live", 44),
    ("store", "v2.9.6", "live", 61),
    ("status", "v1.2.0", "live", 9),
    ("tokens", "v3.3.3", "live", 21),
    ("files", "v2.7.1", "live", 49),
    ("proxy", "v5.0.9", "live", 11),
    ("billing-api", "v3.2.2", "rolling", 57),
    ("preview", "v0.6.0", "live", 68),
    ("crawler", "v1.5.5", "queued", 0),
    ("sso", "v4.1.0", "live", 24),
    ("limits", "v1.0.3", "live", 13),
    ("archive", "v2.2.8", "live", 72),
    ("digest", "v1.9.1", "live", 33),
    ("relay", "v3.8.0", "live", 18),
    ("console", "v6.0.2", "live", 46),
];

fn wave(tick: u32, period: u32, offset: u32) -> u32 {
    let phase = (tick + offset) % period;
    let half = period / 2;
    let rise = if phase < half { phase } else { period - phase };
    rise * 100 / half
}

fn source(tick: u32) -> VecSource {
    let mut source = VecSource::new();
    for (index, (name, version, state, base)) in SERVICES.iter().enumerate() {
        let state = if *state == "rolling" && tick >= 180 {
            "live"
        } else {
            state
        };
        let latency = if *base == 0 {
            "-".to_owned()
        } else {
            let swing = wave(tick, 120, u32::try_from(index).unwrap_or(0) * 17) / 4;
            format!("{} ms", base + swing)
        };
        source.push_row(vec![
            (*name).to_owned(),
            (*version).to_owned(),
            state.to_owned(),
            latency,
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
    let source = source(tick);
    let mut state = TableState::new();
    if tick >= 150 {
        state.sort(&source, 0, SortDirection::Ascending);
    }
    DataTable::new(&source, table_columns()).sync(board, &mut state);
    state.set_cursor(Some(((tick / 15) as usize) % SERVICES.len()));
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
