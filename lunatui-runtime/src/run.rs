// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, IsTerminal};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs_visible_into};
use lunatui_term::{ColorMode, Encoder, MouseTracking, Session, SessionOptions, SyncOutput};

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
    let (images, pending_input) = if options.pixel_images {
        probe_pixel_layer()
    } else {
        (PixelLayer::new(PixelMode::Cells), Vec::new())
    };
    #[cfg(feature = "image-presenter")]
    let mut input = Input::open_prefixed(pending_input)?;
    #[cfg(not(feature = "image-presenter"))]
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
                render_present(&mut app, area, &mut frames, |parts| {
                    session.present_frame_parts(parts).map_err(io::Error::from)
                })?;
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
    mut present: impl FnMut(&[&[u8]]) -> io::Result<()>,
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
    let repaint = palette != frames.last_palette;
    frames.last_palette = palette;
    #[cfg(feature = "image-presenter")]
    let repaint = repaint || frames.images.needs_cell_repaint();
    let has_cells = if repaint {
        frames.next.width() != 0 && frames.next.height() != 0
    } else {
        let mut runs = Vec::new();
        diff_runs_visible_into(&frames.prev, &frames.next, &mut runs);
        !runs.is_empty()
    };
    if let Some(colors) = frames.last_palette.as_ref() {
        frames.encoder.set_token_palette(colors.clone());
    } else {
        frames.encoder.set_token_palette(Vec::new());
    }
    let cell_bytes = if repaint {
        frames
            .encoder
            .encode_full(&frames.next, &frames.clusters, ColorMode::TrueColor)
    } else {
        frames.encoder.encode_diff(
            &frames.prev,
            &frames.next,
            &frames.clusters,
            ColorMode::TrueColor,
        )
    };
    #[cfg(feature = "image-presenter")]
    let (upload_bytes, image_bytes) = frames.images.finish_frame_parts();
    #[cfg(feature = "image-presenter")]
    let has_images = !image_bytes.is_empty();
    #[cfg(feature = "image-presenter")]
    let payloads = [cell_bytes, image_bytes];
    #[cfg(feature = "image-presenter")]
    if !upload_bytes.is_empty() {
        present(&[upload_bytes])?;
    }
    #[cfg(not(feature = "image-presenter"))]
    let has_images = false;
    if has_cells || has_images {
        #[cfg(feature = "image-presenter")]
        present(&payloads)?;
        #[cfg(not(feature = "image-presenter"))]
        present(&[cell_bytes])?;
    }
    frames.prev = frames.next.clone();
    frames.next = Grid::new(area.width, area.height);
    frames.next.seed_native_from(&frames.prev);
    Ok(())
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
fn probe_pixel_layer() -> (PixelLayer, Vec<u8>) {
    use std::fs::OpenOptions;
    use std::os::fd::AsFd;

    use lunatui_term::{ProbeEnv, probe_fd};

    let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") else {
        return (PixelLayer::new(PixelMode::Cells), Vec::new());
    };
    let environment = ProbeEnv::from_process();
    probe_fd(tty.as_fd(), &environment).map_or_else(
        |_| (PixelLayer::new(PixelMode::Cells), Vec::new()),
        |reply| (PixelLayer::from_caps(&reply.caps), reply.pending_input),
    )
}

#[cfg(all(feature = "image-presenter", not(unix)))]
fn probe_pixel_layer() -> (PixelLayer, Vec<u8>) {
    (PixelLayer::new(PixelMode::Cells), Vec::new())
}

#[cfg(test)]
mod tests {
    use lunatui_core::{ClusterTable, Color, GlyphSlot, Grid, Rect, Style};
    use lunatui_term::{Encoder, Event};

    use super::{FrameState, render_once, render_present};
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

    struct PaletteApp {
        palette: Option<Vec<Color>>,
    }

    impl App for PaletteApp {
        fn update(&mut self, _: &Event) -> ControlFlow {
            ControlFlow::Continue
        }

        fn render(&mut self, grid: &mut Grid, _: Rect, clusters: &mut ClusterTable) {
            grid.put_str(0, 0, Style::new().fg(Color::Token(0)), "界x", clusters);
        }

        fn token_palette(&self) -> Option<&[Color]> {
            self.palette.as_deref()
        }
    }

    #[test]
    fn palette_changes_and_removal_repaint_then_unchanged_frames_skip() -> Result<(), String> {
        let area = Rect::new(0, 0, 4, 1);
        let mut frames = FrameState {
            prev: Grid::new(4, 1),
            next: Grid::new(4, 1),
            clusters: ClusterTable::new(),
            encoder: Encoder::new(),
            last_palette: None,
            #[cfg(feature = "image-presenter")]
            images: super::PixelLayer::new(super::PixelMode::Cells),
        };
        let mut app = PaletteApp { palette: None };
        for (palette, expected) in [
            (Some(vec![Color::Rgb(1, 2, 3)]), "\x1b[38;2;1;2;3m界x"),
            (Some(vec![Color::Rgb(4, 5, 6)]), "\x1b[38;2;4;5;6m界x"),
            (None, "\x1b[38;5;0m界x"),
        ] {
            app.palette = palette;
            let mut output = Vec::new();
            render_present(&mut app, area, &mut frames, |parts| {
                output.extend(parts.iter().flat_map(|part| part.iter()).copied());
                Ok(())
            })
            .map_err(|error| error.to_string())?;
            assert!(String::from_utf8_lossy(&output).contains(expected));
            let mut screen = lunatui_test::Oracle::new(1, 4);
            screen.process(&output);
            assert_eq!(screen.contents(), "界x ");

            let mut presents = 0;
            render_present(&mut app, area, &mut frames, |_| {
                presents += 1;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
            assert_eq!(presents, 0, "same palette and cells must skip output");
        }
        Ok(())
    }

    struct Tinted {
        reverse: bool,
    }

    impl App for Tinted {
        fn update(&mut self, _: &Event) -> ControlFlow {
            ControlFlow::Continue
        }

        fn render(&mut self, grid: &mut Grid, _: Rect, clusters: &mut ClusterTable) {
            grid.put_str(0, 0, Style::new(), "ab", clusters);
            let first = lunatui_core::NativeAttrs::new(255, 100, 0, 0);
            let second = lunatui_core::NativeAttrs::new(255, 50, 0, 0);
            if self.reverse {
                grid.set_native(1, 0, second);
                grid.set_native(0, 0, first);
            } else {
                grid.set_native(0, 0, first);
                grid.set_native(1, 0, second);
            }
        }
    }

    fn present_once(
        app: &mut Tinted,
        area: Rect,
        frames: &mut FrameState,
    ) -> Result<usize, String> {
        let mut bytes = 0;
        render_present(app, area, frames, |parts| {
            bytes += parts.iter().map(|part| part.len()).sum::<usize>();
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        Ok(bytes)
    }

    #[test]
    fn frames_keep_attribute_indices_and_attribute_only_changes_write_nothing() -> Result<(), String>
    {
        let area = Rect::new(0, 0, 4, 1);
        let mut frames = FrameState {
            prev: Grid::new(4, 1),
            next: Grid::new(4, 1),
            clusters: ClusterTable::new(),
            encoder: Encoder::new(),
            last_palette: None,
            #[cfg(feature = "image-presenter")]
            images: super::PixelLayer::new(super::PixelMode::Cells),
        };
        let mut app = Tinted { reverse: false };
        assert!(present_once(&mut app, area, &mut frames)? > 0);
        let first = (frames.prev.cell_bits(0, 0), frames.prev.cell_bits(1, 0));
        app.reverse = true;
        assert_eq!(present_once(&mut app, area, &mut frames)?, 0);
        assert_eq!(
            (frames.prev.cell_bits(0, 0), frames.prev.cell_bits(1, 0)),
            first,
            "equal attributes keep their table index across frames"
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "image-presenter"))]
#[path = "run/pixel_tests.rs"]
mod pixel_tests;
