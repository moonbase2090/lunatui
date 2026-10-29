// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_term::{
    ColorMode, Encoder, Event, KeyCode, Modifiers, MouseTracking, Session, SessionOptions,
    SyncOutput,
};

#[cfg(feature = "image-presenter")]
use lunatui_term::{PixelLayer, PixelMode};

use crate::app::{App, ControlFlow};
use crate::input::Input;
use crate::options::RunOptions;
use crate::sched::{SchedulePoll, Scheduler, State};

struct FrameState {
    prev: Grid,
    next: Grid,
    clusters: ClusterTable,
    encoder: Encoder,
    last_palette: Option<Vec<Color>>,
    #[cfg(feature = "image-presenter")]
    images: PixelLayer,
}

/// Runs `app` in the alternate screen until it returns [`ControlFlow::Break`] or input ends.
///
/// Requires an interactive terminal on stdin and stdout.
///
/// # Errors
///
/// Returns I/O, session, or encoder errors from the host loop.
#[cfg(any(unix, windows))]
pub fn run<A: App>(mut app: A, options: RunOptions) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lunatui_runtime::run requires an interactive terminal",
        ));
    }
    #[cfg(feature = "image-presenter")]
    let images = probe_pixel_layer();
    let mut input = Input::open()?;
    let mut session = Session::enter_with_options(
        io::stdout(),
        SessionOptions {
            sync_output: SyncOutput::Enabled,
            mouse_tracking: MouseTracking::Sgr,
        },
    )?;
    let area = Rect::new(0, 0, options.width, options.height);
    let mut frames = FrameState {
        prev: Grid::new(options.width, options.height),
        next: Grid::new(options.width, options.height),
        clusters: ClusterTable::new(),
        encoder: Encoder::new(),
        last_palette: None,
        #[cfg(feature = "image-presenter")]
        images,
    };
    let mut sched = Scheduler::idle();
    sched.on_input();
    let epoch = Instant::now();
    let mut quit = false;
    while !quit {
        let now = epoch.elapsed();
        sync_scheduler(&mut sched, &app, now);
        match sched.poll(now, false) {
            SchedulePoll::Render => {
                let present_start = epoch.elapsed();
                render_present(&mut app, area, &mut frames, &mut session)?;
                let present_done = epoch.elapsed();
                sched.after_present(present_start, present_done.saturating_sub(present_start));
            }
            SchedulePoll::SleepUntil(deadline) => {
                thread::sleep(deadline.saturating_sub(now));
            }
            SchedulePoll::WaitInput => {}
        }

        if let Some(events) = input.try_next_frame()? {
            for event in events {
                if app.update(&event) == ControlFlow::Break {
                    quit = true;
                }
            }
            sched.on_input();
            continue;
        }

        if app.needs_animation_frames() {
            continue;
        }

        match input.next_frame()? {
            None => break,
            Some(events) => {
                for event in events {
                    if app.update(&event) == ControlFlow::Break {
                        quit = true;
                    }
                }
                sched.on_input();
            }
        }
    }
    #[cfg(feature = "image-presenter")]
    {
        let clear = frames.images.clear();
        if !clear.is_empty() {
            session.present_frame(clear)?;
        }
    }
    Ok(())
}

fn sync_scheduler<A: App>(sched: &mut Scheduler, app: &A, now: Duration) {
    if app.needs_animation_frames() {
        if sched.state() == State::Idle {
            sched.resume_animation(now);
        }
    } else if sched.state() == State::Animating {
        sched.settle_idle();
    }
}

/// Runs `app` on hosts without a supported terminal backend.
#[cfg(not(any(unix, windows)))]
pub fn run<A: App>(_app: A, _options: RunOptions) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "lunatui_runtime::run requires Unix or Windows",
    ))
}

fn render_present<A: App>(
    app: &mut A,
    area: Rect,
    frames: &mut FrameState,
    session: &mut Session<impl Write>,
) -> io::Result<()> {
    frames.next.fill(
        area,
        Style::new().bg(Color::Default),
        ' ',
        &mut frames.clusters,
    );
    #[cfg(feature = "image-presenter")]
    {
        frames.images.begin_frame();
        let mut render_frame = crate::frame::RenderFrame::new(
            &mut frames.next,
            &mut frames.clusters,
            &mut frames.images,
        );
        app.render_frame(&mut render_frame, area);
    }
    #[cfg(not(feature = "image-presenter"))]
    app.render(&mut frames.next, area, &mut frames.clusters);
    let palette = app.token_palette().map(<[Color]>::to_vec);
    if palette != frames.last_palette {
        frames.prev = Grid::new(area.width, area.height);
        frames.last_palette = palette;
    }
    #[cfg(feature = "image-presenter")]
    if frames.images.needs_cell_repaint() {
        frames.prev = Grid::new(area.width, area.height);
    }
    let runs = diff_runs(&frames.prev, &frames.next);
    if let Some(colors) = frames.last_palette.as_ref() {
        frames.encoder.set_token_palette(colors.clone());
    } else {
        frames.encoder.set_token_palette(Vec::new());
    }
    let cell_bytes = frames.encoder.encode_diff(
        &frames.prev,
        &frames.next,
        &frames.clusters,
        ColorMode::TrueColor,
    );
    #[cfg(feature = "image-presenter")]
    let image_bytes = frames.images.finish_frame();
    #[cfg(feature = "image-presenter")]
    let has_images = !image_bytes.is_empty();
    #[cfg(feature = "image-presenter")]
    let payloads = [cell_bytes, image_bytes];
    #[cfg(not(feature = "image-presenter"))]
    let has_images = false;
    if !runs.is_empty() || has_images {
        #[cfg(feature = "image-presenter")]
        session.present_frame_parts(&payloads)?;
        #[cfg(not(feature = "image-presenter"))]
        session.present_frame(cell_bytes)?;
    }
    frames.prev = frames.next.clone();
    frames.next = Grid::new(area.width, area.height);
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
pub fn render_once<A: App>(app: &mut A, width: u16, height: u16) -> (Grid, ClusterTable) {
    let area = Rect::new(0, 0, width, height);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    grid.fill(area, Style::new().bg(Color::Default), ' ', &mut clusters);
    #[cfg(feature = "image-presenter")]
    {
        let mut images = PixelLayer::new(PixelMode::Cells);
        images.begin_frame();
        let mut frame = crate::frame::RenderFrame::new(&mut grid, &mut clusters, &mut images);
        app.render_frame(&mut frame, area);
    }
    #[cfg(not(feature = "image-presenter"))]
    app.render(&mut grid, area, &mut clusters);
    (grid, clusters)
}

#[cfg(all(feature = "image-presenter", unix))]
fn probe_pixel_layer() -> PixelLayer {
    use std::fs::OpenOptions;
    use std::os::fd::AsFd;

    use lunatui_term::{ProbeEnv, probe_fd};

    let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") else {
        return PixelLayer::new(PixelMode::Cells);
    };
    let environment = ProbeEnv::from_process();
    probe_fd(tty.as_fd(), &environment).map_or_else(
        |_| PixelLayer::new(PixelMode::Cells),
        |reply| PixelLayer::from_caps(&reply.caps),
    )
}

#[cfg(all(feature = "image-presenter", not(unix)))]
fn probe_pixel_layer() -> PixelLayer {
    PixelLayer::new(PixelMode::Cells)
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

        fn render(
            &mut self,
            grid: &mut lunatui_core::Grid,
            area: lunatui_core::Rect,
            clusters: &mut lunatui_core::ClusterTable,
        ) {
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
        let mut hello = Hello;
        let (grid, _) = render_once(&mut hello, 20, 5);
        assert_eq!(
            grid.get(1, 1).map(lunatui_core::Cell::glyph),
            GlyphSlot::inline_scalar('h')
        );
    }

    #[test]
    fn run_options_round_trip() {
        let options = RunOptions::new(80, 24);
        assert_eq!(options.width, 80);
        assert_eq!(options.height, 24);
    }
}
