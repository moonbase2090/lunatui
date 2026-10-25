// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

struct ByteSink {
    total: Arc<AtomicUsize>,
    buf: Vec<u8>,
}

impl Write for ByteSink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.total.fetch_add(data.len(), Ordering::Relaxed);
        self.buf.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf.flush()
    }
}

const SHORT_FRAMES: u32 = 240;
const DEMO_FRAMES: u32 = 1_500;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
    snapshot: bool,
    bench: bool,
    demo: bool,
}

fn main() -> io::Result<()> {
    let options = parse_args(&env::args().skip(1).collect::<Vec<_>>())?;
    if options.snapshot {
        let mut model = Model::new();
        advance_model(&mut model, 42);
        let area = Rect::new(0, 0, options.width, options.height);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        dashboard::render(&model, area, &mut buf);
        for y in 0..options.height {
            let mut line = String::new();
            for x in 0..options.width {
                line.push_str(buf[(x, y)].symbol());
            }
            println!("{line}");
        }
        return Ok(());
    }
    if options.bench {
        let (times, bytes) = run_bench(options)?;
        println!("{}", bench_summary(options, times, bytes));
        return Ok(());
    }
    let times = run_live(options)?;
    println!("{}", live_summary(options, times));
    Ok(())
}

fn parse_args(args: &[String]) -> io::Result<Options> {
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut frames = SHORT_FRAMES;
    let mut snapshot = false;
    let mut bench = false;
    let mut demo = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--snapshot" => snapshot = true,
            "--bench" => bench = true,
            "--demo" => {
                demo = true;
                frames = DEMO_FRAMES;
            }
            "--width" => {
                i += 1;
                width = parse_u16(args.get(i), "--width")?;
            }
            "--height" => {
                i += 1;
                height = parse_u16(args.get(i), "--height")?;
            }
            "--frames" => {
                i += 1;
                frames = u32::from(parse_u16(args.get(i), "--frames")?);
            }
            flag => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag: {flag}"),
                ));
            }
        }
        i += 1;
    }
    Ok(Options {
        width: width.max(40),
        height: height.max(12),
        frames,
        snapshot,
        bench,
        demo,
    })
}

fn parse_u16(value: Option<&String>, flag: &str) -> io::Result<u16> {
    let text = value.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
    })?;
    text.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

fn advance_model(model: &mut Model, tick: u32) {
    model.tick = tick;
    model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
}

fn demo_size(base_w: u16, base_h: u16, tick: u32) -> (u16, u16) {
    if (500..1000).contains(&tick) {
        (
            base_w.saturating_add(20).max(60),
            base_h.saturating_add(6).max(18),
        )
    } else {
        (base_w, base_h)
    }
}

fn run_bench(options: Options) -> io::Result<(FrameTimes, usize)> {
    let mut model = Model::new();
    let mut samples = Vec::with_capacity(usize::try_from(options.frames).unwrap_or(0));
    let mut cur_w = options.width;
    let mut cur_h = options.height;
    let total = Arc::new(AtomicUsize::new(0));
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(ByteSink {
            total: Arc::clone(&total),
            buf: Vec::new(),
        }),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, cur_w, cur_h)),
        },
    )?;
    for tick in 0..options.frames {
        advance_model(&mut model, tick);
        let (w, h) = if options.demo {
            demo_size(options.width, options.height, tick)
        } else {
            (options.width, options.height)
        };
        let start = Instant::now();
        if w != cur_w || h != cur_h {
            cur_w = w;
            cur_h = h;
            terminal = Terminal::with_options(
                CrosstermBackend::new(ByteSink {
                    total: Arc::clone(&total),
                    buf: Vec::new(),
                }),
                TerminalOptions {
                    viewport: Viewport::Fixed(Rect::new(0, 0, cur_w, cur_h)),
                },
            )?;
        }
        terminal.draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))?;
        samples.push(start.elapsed());
    }
    let bytes = total.load(Ordering::Relaxed);
    let times = FrameTimes::from_samples(&samples)
        .ok_or_else(|| io::Error::other("no frames were drawn"))?;
    Ok((times, bytes))
}

fn run_live(options: Options) -> io::Result<FrameTimes> {
    let mut terminal = ratatui::try_init()?;
    let mut model = Model::new();
    let mut samples = Vec::with_capacity(usize::try_from(options.frames).unwrap_or(0));
    for tick in 0..options.frames {
        advance_model(&mut model, tick);
        let start = Instant::now();
        if options.demo {
            let (w, h) = demo_size(options.width, options.height, tick);
            let _ = write!(io::stdout(), "\x1b[8;{h};{w}t");
            let _ = io::stdout().flush();
        }
        terminal.draw(|frame| {
            let area = if options.demo {
                let (w, h) = demo_size(options.width, options.height, tick);
                Rect::new(0, 0, w.min(frame.area().width), h.min(frame.area().height))
            } else {
                frame.area()
            };
            dashboard::render(&model, area, frame.buffer_mut());
        })?;
        let elapsed = start.elapsed();
        samples.push(elapsed);
        thread::sleep(FRAME_INTERVAL.saturating_sub(elapsed));
    }
    ratatui::try_restore()?;
    FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no frames were drawn"))
}

fn live_summary(options: Options, times: FrameTimes) -> String {
    let mode = if options.demo { "demo" } else { "live" };
    format!(
        "ratatui 0.30.2 dashboard {mode}, {}x{}, {} frames, {}: p50 {}, p95 {}, p99 {}, max {} ({})",
        options.width,
        options.height,
        times.samples,
        build_profile(),
        millis(times.p50),
        millis(times.p95),
        millis(times.p99),
        millis(times.max),
        target_phrase(times),
    )
}

fn bench_summary(options: Options, times: FrameTimes, bytes: usize) -> String {
    let per_frame = bytes.checked_div(times.samples).unwrap_or(0);
    format!(
        "ratatui 0.30.2 dashboard bench, {}x{}, {} frames, {}: p50 {}, p95 {}, p99 {}, max {}, bytes_total={}, bytes_per_frame≈{} ({})",
        options.width,
        options.height,
        times.samples,
        build_profile(),
        millis(times.p50),
        millis(times.p95),
        millis(times.p99),
        millis(times.max),
        bytes,
        per_frame,
        target_phrase(times),
    )
}

fn target_phrase(times: FrameTimes) -> String {
    let verdict = match times.verdict() {
        Verdict::WithinTarget => "within",
        Verdict::OverTarget | Verdict::OverCeiling => "over",
    };
    format!("{verdict} the {} target", millis(FRAME_TARGET))
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    }
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lunatui_bench::ceiling::FrameTimes;

    use super::{Options, live_summary};

    #[test]
    fn summary_reports_size_percentiles_and_target() {
        let times = FrameTimes {
            samples: 240,
            p50: Duration::from_micros(500),
            p95: Duration::from_micros(1_000),
            p99: Duration::from_micros(1_250),
            max: Duration::from_millis(2),
        };
        let options = Options {
            width: 200,
            height: 60,
            frames: 240,
            snapshot: false,
            bench: false,
            demo: false,
        };
        assert_eq!(
            live_summary(options, times),
            "ratatui 0.30.2 dashboard live, 200x60, 240 frames, debug build: p50 0.500 ms, p95 1.000 ms, p99 1.250 ms, max 2.000 ms (within the 8.000 ms target)"
        );
    }

    #[test]
    fn summary_flags_a_missed_target() {
        let times = FrameTimes {
            samples: 1,
            p50: Duration::from_millis(9),
            p95: Duration::from_millis(9),
            p99: Duration::from_millis(9),
            max: Duration::from_millis(9),
        };
        let options = Options {
            width: 400,
            height: 120,
            frames: 1,
            snapshot: false,
            bench: false,
            demo: false,
        };
        assert!(live_summary(options, times).ends_with("(over the 8.000 ms target)"));
    }
}
