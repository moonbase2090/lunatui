// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;

use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
use lunatui_bench::scenes::ratatui::log::{self, Layout};
use lunatui_bench::scenes::ratatui::{ColorOutput, encode, gradient, set_color_output, table};
use lunatui_core::Grid;
use lunatui_ratatui::LunatuiBackend;
use lunatui_term::ColorMode;
use lunatui_test::diff_matches;
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
    let mut sink = io::sink();
    let mut backend = LunatuiBackend::new(area.width, area.height, &mut sink)
        .with_color_mode(ColorMode::TrueColor);
    paint_buffer(&mut backend, previous)?;
    backend.flush()?;
    paint_buffer(&mut backend, next)?;
    backend.flush()?;

    let grid_next = backend.grid();
    let clusters = backend.clusters();
    let blank = Grid::new(area.width, area.height);
    let oracle_ok = diff_matches(&blank, grid_next, clusters, ColorMode::TrueColor);
    assert!(
        oracle_ok.is_ok(),
        "{name} lunatui oracle: {}",
        oracle_ok.map_or_else(|err| err.to_string(), |()| String::new())
    );

    let mut rat_bytes = Vec::new();
    let blank_buf = Buffer::empty(area);
    encode(&blank_buf, previous, &mut rat_bytes)?;
    encode(previous, next, &mut rat_bytes)?;

    let lun_bytes = backend.bytes_flushed();
    let rat_len = rat_bytes.len();
    assert!(
        lun_bytes > 0 && rat_len > 0,
        "{name} byte counts lunatui={lun_bytes} ratatui={rat_len}"
    );
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
    assert_ne!(bytes.len(), 0);
    Ok(())
}
