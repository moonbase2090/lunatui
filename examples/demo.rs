// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_runtime::{
    App, ControlFlow, RunOptions, SchedulePoll, Scheduler, is_quit, render_once, run,
};
use lunatui_term::{ColorMode, Encoder, Event, KeyCode};

const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 24;
const BENCH_FRAMES: u32 = 4;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let counter = args.iter().any(|arg| arg == "--counter");
    let snapshot = args.iter().any(|arg| arg == "--snapshot");
    let flag_args: Vec<String> = args
        .iter()
        .filter(|arg| *arg != "--counter" && *arg != "--snapshot")
        .cloned()
        .collect();
    let geometry = parse_geometry(&flag_args)?;
    if snapshot {
        print_snapshot(counter, geometry);
        return Ok(());
    }
    if io::stdout().is_terminal() {
        let options = RunOptions::new(geometry.width, geometry.height, 60);
        if counter {
            run(Counter::default(), options)?;
        } else {
            run(Hello::default(), options)?;
        }
        return Ok(());
    }
    let times = run_bench(counter, geometry)?;
    let label = if counter {
        "lunatui demo counter (App + run)"
    } else {
        "lunatui demo hello (App + run, bench uses counter render path)"
    };
    println!(
        "{}",
        bench_summary(geometry.width, geometry.height, times, build_profile(), label)
    );
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct Hello;

impl App for Hello {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }

    fn render(&self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let style = Style::new().fg(Color::Rgb(0x7a, 0xa2, 0xf7));
        grid.put_str(area.x + 2, area.y + 2, style, "Hello, lunatui", clusters);
        grid.put_str(
            area.x + 2,
            area.y + 4,
            Style::new(),
            "PR 23: App + run() — press q to quit",
            clusters,
        );
    }
}

#[derive(Clone, Debug, Default)]
struct Counter {
    value: i32,
}

impl App for Counter {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('+') | KeyCode::Char('=') => self.value += 1,
                KeyCode::Char('-') => self.value -= 1,
                _ => {}
            }
        }
        ControlFlow::Continue
    }

    fn render(&self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let title = Style::new().fg(Color::Rgb(0x9e, 0xce, 0x6a));
        grid.put_str(area.x + 2, area.y + 2, title, "Counter", clusters);
        let line = format!("value: {value}  (+ / -, q quit)", value = self.value);
        grid.put_str(area.x + 2, area.y + 4, Style::new(), &line, clusters);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Geometry {
    width: u16,
    height: u16,
}

fn parse_geometry(args: &[String]) -> io::Result<Geometry> {
    let mut width = None;
    let mut height = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--width" => {
                index += 1;
                width = Some(parse_u16(args.get(index), "--width")?);
            }
            "--height" => {
                index += 1;
                height = Some(parse_u16(args.get(index), "--height")?);
            }
            flag => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag: {flag}"),
                ));
            }
        }
        index += 1;
    }
    let (width, height) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (None, None) => terminal_size().unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT)),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pass both --width and --height, or neither",
            ));
        }
    };
    Ok(Geometry { width, height })
}

fn parse_u16(value: Option<&String>, flag: &str) -> io::Result<u16> {
    let text = value.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
    })?;
    text.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

fn terminal_size() -> Option<(u16, u16)> {
    #[cfg(unix)]
    {
        let (cols, rows) = unsafe {
            let mut size: libc::winsize = core::mem::zeroed();
            if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) != 0 {
                return None;
            }
            (size.ws_col, size.ws_row)
        };
        Some((cols, rows))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn print_snapshot(counter: bool, geometry: Geometry) {
    let (grid, _) = if counter {
        let app = Counter { value: 3 };
        render_once(&app, geometry.width, geometry.height)
    } else {
        render_once(&Hello, geometry.width, geometry.height)
    };
    for y in 0..geometry.height {
        let mut line = String::new();
        for x in 0..geometry.width {
            let ch = match grid.get(x, y) {
                Some(cell) => char::from_u32(cell.glyph().bits()).unwrap_or(' '),
                None => ' ',
            };
            line.push(ch);
        }
        println!("{line}");
    }
}

enum DemoApp {
    Hello(Hello),
    Counter(Counter),
}

impl App for DemoApp {
    fn update(&mut self, event: &Event) -> ControlFlow {
        match self {
            Self::Hello(app) => app.update(event),
            Self::Counter(app) => app.update(event),
        }
    }

    fn render(&self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        match self {
            Self::Hello(app) => app.render(grid, area, clusters),
            Self::Counter(app) => app.render(grid, area, clusters),
        }
    }
}

fn run_bench(counter: bool, geometry: Geometry) -> io::Result<FrameTimes> {
    let mut app = DemoApp::Counter(Counter { value: 0 });
    let mut prev = Grid::new(geometry.width, geometry.height);
    let mut next = Grid::new(geometry.width, geometry.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let area = Rect::new(0, 0, geometry.width, geometry.height);
    let mut samples = Vec::new();
    let epoch = Instant::now();
    let mut sched = Scheduler::idle();
    sched.on_input();
    let mut frame = 0_u32;
    while frame < BENCH_FRAMES {
        let now = epoch.elapsed();
        match sched.poll(now, false) {
            SchedulePoll::WaitInput => break,
            SchedulePoll::SleepUntil(deadline) => thread::sleep(deadline.saturating_sub(now)),
            SchedulePoll::Render => {
                let start = Instant::now();
                if let DemoApp::Counter(counter) = &mut app {
                    counter.value = i32::try_from(frame).unwrap_or(0);
                }
                next.fill(area, Style::new().bg(Color::Default), ' ', &mut clusters);
                app.render(&mut next, area, &mut clusters);
                let runs = diff_runs(&prev, &next);
                let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
                if payload.is_empty() && frame > 0 {
                    return Err(io::Error::other("encode_diff returned an empty payload"));
                }
                if runs.is_empty() && frame > 0 {
                    return Err(io::Error::other("expected diff runs after the first frame"));
                }
                samples.push(start.elapsed());
                prev = next;
                next = Grid::new(geometry.width, geometry.height);
                sched.after_present(epoch.elapsed(), Duration::ZERO);
                frame += 1;
                sched.on_input();
            }
        }
    }
    FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no samples"))
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    }
}

fn bench_summary(
    width: u16,
    height: u16,
    times: FrameTimes,
    profile: &str,
    label: &str,
) -> String {
    let verdict = match times.verdict() {
        Verdict::WithinTarget => "within",
        Verdict::OverTarget | Verdict::OverCeiling => "over",
    };
    format!(
        "{label}, {width}x{height}, {} frames, {profile}: p50 {}, p99 {}, max {} ({verdict} the {} target)",
        times.samples,
        millis(times.p50),
        millis(times.p99),
        millis(times.max),
        millis(FRAME_TARGET),
    )
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use lunatui_core::{Cell, GlyphSlot};
    use lunatui_runtime::App;
    use lunatui_term::{Event, KeyCode, KeyEvent};

    use super::{Counter, Hello, parse_geometry};

    #[test]
    fn hello_renders_title_glyph() {
        let (grid, _) = lunatui_runtime::render_once(&Hello, 40, 10);
        assert_eq!(
            grid.get(2, 2).map(Cell::glyph),
            GlyphSlot::inline_scalar('H')
        );
    }

    #[test]
    fn counter_increments_on_plus() {
        let mut counter = Counter::default();
        let plus = Event::Key(KeyEvent::press(KeyCode::Char('+')));
        assert_eq!(counter.update(&plus), lunatui_runtime::ControlFlow::Continue);
        assert_eq!(counter.value, 1);
    }

    #[test]
    fn parse_geometry_accepts_explicit_size() {
        let args = [
            "--width".to_string(),
            "80".to_string(),
            "--height".to_string(),
            "24".to_string(),
        ];
        let geometry = parse_geometry(&args).expect("geometry");
        assert_eq!(geometry.width, 80);
        assert_eq!(geometry.height, 24);
    }
}
