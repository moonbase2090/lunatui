// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_bench::scenes::lunatui::dashboard as luna_dash;
use lunatui_bench::scenes::ratatui::dashboard::Model;
use lunatui_core::{ClusterTable, Grid};
use lunatui_term::{ColorMode, Encoder};
#[cfg(unix)]
use lunatui_term::{Session, SessionOptions, SyncOutput};

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
        let (grid, clusters) = luna_dash::render_new(&model, options.width, options.height);
        for y in 0..options.height {
            println!("{}", grid.row_display(y, &clusters));
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
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut samples = Vec::with_capacity(usize::try_from(options.frames).unwrap_or(0));
    let mut output = Vec::new();
    let mut bytes = 0_usize;
    for tick in 0..options.frames {
        advance_model(&mut model, tick);
        let (w, h) = if options.demo {
            demo_size(options.width, options.height, tick)
        } else {
            (options.width, options.height)
        };
        let start = Instant::now();
        if next.width() != w || next.height() != h {
            prev = Grid::new(w, h);
            next = Grid::new(w, h);
            clusters = ClusterTable::new();
            encoder = Encoder::new();
        }
        luna_dash::render(&model, w, h, &mut next, &mut clusters);
        let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
        output.write_all(payload)?;
        bytes = bytes.saturating_add(payload.len());
        samples.push(start.elapsed());
        std::mem::swap(&mut prev, &mut next);
    }
    let times = FrameTimes::from_samples(&samples)
        .ok_or_else(|| io::Error::other("no frames were drawn"))?;
    Ok((times, bytes))
}

fn run_live(options: Options) -> io::Result<FrameTimes> {
    #[cfg(unix)]
    {
        let mut model = Model::new();
        let mut prev = Grid::new(options.width, options.height);
        let mut next = Grid::new(options.width, options.height);
        let mut clusters = ClusterTable::new();
        let mut encoder = Encoder::new();
        let mut samples = Vec::with_capacity(usize::try_from(options.frames).unwrap_or(0));
        let mut session = Session::enter_with_options(
            io::stdout().lock(),
            SessionOptions {
                sync_output: SyncOutput::Enabled,
                mouse_tracking: lunatui_term::MouseTracking::Disabled,
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
            if next.width() != w || next.height() != h {
                prev = Grid::new(w, h);
                next = Grid::new(w, h);
                clusters = ClusterTable::new();
                encoder = Encoder::new();
            }
            luna_dash::render(&model, w, h, &mut next, &mut clusters);
            let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
            session.present_frame(payload)?;
            samples.push(start.elapsed());
            std::mem::swap(&mut prev, &mut next);
            thread::sleep(FRAME_INTERVAL.saturating_sub(start.elapsed()));
        }
        FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no frames were drawn"))
    }
    #[cfg(not(unix))]
    {
        let _ = options;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lunatui_dashboard live mode requires Unix",
        ))
    }
}

fn live_summary(options: Options, times: FrameTimes) -> String {
    let mode = if options.demo { "demo" } else { "live" };
    format!(
        "lunatui_dashboard {mode}, {}x{}, {} frames, {}: p50 {}, p99 {}, max {} ({})",
        options.width,
        options.height,
        times.samples,
        build_profile(),
        millis(times.p50),
        millis(times.p99),
        millis(times.max),
        target_phrase(times),
    )
}

fn bench_summary(options: Options, times: FrameTimes, bytes: usize) -> String {
    let per_frame = bytes.checked_div(times.samples).unwrap_or(0);
    format!(
        "lunatui_dashboard bench, {}x{}, {} frames, {}: p50 {}, p99 {}, max {}, bytes_total={}, bytes_per_frame≈{} ({})",
        options.width,
        options.height,
        times.samples,
        build_profile(),
        millis(times.p50),
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
