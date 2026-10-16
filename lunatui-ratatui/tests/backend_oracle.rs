// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;

use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
use lunatui_bench::scenes::ratatui::log::{self, Layout};
use lunatui_bench::scenes::ratatui::{ColorOutput, encode, gradient, set_color_output, table};
use lunatui_core::Grid;
use lunatui_ratatui::{LunatuiBackend, buffer_row_text, grid_from_buffer};
use lunatui_term::{ColorMode, Encoder};
use lunatui_test::{Oracle, diff_matches};
use ratatui::backend::Backend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as RColor, Modifier, Style as RStyle};
use ratatui::{Terminal, TerminalOptions, Viewport};

const SCENE: Rect = Rect::new(0, 0, 40, 12);

#[test]
fn five_scenes_match_oracle_and_report_bytes() -> io::Result<()> {
    set_color_output(ColorOutput::Enabled);
    scene_dashboard()?;
    scene_log()?;
    scene_table()?;
    scene_gradient()?;
    scene_styled_tick()?;
    Ok(())
}

fn scene_dashboard() -> io::Result<()> {
    let mut frame = Buffer::empty(SCENE);
    dashboard::render(&Model::new(), SCENE, &mut frame);
    assert_terminal_frame("dashboard", &frame)
}

fn scene_log() -> io::Result<()> {
    let mut frame = Buffer::empty(SCENE);
    log::render(&log::text(30), 0, Layout::Wrapped, SCENE, &mut frame);
    assert_terminal_frame("log", &frame)
}

fn scene_table() -> io::Result<()> {
    let rows = table::rows(40);
    let mut frame = Buffer::empty(SCENE);
    table::render(&rows, rows.len(), SCENE, &mut frame);
    assert_terminal_frame("table", &frame)
}

fn scene_gradient() -> io::Result<()> {
    let frame = gradient::frame(SCENE);
    assert_terminal_frame("gradient", &frame)
}

fn scene_styled_tick() -> io::Result<()> {
    let area = Rect::new(0, 0, 3, 1);
    let mut previous = Buffer::empty(area);
    previous[(0, 0)].set_symbol("x").set_style(
        RStyle::default()
            .fg(RColor::Red)
            .add_modifier(Modifier::BOLD),
    );
    let mut next = Buffer::empty(area);
    next[(0, 0)].set_symbol("y").set_style(
        RStyle::default()
            .bg(RColor::Rgb(1, 2, 3))
            .add_modifier(Modifier::ITALIC | Modifier::UNDERLINED),
    );
    assert_pair("styled", &previous, &next)
}

fn assert_terminal_frame(name: &str, frame: &Buffer) -> io::Result<()> {
    let blank = Buffer::empty(frame.area);
    assert_pair(name, &blank, frame)
}

fn assert_pair(name: &str, previous: &Buffer, next: &Buffer) -> io::Result<()> {
    let area = next.area;
    let (expected, expected_clusters) = grid_from_buffer(next);
    let mut sink = io::sink();
    let mut backend = LunatuiBackend::new(area.width, area.height, &mut sink)
        .with_color_mode(ColorMode::TrueColor);
    paint_buffer(&mut backend, previous)?;
    backend.flush()?;
    paint_buffer(&mut backend, next)?;
    backend.flush()?;

    assert_grid_matches_buffer(name, backend.grid(), backend.clusters(), &expected, &expected_clusters);
    assert_ratatui_oracle_parity(name, previous, next, backend.grid(), backend.clusters())?;

    Ok(())
}

fn assert_grid_matches_buffer(
    name: &str,
    grid: &Grid,
    clusters: &lunatui_core::ClusterTable,
    expected: &Grid,
    expected_clusters: &lunatui_core::ClusterTable,
) {
    for row in 0..grid.height() {
        let got = grid.row_display(row, clusters);
        let want = expected.row_display(row, expected_clusters);
        assert_eq!(got, want, "{name} row {row}");
    }
}

fn assert_ratatui_oracle_parity(
    name: &str,
    previous: &Buffer,
    next: &Buffer,
    grid_next: &Grid,
    clusters: &lunatui_core::ClusterTable,
) -> io::Result<()> {
    let area = next.area;
    let (grid_prev, _) = grid_from_buffer(previous);
    let ours = diff_matches(&grid_prev, grid_next, clusters, ColorMode::TrueColor);
    assert!(
        ours.is_ok(),
        "{name} lunatui oracle: {}",
        ours.map_or_else(|err| err.to_string(), |()| String::new())
    );

    let blank_buf = Buffer::empty(area);
    let mut rat_prev = Vec::new();
    let mut rat_next_bytes = Vec::new();
    encode(&blank_buf, previous, &mut rat_prev)?;
    encode(previous, next, &mut rat_next_bytes)?;
    let mut rat_oracle = Oracle::new(area.height, area.width);
    rat_oracle.process(&rat_prev);
    rat_oracle.process(&rat_next_bytes);

    let mut lun_oracle = Oracle::new(area.height, area.width);
    let blank = Grid::new(grid_next.width(), grid_next.height());
    let mut encoder = Encoder::new();
    let paint = encoder
        .encode_diff(&blank, &grid_prev, clusters, ColorMode::TrueColor)
        .to_vec();
    lun_oracle.process(&paint);
    let diff = encoder
        .encode_diff(&grid_prev, grid_next, clusters, ColorMode::TrueColor)
        .to_vec();
    lun_oracle.process(&diff);
    assert_eq!(
        rat_oracle.contents(),
        lun_oracle.contents(),
        "{name} ratatui and lunatui oracle text"
    );

    for row in 0..area.height {
        assert_eq!(
            grid_next.row_display(row, clusters).trim_end(),
            buffer_row_text(next, row).trim_end(),
            "{name} buffer row {row}"
        );
    }
    Ok(())
}

fn paint_buffer<W: io::Write>(backend: &mut LunatuiBackend<W>, buffer: &Buffer) -> io::Result<()> {
    backend.clear()?;
    let area = buffer.area;
    let cells = (0..area.height).flat_map(|y| {
        (0..area.width).map(move |x| {
            let cell = &buffer[(x, y)];
            (x, y, cell)
        })
    });
    backend.draw(cells)?;
    Ok(())
}

#[test]
fn terminal_draw_flush_path() -> io::Result<()> {
    set_color_output(ColorOutput::Enabled);
    let mut bytes = Vec::new();
    {
        let backend = LunatuiBackend::new(SCENE.width, SCENE.height, &mut bytes);
        let options = TerminalOptions {
            viewport: Viewport::Fixed(SCENE),
        };
        let mut terminal = Terminal::with_options(backend, options)?;
        terminal.draw(|frame| {
            dashboard::render(&Model::new(), frame.area(), frame.buffer_mut());
        })?;
        terminal.flush()?;
    }
    let mut rat = Vec::new();
    let blank = Buffer::empty(SCENE);
    let mut frame = Buffer::empty(SCENE);
    dashboard::render(&Model::new(), SCENE, &mut frame);
    encode(&blank, &frame, &mut rat)?;
    let mut rat_oracle = Oracle::new(SCENE.height, SCENE.width);
    rat_oracle.process(&rat);
    let mut lun_oracle = Oracle::new(SCENE.height, SCENE.width);
    lun_oracle.process(&bytes);
    assert_eq!(
        rat_oracle.contents(),
        lun_oracle.contents(),
        "terminal draw flush oracle"
    );
    Ok(())
}
