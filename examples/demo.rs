// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_runtime::{
    App, ControlFlow, FocusDirection, FocusManager, HitId, HitMap, RunOptions, SchedulePoll,
    Scheduler, is_quit, render_once, run,
};
use lunatui_term::{ColorMode, Encoder, Event, KeyCode, KeyEvent, MouseButton, MouseKind};

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
        let options = RunOptions::new(geometry.width, geometry.height);
        if counter {
            run(Counter::default(), options)?;
        } else {
            run(Hello, options)?;
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
        bench_summary(
            geometry.width,
            geometry.height,
            times,
            build_profile(),
            label
        )
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

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
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

const HIT_DEC: HitId = HitId(1);
const HIT_INC: HitId = HitId(2);

#[derive(Clone, Debug)]
struct Counter {
    value: i32,
    hits: HitMap,
    focus: FocusManager,
    hover: Option<HitId>,
    drag: Option<HitId>,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            value: 0,
            hits: HitMap::new(),
            focus: FocusManager::new(),
            hover: None,
            drag: None,
        }
    }
}

impl Counter {
    fn activate(&mut self, id: HitId) {
        if id == HIT_DEC {
            self.value -= 1;
        } else if id == HIT_INC {
            self.value += 1;
        }
    }
}

impl App for Counter {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        match event {
            Event::Key(key) => self.on_key(key),
            Event::Mouse(mouse) => self.on_mouse(mouse),
            _ => {}
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let title = Style::new().fg(Color::Rgb(0x9e, 0xce, 0x6a));
        grid.put_str(area.x + 2, area.y + 2, title, "Counter", clusters);
        let line = format!("value: {value}", value = self.value);
        grid.put_str(area.x + 2, area.y + 4, Style::new(), &line, clusters);
        grid.put_str(
            area.x + 2,
            area.y + 6,
            Style::new(),
            "Tab/click buttons; q quit",
            clusters,
        );

        let dec = Rect::new(area.x + 2, area.y + 8, 5, 1);
        let inc = Rect::new(area.x + 9, area.y + 8, 5, 1);
        self.hits.clear();
        self.hits.push(HIT_DEC, dec, 0);
        self.hits.push(HIT_INC, inc, 1);
        self.focus.sync(vec![(HIT_DEC, dec), (HIT_INC, inc)]);

        let focused = self.focus.focused();
        draw_button(
            grid,
            dec,
            "[ - ]",
            focused == Some(HIT_DEC),
            self.hover == Some(HIT_DEC),
            clusters,
        );
        draw_button(
            grid,
            inc,
            "[ + ]",
            focused == Some(HIT_INC),
            self.hover == Some(HIT_INC),
            clusters,
        );
    }
}

impl Counter {
    fn on_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Tab => self.focus.tab(key.modifiers.is_shift()),
            KeyCode::Up => self.focus.arrow(FocusDirection::Up),
            KeyCode::Down => self.focus.arrow(FocusDirection::Down),
            KeyCode::Left => self.focus.arrow(FocusDirection::Left),
            KeyCode::Right => self.focus.arrow(FocusDirection::Right),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(id) = self.focus.focused() {
                    self.activate(id);
                }
            }
            KeyCode::Char('+' | '=') => self.value += 1,
            KeyCode::Char('-') => self.value -= 1,
            _ => {}
        }
    }

    fn on_mouse(&mut self, mouse: &lunatui_term::MouseEvent) {
        if mouse.unit != lunatui_term::MouseUnit::Cells {
            return;
        }
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                if let Some(id) = self.hits.pick(mouse.column, mouse.row) {
                    self.drag = Some(id);
                    self.focus.focus(id);
                    self.activate(id);
                }
            }
            MouseKind::Up(MouseButton::Left) => {
                self.drag = None;
            }
            MouseKind::Drag(MouseButton::Left) | MouseKind::Move => {
                self.hover = self.hits.pick(mouse.column, mouse.row);
            }
            _ => {}
        }
    }
}

fn draw_button(
    grid: &mut Grid,
    area: Rect,
    label: &str,
    focused: bool,
    hovered: bool,
    clusters: &mut ClusterTable,
) {
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let style = if focused {
        Style::new().fg(Color::Rgb(0x1a, 0x1b, 0x26)).bg(accent)
    } else if hovered {
        Style::new().fg(accent)
    } else {
        Style::new()
    };
    grid.put_str(area.x, area.y, style, label, clusters);
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
        use std::os::fd::AsRawFd;

        let stdout = io::stdout();
        if !stdout.is_terminal() {
            return None;
        }
        let mut winsize = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ok =
            // SAFETY: TIOCGWINSZ reads the tty geometry for stdout when it is a terminal.
            unsafe { libc::ioctl(stdout.as_raw_fd(), libc::TIOCGWINSZ, &raw mut winsize) == 0 };
        if !ok || winsize.ws_col == 0 || winsize.ws_row == 0 {
            return None;
        }
        Some((winsize.ws_col, winsize.ws_row))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn print_snapshot(counter: bool, geometry: Geometry) {
    let (grid, _) = if counter {
        let mut app = Counter {
            value: 3,
            ..Counter::default()
        };
        render_once(&mut app, geometry.width, geometry.height)
    } else {
        let mut hello = Hello;
        render_once(&mut hello, geometry.width, geometry.height)
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

fn run_bench(_counter: bool, geometry: Geometry) -> io::Result<FrameTimes> {
    let mut app = Counter {
        value: 0,
        ..Counter::default()
    };
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
                app.value = i32::try_from(frame).unwrap_or(0);
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

fn bench_summary(width: u16, height: u16, times: FrameTimes, profile: &str, label: &str) -> String {
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
        let mut hello = Hello;
        let (grid, _) = lunatui_runtime::render_once(&mut hello, 40, 10);
        assert_eq!(
            grid.get(2, 2).map(Cell::glyph),
            GlyphSlot::inline_scalar('H')
        );
    }

    #[test]
    fn counter_increments_on_plus() {
        let mut counter = Counter::default();
        let plus = Event::Key(KeyEvent::press(KeyCode::Char('+')));
        assert_eq!(
            counter.update(&plus),
            lunatui_runtime::ControlFlow::Continue
        );
        assert_eq!(counter.value, 1);
    }

    #[test]
    fn parse_geometry_accepts_explicit_size() -> Result<(), Box<dyn std::error::Error>> {
        let args = [
            "--width".to_string(),
            "80".to_string(),
            "--height".to_string(),
            "24".to_string(),
        ];
        let geometry = parse_geometry(&args)?;
        assert_eq!(geometry.width, 80);
        assert_eq!(geometry.height, 24);
        Ok(())
    }
}
