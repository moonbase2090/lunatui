// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, IsTerminal, Write};
use std::time::Instant;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_term::{
    ColorMode, Encoder, Event, KeyCode, Modifiers, Session, SessionOptions, SyncOutput,
};

use crate::app::{App, ControlFlow};
use crate::input::Input;
use crate::options::RunOptions;
use crate::sched::{SchedulePoll, Scheduler};

/// Runs `app` in the alternate screen until it returns [`ControlFlow::Break`] or input ends.
///
/// Requires a Unix terminal on stdin and stdout. Non-interactive stdout is rejected.
///
/// # Errors
///
/// Returns I/O, session, or encoder errors from the host loop.
#[cfg(unix)]
pub fn run<A: App>(mut app: A, options: RunOptions) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lunatui_runtime::run requires an interactive terminal",
        ));
    }
    let stdin = io::stdin();
    let mut input = Input::from_reader(stdin)?;
    let mut session = Session::enter_with_options(
        io::stdout(),
        SessionOptions {
            sync_output: SyncOutput::Enabled,
        },
    )?;
    let area = Rect::new(0, 0, options.width, options.height);
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut sched = Scheduler::idle();
    sched.on_input();
    let epoch = Instant::now();
    let mut quit = false;
    while !quit {
        let now = epoch.elapsed();
        if matches!(sched.poll(now, false), SchedulePoll::Render) {
            let present_start = epoch.elapsed();
            render_present(
                &app,
                area,
                &mut prev,
                &mut next,
                &mut clusters,
                &mut encoder,
                &mut session,
            )?;
            let present_done = epoch.elapsed();
            sched.after_present(present_start, present_done.saturating_sub(present_start));
        }
        let Some(events) = input.next_frame()? else {
            break;
        };
        for event in events {
            if app.update(&event) == ControlFlow::Break {
                quit = true;
            }
        }
        sched.on_input();
    }
    Ok(())
}

/// Runs `app` on non-Unix hosts (unsupported).
#[cfg(not(unix))]
pub fn run<A: App>(_app: A, _options: RunOptions) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "lunatui_runtime::run requires Unix",
    ))
}

fn render_present<A: App>(
    app: &A,
    area: Rect,
    prev: &mut Grid,
    next: &mut Grid,
    clusters: &mut ClusterTable,
    encoder: &mut Encoder,
    session: &mut Session<impl Write>,
) -> io::Result<()> {
    next.fill(area, Style::new().bg(Color::Default), ' ', clusters);
    app.render(next, area, clusters);
    let runs = diff_runs(prev, next);
    let payload = encoder.encode_diff(prev, next, clusters, ColorMode::TrueColor);
    if !runs.is_empty() {
        session.present_frame(payload)?;
    }
    *prev = next.clone();
    *next = Grid::new(area.width, area.height);
    Ok(())
}

/// Returns true when `event` is a plain `q` press (quit).
#[must_use]
pub fn is_quit(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(key) if key.code == KeyCode::Char('q') && key.modifiers == Modifiers::new()
    )
}

/// Renders `app` once into a fresh grid (for snapshots and benches).
pub fn render_once<A: App>(
    app: &A,
    width: u16,
    height: u16,
) -> (Grid, ClusterTable) {
    let area = Rect::new(0, 0, width, height);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    grid.fill(area, Style::new().bg(Color::Default), ' ', &mut clusters);
    app.render(&mut grid, area, &mut clusters);
    (grid, clusters)
}

#[cfg(test)]
mod tests {
    use lunatui_core::GlyphSlot;
    use lunatui_term::{Event, KeyCode, KeyEvent};

    use super::{is_quit, render_once};
    use crate::app::{App, ControlFlow};
    use crate::options::RunOptions;

    struct Hello;

    impl App for Hello {
        fn update(&mut self, _event: &Event) -> ControlFlow {
            ControlFlow::Continue
        }

        fn render(&self, grid: &mut lunatui_core::Grid, area: lunatui_core::Rect, clusters: &mut lunatui_core::ClusterTable) {
            grid.put_str(
                area.x + 1,
                area.y + 1,
                lunatui_core::Style::new(),
                "hello",
                clusters,
            );
        }
    }

    #[test]
    fn is_quit_matches_plain_q() {
        assert!(is_quit(&Event::Key(KeyEvent::press(KeyCode::Char('q')))));
        assert!(!is_quit(&Event::Key(KeyEvent::press(KeyCode::Char('x')))));
    }

    #[test]
    fn render_once_writes_hello() {
        let (grid, _) = render_once(&Hello, 20, 5);
        assert_eq!(grid.get(1, 1).map(lunatui_core::Cell::glyph), GlyphSlot::inline_scalar('h'));
    }

    #[test]
    fn run_options_round_trip() {
        let options = RunOptions::new(80, 24);
        assert_eq!(options.width, 80);
        assert_eq!(options.height, 24);
    }
}
