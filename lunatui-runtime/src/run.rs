// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, IsTerminal};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs_visible_into};
use lunatui_term::{
    CellPixels, ColorMode, Encoder, Event, MouseTracking, Session, SessionOptions, SyncOutput,
    terminal_cell_pixels,
};

#[cfg(all(feature = "image-presenter", unix))]
use lunatui_image::ImageSource;
#[cfg(all(feature = "image-presenter", unix))]
use lunatui_prismattyc::native_image::{
    ImageKey, ImagePresenter as NativeImagePresenter, ImageSession as NativeImageSession,
    PreparedImage,
};
#[cfg(all(feature = "image-presenter", unix))]
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

#[cfg(feature = "image-presenter")]
use lunatui_term::{PixelLayer, PixelMode};

use crate::app::{App, ControlFlow};
use crate::input::Input;
#[cfg(feature = "image-presenter")]
use crate::mode::{Capabilities, RenderMode};
use crate::options::RunOptions;
use crate::sched::{SchedulePoll, Scheduler, State};

struct FrameState {
    prev: Grid,
    next: Grid,
    clusters: ClusterTable,
    encoder: Encoder,
    last_palette: Option<Vec<Color>>,
    repaint: bool,
    pointer: Option<CellPixels>,
    #[cfg(feature = "image-presenter")]
    images: PixelLayer,
    #[cfg(all(feature = "image-presenter", unix))]
    native_images: Option<NativeImagePresenter>,
    #[cfg(feature = "image-presenter")]
    mode: RenderMode,
}

impl FrameState {
    fn resize(&mut self, area: &mut Rect, width: u16, height: u16) {
        if width == 0 || height == 0 || (area.width, area.height) == (width, height) {
            return;
        }
        *area = Rect::new(0, 0, width, height);
        self.prev = Grid::new(width, height);
        self.next = Grid::new(width, height);
        self.repaint = true;
    }
}

/// Runs `app` in the alternate screen until it returns [`ControlFlow::Break`] or input ends.
///
/// Requires an interactive terminal on stdin and stdout.
///
/// # Errors
///
/// Returns I/O, session, or encoder errors from the host loop.
#[cfg(any(unix, windows))]
#[expect(
    clippy::too_many_lines,
    reason = "The terminal event loop keeps frame rendering, input, and native image lifecycle ordered in one loop."
)]
pub fn run<A: App>(mut app: A, options: RunOptions) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lunatui_runtime::run requires an interactive terminal",
        ));
    }
    #[cfg(feature = "image-presenter")]
    let (images, pending_input, pointer) = if options.pixel_images {
        probe_pixel_layer()
    } else {
        (PixelLayer::new(PixelMode::Cells), Vec::new(), None)
    };
    #[cfg(all(feature = "image-presenter", unix))]
    let mut pending_input = pending_input;
    #[cfg(not(feature = "image-presenter"))]
    let pointer = None::<CellPixels>;
    let tracking = if pointer.is_some() {
        MouseTracking::SgrPixels
    } else {
        MouseTracking::Sgr
    };
    let mut session = Session::enter_with_options(
        io::stdout(),
        SessionOptions::default()
            .sync_output(SyncOutput::Enabled)
            .mouse_tracking(tracking),
    )?;
    #[cfg(all(feature = "image-presenter", unix))]
    let mut native_session = activate_native_images(options, &mut pending_input);
    #[cfg(all(feature = "image-presenter", unix))]
    let mut native_last_ping = Instant::now();
    #[cfg(all(feature = "image-presenter", unix))]
    let mut native_preparation = native_session
        .as_ref()
        .and_then(|_| NativePreparation::start().ok());
    #[cfg(feature = "image-presenter")]
    let mut input = Input::open_prefixed(pending_input, pointer.is_some())?;
    #[cfg(not(feature = "image-presenter"))]
    let mut input = Input::open()?;
    let mut area = Rect::new(0, 0, options.width, options.height);
    let mut frames = FrameState {
        prev: Grid::new(options.width, options.height),
        next: Grid::new(options.width, options.height),
        clusters: ClusterTable::new(),
        encoder: Encoder::new(),
        last_palette: None,
        repaint: false,
        pointer,
        #[cfg(feature = "image-presenter")]
        images,
        #[cfg(all(feature = "image-presenter", unix))]
        native_images: native_session
            .as_ref()
            .map(|native| NativeImagePresenter::new(native.cell_pixels())),
        #[cfg(feature = "image-presenter")]
        mode: RenderMode::Cell,
    };
    #[cfg(feature = "image-presenter")]
    {
        frames.mode = options
            .mode
            .with_env_override()
            .resolve(&capabilities(&frames.images));
    }
    let mut sched = Scheduler::idle();
    sched.on_input();
    let epoch = Instant::now();
    let mut quit = false;
    while !quit {
        let now = epoch.elapsed();
        #[cfg(all(feature = "image-presenter", unix))]
        if let (Some(native), Some(preparation)) =
            (native_session.as_mut(), native_preparation.as_mut())
            && let Some((key, source, prepared)) = preparation.try_result()
        {
            if let Ok(prepared) = prepared {
                let result = native
                    .upload_prepared(key, &source, prepared)
                    .and_then(|reply| {
                        frames
                            .native_images
                            .as_mut()
                            .ok_or_else(|| io::Error::other("native image sink ended"))?
                            .admission(key, &reply)
                    });
                if result.is_err() {
                    fail_native_session(
                        &mut native_session,
                        &mut frames,
                        &mut session,
                        &mut sched,
                    )?;
                    native_preparation = None;
                    continue;
                }
            } else if let Some(images) = frames.native_images.as_mut() {
                images.decline_preparation(key);
            }
            sched.on_input();
        }
        #[cfg(all(feature = "image-presenter", unix))]
        if native_session.is_some() && native_last_ping.elapsed() >= Duration::from_millis(100) {
            native_last_ping = Instant::now();
            let heartbeat = native_session
                .as_mut()
                .and_then(|native| native.ping().ok());
            let pong = heartbeat.and_then(|reply| {
                frames
                    .native_images
                    .as_mut()
                    .map(|images| images.pong(&reply.bytes))
            });
            let Some(pong) = pong else {
                fail_native_session(&mut native_session, &mut frames, &mut session, &mut sched)?;
                native_preparation = None;
                continue;
            };
            match pong {
                Ok(true) => sched.on_input(),
                Ok(false) => {}
                Err(_) => {
                    fail_native_session(
                        &mut native_session,
                        &mut frames,
                        &mut session,
                        &mut sched,
                    )?;
                    native_preparation = None;
                    continue;
                }
            }
        }
        sync_scheduler(&mut sched, &app, now);
        match sched.poll(now, false) {
            SchedulePoll::Render => {
                let present_start = epoch.elapsed();
                let mut ansi_present =
                    |parts: &[&[u8]]| session.present_frame_parts(parts).map_err(io::Error::from);
                #[cfg(all(feature = "image-presenter", unix))]
                let native_result = render_present_with_native(
                    &mut app,
                    area,
                    &mut frames,
                    &mut ansi_present,
                    native_session.as_mut(),
                );
                #[cfg(all(feature = "image-presenter", unix))]
                if let Err(error) = native_result {
                    if native_session.is_some() {
                        fail_native_session(
                            &mut native_session,
                            &mut frames,
                            &mut session,
                            &mut sched,
                        )?;
                        native_preparation = None;
                        continue;
                    }
                    return Err(error);
                }
                #[cfg(not(all(feature = "image-presenter", unix)))]
                render_present(&mut app, area, &mut frames, &mut ansi_present)?;
                #[cfg(all(feature = "image-presenter", unix))]
                let native_result = (|| -> io::Result<()> {
                    if let (Some(native), Some(images)) =
                        (native_session.as_mut(), frames.native_images.as_mut())
                    {
                        if let Some(preparation) = native_preparation.as_mut() {
                            if !preparation.is_busy()
                                && let Some((key, source)) = images.take_request()
                            {
                                preparation.submit(key, source)?;
                            }
                        } else if let Some((key, _source)) = images.take_request() {
                            images.decline_preparation(key);
                        }
                        if let Some((key, handle)) = images.retirement() {
                            let response = native
                                .retire(handle, native.frame_sequence())
                                .map_err(io::Error::other)?;
                            images.retired(key, &response.bytes)?;
                        }
                    }
                    Ok(())
                })();
                #[cfg(all(feature = "image-presenter", unix))]
                if let Err(error) = native_result {
                    if native_session.is_some() {
                        fail_native_session(
                            &mut native_session,
                            &mut frames,
                            &mut session,
                            &mut sched,
                        )?;
                        continue;
                    }
                    return Err(error);
                }
                let present_done = epoch.elapsed();
                sched.after_present(present_start, present_done.saturating_sub(present_start));
            }
            SchedulePoll::SleepUntil(deadline) => {
                let wait = deadline.saturating_sub(now);
                #[cfg(all(feature = "image-presenter", unix))]
                let wait = if native_session.is_some() {
                    wait.min(Duration::from_millis(100))
                } else {
                    wait
                };
                thread::sleep(wait);
            }
            SchedulePoll::WaitInput => {}
        }

        if let Some(events) = input.try_next_frame()? {
            quit |= dispatch(&mut app, events, &mut area, &mut frames);
            sched.on_input();
            continue;
        }

        if app.needs_animation_frames() {
            continue;
        }

        #[cfg(all(feature = "image-presenter", unix))]
        let next_input = if native_session.is_some() {
            input.next_frame_timeout(Duration::from_millis(100))?
        } else {
            input.next_frame()?
        };
        #[cfg(not(all(feature = "image-presenter", unix)))]
        let next_input = input.next_frame()?;
        match next_input {
            None => {
                #[cfg(all(feature = "image-presenter", unix))]
                if native_session.is_some() && !input.is_finished() {
                    continue;
                }
                break;
            }
            Some(events) => {
                quit |= dispatch(&mut app, events, &mut area, &mut frames);
                sched.on_input();
            }
        }
    }
    #[cfg(all(feature = "image-presenter", unix))]
    if let Some(mut native) = native_session.take() {
        let _ = native.release();
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

#[cfg(all(feature = "image-presenter", unix))]
fn activate_native_images(
    options: RunOptions,
    pending_input: &mut Vec<u8>,
) -> Option<NativeImageSession> {
    use std::fs::OpenOptions;

    let stream = lunatui_prismattyc::native_image::inherited_stream()
        .ok()
        .flatten()?;
    if !options.pixel_images {
        return None;
    }
    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()?;
    let discovery = lunatui_prismattyc::native_image::discover(&mut tty).ok()?;
    pending_input.extend(discovery.preserved_input);
    let offer = discovery.offer?;
    let grid = Grid::new(options.width, options.height);
    let clusters = ClusterTable::new();
    NativeImageSession::activate(stream, &offer, &grid, &clusters, Vec::new()).ok()
}

#[cfg(all(feature = "image-presenter", unix))]
trait FallbackTerminal {
    fn present_fallback(&mut self, bytes: &[u8]) -> io::Result<()>;
}

#[cfg(all(feature = "image-presenter", unix))]
impl<W: std::io::Write> FallbackTerminal for Session<W> {
    fn present_fallback(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.present_frame(bytes).map_err(io::Error::other)
    }
}

#[cfg(all(feature = "image-presenter", unix))]
fn fail_native_session<T>(
    native: &mut Option<T>,
    frames: &mut FrameState,
    terminal: &mut impl FallbackTerminal,
    scheduler: &mut Scheduler,
) -> io::Result<()> {
    native.take();
    frames.native_images = None;
    frames.repaint = true;
    terminal.present_fallback(lunatui_prismattyc::ansi_fallback_repaint_prefix())?;
    scheduler.on_input();
    Ok(())
}

#[cfg(all(feature = "image-presenter", unix))]
struct NativePreparation {
    sender: Sender<(ImageKey, ImageSource)>,
    receiver: Receiver<(ImageKey, ImageSource, io::Result<PreparedImage>)>,
    pending: bool,
}

#[cfg(all(feature = "image-presenter", unix))]
impl NativePreparation {
    fn start() -> io::Result<Self> {
        let (sender, jobs) = mpsc::channel::<(ImageKey, ImageSource)>();
        let (results, receiver) = mpsc::channel();
        thread::Builder::new()
            .name(String::from("lunatui-image-prepare"))
            .spawn(move || {
                while let Ok((key, source)) = jobs.recv() {
                    let prepared = PreparedImage::prepare(source.rgba8_pixels());
                    if results.send((key, source, prepared)).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender,
            receiver,
            pending: false,
        })
    }

    fn submit(&mut self, key: ImageKey, source: ImageSource) -> io::Result<()> {
        if self.pending {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "native image preparation is busy",
            ));
        }
        self.sender
            .send((key, source))
            .map_err(|_| io::Error::other("native image preparation stopped"))?;
        self.pending = true;
        Ok(())
    }

    fn is_busy(&self) -> bool {
        self.pending
    }

    fn try_result(&mut self) -> Option<(ImageKey, ImageSource, io::Result<PreparedImage>)> {
        match self.receiver.try_recv() {
            Ok(result) => {
                self.pending = false;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = false;
                None
            }
        }
    }
}

fn dispatch<A: App>(
    app: &mut A,
    events: Vec<Event>,
    area: &mut Rect,
    frames: &mut FrameState,
) -> bool {
    let mut quit = false;
    for event in events {
        if let Event::Resize(width, height) = event {
            frames.resize(area, width, height);
            if let Some(now) = terminal_cell_pixels() {
                if let Some(cell) = frames.pointer.as_mut() {
                    *cell = now;
                }
                #[cfg(feature = "image-presenter")]
                if frames.images.mode().is_pixels() {
                    frames.images.set_cell_pixels(now);
                }
            }
        }
        let event = match (event, frames.pointer) {
            (Event::Mouse(mouse), Some(cell)) => Event::Mouse(mouse.in_cells(cell)),
            (event, _) => event,
        };
        if app.update(&event) == ControlFlow::Break {
            quit = true;
        }
    }
    quit
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

#[cfg(any(not(all(feature = "image-presenter", unix)), test))]
fn render_present<A: App>(
    app: &mut A,
    area: Rect,
    frames: &mut FrameState,
    mut present: impl FnMut(&[&[u8]]) -> io::Result<()>,
) -> io::Result<()> {
    #[cfg(all(feature = "image-presenter", unix))]
    return render_present_with_native(app, area, frames, &mut present, None);
    #[cfg(not(all(feature = "image-presenter", unix)))]
    render_present_with_native(app, area, frames, &mut present)
}

fn render_present_with_native<A: App>(
    app: &mut A,
    area: Rect,
    frames: &mut FrameState,
    present: &mut impl FnMut(&[&[u8]]) -> io::Result<()>,
    #[cfg(all(feature = "image-presenter", unix))] mut native: Option<&mut NativeImageSession>,
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
        #[cfg(all(feature = "image-presenter", unix))]
        let image_sink: &mut dyn lunatui_image::ImageSink =
            if let Some(images) = frames.native_images.as_mut() {
                images.begin_frame();
                images
            } else {
                &mut frames.images
            };
        #[cfg(not(all(feature = "image-presenter", unix)))]
        let image_sink: &mut dyn lunatui_image::ImageSink = &mut frames.images;
        let mut render_frame = crate::frame::RenderFrame::new(
            &mut frames.next,
            &mut frames.clusters,
            image_sink,
            frames.mode,
        );
        app.render_frame(&mut render_frame, area);
    }
    #[cfg(not(feature = "image-presenter"))]
    app.render(&mut frames.next, area, &mut frames.clusters);
    let palette = app.token_palette().map(<[Color]>::to_vec);
    let repaint = palette != frames.last_palette || std::mem::take(&mut frames.repaint);
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
    #[cfg(all(feature = "image-presenter", unix))]
    if let Some(native) = native.as_mut() {
        native.set_token_palette(frames.last_palette.clone().unwrap_or_default());
        let placements = frames
            .native_images
            .as_ref()
            .map_or(&[][..], NativeImagePresenter::placements);
        native
            .present(&frames.next, &frames.clusters, placements)
            .map_err(io::Error::other)?;
    } else if has_cells || has_images {
        #[cfg(feature = "image-presenter")]
        present(&payloads)?;
        #[cfg(not(feature = "image-presenter"))]
        present(&[cell_bytes])?;
    }
    #[cfg(not(all(feature = "image-presenter", unix)))]
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
        let mut frame =
            crate::frame::RenderFrame::new(&mut grid, &mut clusters, &mut images, RenderMode::Cell);
        app.render_frame(&mut frame, area);
    }
    #[cfg(not(feature = "image-presenter"))]
    app.render(&mut grid, area, &mut clusters);
    (grid, clusters)
}

#[cfg(feature = "image-presenter")]
pub(crate) fn capabilities(layer: &PixelLayer) -> Capabilities {
    use std::num::NonZeroU16;

    use crate::mode::GraphiteCaps;

    let cell = layer.cell_pixels().filter(|_| layer.mode().is_pixels());
    cell.and_then(|cell| {
        Some(Capabilities::with_graphite(
            GraphiteCaps::new(
                NonZeroU16::new(cell.width())?,
                NonZeroU16::new(cell.height())?,
            )
            .proportional_text(true)
            .alpha(layer.mode() == PixelMode::Kitty),
        ))
    })
    .unwrap_or(Capabilities::CELL_ONLY)
}

#[cfg(all(feature = "image-presenter", unix))]
fn probe_pixel_layer() -> (PixelLayer, Vec<u8>, Option<CellPixels>) {
    use std::fs::OpenOptions;
    use std::os::fd::AsFd;

    use lunatui_term::{PrivateMode, ProbeEnv, probe_fd};

    let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") else {
        return (PixelLayer::new(PixelMode::Cells), Vec::new(), None);
    };
    let environment = ProbeEnv::from_process();
    probe_fd(tty.as_fd(), &environment).map_or_else(
        |_| (PixelLayer::new(PixelMode::Cells), Vec::new(), None),
        |reply| {
            let pointer = reply.caps.cell_pixels.filter(|_| {
                matches!(
                    reply.caps.modes.sgr_pixel_mouse,
                    PrivateMode::Set | PrivateMode::Reset
                )
            });
            (
                PixelLayer::from_caps(&reply.caps),
                reply.pending_input,
                pointer,
            )
        },
    )
}

#[cfg(all(feature = "image-presenter", not(unix)))]
fn probe_pixel_layer() -> (PixelLayer, Vec<u8>, Option<CellPixels>) {
    (PixelLayer::new(PixelMode::Cells), Vec::new(), None)
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

    #[cfg(feature = "image-presenter")]
    #[test]
    fn a_cell_only_backend_gives_every_frame_cell_mode() {
        struct Probe(Vec<crate::RenderMode>);
        impl App for Probe {
            fn update(&mut self, _: &Event) -> ControlFlow {
                ControlFlow::Continue
            }
            fn render(&mut self, _: &mut Grid, _: Rect, _: &mut ClusterTable) {}
            fn render_frame(&mut self, frame: &mut crate::RenderFrame<'_>, _: Rect) {
                self.0.push(frame.mode());
            }
        }
        let mut probe = Probe(Vec::new());
        let _ = render_once(&mut probe, 4, 2);
        assert_eq!(probe.0, [crate::RenderMode::Cell]);
    }

    #[test]
    fn run_options_ask_for_the_automatic_policy_until_told_otherwise() {
        use crate::ModePolicy;
        let options = RunOptions::new(80, 24);
        assert_eq!(options.mode, ModePolicy::Auto);
        assert_eq!(options.mode(ModePolicy::Cell).mode, ModePolicy::Cell);
    }

    #[test]
    fn run_options_round_trip() {
        let options = RunOptions::new(80, 24);
        assert_eq!(options.width, 80);
        assert_eq!(options.height, 24);
    }

    #[cfg(all(feature = "image-presenter", unix))]
    #[test]
    fn native_failure_repaints_cells_on_the_virtual_scheduler_clock() {
        use std::{cell::RefCell, rc::Rc, time::Duration};

        struct SharedOutput(Rc<RefCell<Vec<u8>>>);
        struct Fallback;
        impl App for Fallback {
            fn update(&mut self, _: &Event) -> ControlFlow {
                ControlFlow::Continue
            }
            fn render(&mut self, grid: &mut Grid, _: Rect, clusters: &mut ClusterTable) {
                grid.put_str(0, 0, Style::new(), "fallback", clusters);
            }
        }

        impl super::FallbackTerminal for SharedOutput {
            fn present_fallback(&mut self, bytes: &[u8]) -> std::io::Result<()> {
                self.0.borrow_mut().extend_from_slice(bytes);
                Ok(())
            }
        }
        let output = Rc::new(RefCell::new(Vec::new()));
        let mut terminal = SharedOutput(Rc::clone(&output));
        let mut frames = FrameState {
            prev: Grid::new(12, 1),
            next: Grid::new(12, 1),
            clusters: ClusterTable::new(),
            encoder: Encoder::new(),
            last_palette: None,
            repaint: false,
            pointer: None,
            native_images: None,
            images: super::PixelLayer::new(super::PixelMode::Cells),
            mode: super::RenderMode::Cell,
        };
        let mut native = Some(());
        let mut scheduler = crate::sched::Scheduler::animating(Duration::from_millis(16));

        assert!(
            super::fail_native_session(&mut native, &mut frames, &mut terminal, &mut scheduler)
                .is_ok()
        );
        assert!(
            native.is_none(),
            "the failed native session must be discarded"
        );
        assert!(
            frames.native_images.is_none(),
            "the native image sink must be discarded"
        );
        assert!(
            frames.repaint,
            "the next cell render must repaint the full grid"
        );
        assert_eq!(
            scheduler.poll(Duration::ZERO, false),
            crate::sched::SchedulePoll::Render
        );
        let mut app = Fallback;
        assert!(
            super::render_present(&mut app, Rect::new(0, 0, 12, 1), &mut frames, |parts| {
                let mut output = output.borrow_mut();
                for part in parts {
                    output.extend_from_slice(part);
                }
                Ok(())
            })
            .is_ok()
        );
        let bytes = output.borrow();
        assert!(
            bytes
                .windows(b"fallback".len())
                .any(|window| window == b"fallback")
        );
        assert!(
            bytes
                .windows(b"\x1bc\x1b[2J\x1b[H".len())
                .any(|window| window == b"\x1bc\x1b[2J\x1b[H")
        );
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
            repaint: false,
            pointer: None,
            #[cfg(all(feature = "image-presenter", unix))]
            native_images: None,
            #[cfg(feature = "image-presenter")]
            images: super::PixelLayer::new(super::PixelMode::Cells),
            #[cfg(feature = "image-presenter")]
            mode: super::RenderMode::Cell,
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
            repaint: false,
            pointer: None,
            #[cfg(all(feature = "image-presenter", unix))]
            native_images: None,
            #[cfg(feature = "image-presenter")]
            images: super::PixelLayer::new(super::PixelMode::Cells),
            #[cfg(feature = "image-presenter")]
            mode: super::RenderMode::Cell,
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

    struct Frame {
        areas: Vec<Rect>,
        events: Vec<Event>,
    }

    impl App for Frame {
        fn update(&mut self, event: &Event) -> ControlFlow {
            self.events.push(event.clone());
            ControlFlow::Continue
        }

        fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
            self.areas.push(area);
            grid.put_str(area.x, area.y, Style::new(), "top", clusters);
            grid.put_str(
                area.x,
                area.y + area.height - 1,
                Style::new(),
                "[Q] Quit",
                clusters,
            );
            let last = area.x + area.width - 1;
            grid.put_str(last, area.y + 1, Style::new(), "|", clusters);
        }
    }

    fn fresh_frames(width: u16, height: u16) -> FrameState {
        FrameState {
            prev: Grid::new(width, height),
            next: Grid::new(width, height),
            clusters: ClusterTable::new(),
            encoder: Encoder::new(),
            last_palette: None,
            repaint: false,
            pointer: None,
            #[cfg(all(feature = "image-presenter", unix))]
            native_images: None,
            #[cfg(feature = "image-presenter")]
            images: super::PixelLayer::new(super::PixelMode::Cells),
            #[cfg(feature = "image-presenter")]
            mode: super::RenderMode::Cell,
        }
    }

    fn present(
        app: &mut Frame,
        area: Rect,
        frames: &mut FrameState,
        screen: &mut lunatui_test::Oracle,
    ) -> Result<(), String> {
        render_present(app, area, frames, |parts| {
            for part in parts {
                screen.process(part);
            }
            Ok(())
        })
        .map_err(|error| error.to_string())
    }

    #[test]
    fn a_resize_after_the_first_frame_relays_out_and_repaints_everything() -> Result<(), String> {
        let mut area = Rect::new(0, 0, 20, 4);
        let mut frames = fresh_frames(20, 4);
        let mut app = Frame {
            areas: Vec::new(),
            events: Vec::new(),
        };
        let mut screen = lunatui_test::Oracle::new(10, 40);
        present(&mut app, area, &mut frames, &mut screen)?;
        assert_eq!(app.areas, vec![Rect::new(0, 0, 20, 4)]);

        let quit = super::dispatch(
            &mut app,
            vec![Event::Resize(40, 10)],
            &mut area,
            &mut frames,
        );
        assert!(!quit);
        assert_eq!(area, Rect::new(0, 0, 40, 10));
        assert_eq!(app.events, vec![Event::Resize(40, 10)]);
        present(&mut app, area, &mut frames, &mut screen)?;
        assert_eq!(app.areas[1], Rect::new(0, 0, 40, 10));
        let rows: Vec<String> = screen
            .contents()
            .lines()
            .map(|row| row.trim_end().to_string())
            .collect();
        assert_eq!(rows[0], "top");
        assert_eq!(rows[1], format!("{}|", " ".repeat(39)));
        assert_eq!(rows[9], "[Q] Quit");
        assert_eq!(
            screen.mismatches(&frames.prev, &frames.clusters, true),
            Vec::<String>::new()
        );

        let mut presents = 0;
        render_present(&mut app, area, &mut frames, |_| {
            presents += 1;
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        assert_eq!(
            presents, 0,
            "an unchanged frame after the resize is skipped"
        );
        Ok(())
    }

    #[test]
    fn shrinking_repaints_the_whole_smaller_grid() -> Result<(), String> {
        let mut area = Rect::new(0, 0, 40, 10);
        let mut frames = fresh_frames(40, 10);
        let mut app = Frame {
            areas: Vec::new(),
            events: Vec::new(),
        };
        let mut screen = lunatui_test::Oracle::new(10, 40);
        present(&mut app, area, &mut frames, &mut screen)?;
        super::dispatch(&mut app, vec![Event::Resize(12, 3)], &mut area, &mut frames);
        let mut fresh = lunatui_test::Oracle::new(3, 12);
        present(&mut app, area, &mut frames, &mut fresh)?;
        assert_eq!(app.areas[1], Rect::new(0, 0, 12, 3));
        assert_eq!(fresh.contents(), "top         \n           |\n[Q] Quit    ");
        assert_eq!(
            fresh.mismatches(&frames.prev, &frames.clusters, true),
            Vec::<String>::new()
        );
        Ok(())
    }

    #[test]
    fn pixel_reports_reach_the_app_in_cells_with_the_offset_inside_the_cell() -> Result<(), String>
    {
        use lunatui_term::{CellPixels, MouseButton, MouseEvent, MouseKind, MouseUnit};

        let report =
            MouseEvent::new(MouseKind::Down(MouseButton::Left), 35, 50).unit(MouseUnit::Pixels);
        let mut area = Rect::new(0, 0, 20, 4);
        let mut frames = fresh_frames(20, 4);
        frames.pointer = CellPixels::new(8, 16);
        let mut app = Frame {
            areas: Vec::new(),
            events: Vec::new(),
        };
        super::dispatch(&mut app, vec![Event::Mouse(report)], &mut area, &mut frames);
        let [Event::Mouse(seen)] = app.events.as_slice() else {
            return Err(format!("one mouse event, got {:?}", app.events));
        };
        assert_eq!((seen.unit, seen.column, seen.row), (MouseUnit::Cells, 4, 3));
        let inside = seen.inside.ok_or("no offset inside the cell")?;
        assert_eq!((inside.x(), inside.y()), (3, 2));

        let mut frames = fresh_frames(20, 4);
        let mut app = Frame {
            areas: Vec::new(),
            events: Vec::new(),
        };
        let cells = MouseEvent::new(MouseKind::Down(MouseButton::Left), 4, 3);
        super::dispatch(&mut app, vec![Event::Mouse(cells)], &mut area, &mut frames);
        assert_eq!(app.events, vec![Event::Mouse(cells)]);
        Ok(())
    }

    #[test]
    fn repeated_zero_and_unchanged_sizes_do_nothing() {
        let mut area = Rect::new(0, 0, 20, 4);
        let mut frames = fresh_frames(20, 4);
        let mut app = Frame {
            areas: Vec::new(),
            events: Vec::new(),
        };
        let events = vec![
            Event::Resize(20, 4),
            Event::Resize(0, 9),
            Event::Resize(9, 0),
        ];
        super::dispatch(&mut app, events, &mut area, &mut frames);
        assert_eq!(area, Rect::new(0, 0, 20, 4));
        assert!(!frames.repaint);
        assert_eq!(app.events.len(), 3);
    }

    #[test]
    fn several_resizes_in_one_frame_end_at_the_last() {
        let mut area = Rect::new(0, 0, 20, 4);
        let mut frames = fresh_frames(20, 4);
        let mut app = Frame {
            areas: Vec::new(),
            events: Vec::new(),
        };
        super::dispatch(
            &mut app,
            vec![Event::Resize(30, 6), Event::Resize(50, 12)],
            &mut area,
            &mut frames,
        );
        assert_eq!(area, Rect::new(0, 0, 50, 12));
        assert_eq!((frames.next.width(), frames.next.height()), (50, 12));
        assert_eq!((frames.prev.width(), frames.prev.height()), (50, 12));
        assert!(frames.repaint);
    }

    struct Quitter;

    impl App for Quitter {
        fn update(&mut self, event: &Event) -> ControlFlow {
            if matches!(event, Event::Resize(..)) {
                ControlFlow::Break
            } else {
                ControlFlow::Continue
            }
        }

        fn render(&mut self, _: &mut Grid, _: Rect, _: &mut ClusterTable) {}
    }

    #[test]
    fn the_app_can_quit_from_a_resize() {
        let mut area = Rect::new(0, 0, 20, 4);
        let mut frames = fresh_frames(20, 4);
        assert!(super::dispatch(
            &mut Quitter,
            vec![Event::Resize(30, 8)],
            &mut area,
            &mut frames
        ));
    }
}

#[cfg(all(test, feature = "image-presenter"))]
#[path = "run/pixel_tests.rs"]
mod pixel_tests;
