// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_core::{
    Attrs, ClusterTable, Color, Grid, Rect, Style, diff_runs, width as display_width,
};
use lunatui_runtime::{SchedulePoll, Scheduler};
use lunatui_term::{ColorMode, Encoder, Session, SessionOptions, SyncOutput};

const FRAMES: u32 = 240;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const LOG_LINES: usize = 200;
const DEFAULT_WIDTH: u16 = 200;
const DEFAULT_HEIGHT: u16 = 60;

/// Dogfood state: animation tick, log scroll, and prebuilt log lines.
#[derive(Clone, Debug)]
struct Model {
    tick: u32,
    scroll: usize,
    log: Vec<String>,
}

impl Model {
    fn new() -> Self {
        let log = (0..LOG_LINES)
            .map(|i| {
                format!(
                    "[{i:03}] worker-{worker}: processed batch {i} ({rows} rows)",
                    worker = i % 8,
                    rows = (i * 17) % 4096
                )
            })
            .collect();
        Self {
            tick: 0,
            scroll: 0,
            log,
        }
    }
}

fn main() -> io::Result<()> {
    let options = parse_args(env::args().skip(1).collect())?;
    let times = if io::stdout().is_terminal() {
        run_live(&options)?
    } else {
        run_bench(&options)?
    };
    let times = times.ok_or_else(|| io::Error::other("no frames were drawn"))?;
    println!(
        "{}",
        summary(
            options.width,
            options.height,
            times,
            build_profile(),
            "lunatui live_dashboard (Scheduler + Grid + diff_runs + Encoder)",
        )
    );
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
}

fn parse_args(args: Vec<String>) -> io::Result<Options> {
    let mut width = None;
    let mut height = None;
    let mut frames = FRAMES;
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
            "--frames" => {
                index += 1;
                frames = parse_u16(args.get(index), "--frames")?.into();
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
    Ok(Options {
        width,
        height,
        frames,
    })
}

fn parse_u16(value: Option<&String>, flag: &str) -> io::Result<u16> {
    let text = value.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
    })?;
    text.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

fn run_bench(options: &Options) -> io::Result<Option<FrameTimes>> {
    let mut model = Model::new();
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut samples = Vec::new();
    let epoch = Instant::now();
    let mut sched = Scheduler::animating(FRAME_INTERVAL);
    let mut tick = 0_u32;
    while tick < options.frames {
        let now = epoch.elapsed();
        match sched.poll(now, false) {
            SchedulePoll::WaitInput => break,
            SchedulePoll::SleepUntil(deadline) => {
                thread::sleep(deadline.saturating_sub(now));
            }
            SchedulePoll::Render => {
                model.tick = tick;
                model.scroll = usize::try_from(tick / 4 % u32::try_from(LOG_LINES).unwrap_or(1))
                    .unwrap_or(0);
                let start = Instant::now();
                render_frame(&model, &mut next, &mut clusters, options.width, options.height);
                let runs = diff_runs(&prev, &next);
                let encode_start = epoch.elapsed();
                let payload = encoder.encode_diff(
                    &prev,
                    &next,
                    &clusters,
                    ColorMode::TrueColor,
                );
                let encode_done = epoch.elapsed();
                let _payload = payload;
                samples.push(start.elapsed());
                prev = next;
                next = Grid::new(options.width, options.height);
                if runs.is_empty() && tick > 0 {
                    return Err(io::Error::other("expected diff runs after the first frame"));
                }
                sched.after_present(encode_start, encode_done.saturating_sub(encode_start));
                tick += 1;
            }
        }
    }
    FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no samples")).map(Some)
}

fn run_live(options: &Options) -> io::Result<Option<FrameTimes>> {
    #[cfg(unix)]
    {
        let mut model = Model::new();
        let mut prev = Grid::new(options.width, options.height);
        let mut next = Grid::new(options.width, options.height);
        let mut clusters = ClusterTable::new();
        let mut encoder = Encoder::new();
        let mut samples = Vec::new();
        let stdout = io::stdout();
        let mut session = Session::enter_with_options(
            stdout,
            SessionOptions {
                sync_output: SyncOutput::Enabled,
            },
        )?;
        let epoch = Instant::now();
        let mut sched = Scheduler::animating(FRAME_INTERVAL);
        let mut tick = 0_u32;
        while tick < options.frames {
            let now = epoch.elapsed();
            match sched.poll(now, false) {
                SchedulePoll::WaitInput => break,
                SchedulePoll::SleepUntil(deadline) => {
                    thread::sleep(deadline.saturating_sub(now));
                }
                SchedulePoll::Render => {
                    model.tick = tick;
                    model.scroll =
                        usize::try_from(tick / 4 % u32::try_from(LOG_LINES).unwrap_or(1))
                            .unwrap_or(0);
                    let start = Instant::now();
                    render_frame(&model, &mut next, &mut clusters, options.width, options.height);
                    let _runs = diff_runs(&prev, &next);
                    let payload = encoder.encode_diff(
                        &prev,
                        &next,
                        &clusters,
                        ColorMode::TrueColor,
                    );
                    let present_start = epoch.elapsed();
                    session.present_frame(payload)?;
                    let present_done = epoch.elapsed();
                    samples.push(start.elapsed());
                    prev = next;
                    next = Grid::new(options.width, options.height);
                    sched.after_present(
                        present_start,
                        present_done.saturating_sub(present_start),
                    );
                    tick += 1;
                }
            }
        }
        return Ok(FrameTimes::from_samples(&samples));
    }
    #[cfg(not(unix))]
    {
        let _ = options;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "live_dashboard live mode requires Unix",
        ))
    }
}

fn render_frame(
    model: &Model,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    width: u16,
    height: u16,
) {
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let bg_style = Style::new().bg(bg);
    grid.fill(
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        },
        bg_style,
        ' ',
        clusters,
    );

    let bold = Attrs::default().set_bold(true);
    let header_style = Style::new().fg(accent).bg(bg).with_attrs(bold);
    let clock = format_clock(model.tick);
    grid.put_str(1, 0, header_style, "lunatui live dashboard", clusters);
    let clock_x = width.saturating_sub(u16::try_from(display_width(&clock) + 2).unwrap_or(2));
    grid.put_str(clock_x, 0, header_style, &clock, clusters);

    let counter = format!(
        "tick {:>5}  scroll {:>4}  events {:>8}",
        model.tick,
        model.scroll,
        model.tick * 37 + u32::try_from(model.scroll).unwrap_or(0)
    );
    grid.put_str(1, 1, Style::new().fg(dim).bg(bg), &counter, clusters);

    let log_top = 3_u16;
    let log_bottom = height.saturating_sub(2);
    if log_top < log_bottom {
        grid.put_str(1, 2, Style::new().fg(accent).bg(bg), "scrolling log", clusters);
        let visible = usize::from(log_bottom - log_top);
        for row in 0..visible {
            let line_index = (model.scroll + row) % model.log.len();
            let line = &model.log[line_index];
            let y = log_top + u16::try_from(row).unwrap_or(0);
            grid.put_str(1, y, Style::new().fg(Color::Default).bg(bg), line, clusters);
        }
    }

    let spinner = SPINNER[usize::try_from(model.tick % 10).unwrap_or(0)];
    let status = format!("{spinner}  dogfood · compare to ratatui_dashboard baseline");
    grid.put_str(1, height - 1, Style::new().fg(accent).bg(bg), &status, clusters);
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn format_clock(tick: u32) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let seconds = millis / 1000 + u128::from(tick);
    let hours = (seconds / 3600) % 24;
    let minutes = (seconds / 60) % 60;
    let secs = seconds % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

#[cfg(unix)]
fn terminal_size() -> Option<(u16, u16)> {
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
    // SAFETY: `TIOCGWINSZ` reads the tty geometry for stdout when it is a terminal.
    let ok = unsafe {
        libc::ioctl(stdout.as_raw_fd(), libc::TIOCGWINSZ, &raw mut winsize) == 0
    };
    if !ok || winsize.ws_col == 0 || winsize.ws_row == 0 {
        return None;
    }
    Some((winsize.ws_col, winsize.ws_row))
}

#[cfg(not(unix))]
fn terminal_size() -> Option<(u16, u16)> {
    None
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    }
}

fn summary(width: u16, height: u16, times: FrameTimes, profile: &str, label: &str) -> String {
    let verdict = match times.verdict() {
        Verdict::WithinTarget => "within",
        Verdict::OverTarget | Verdict::OverCeiling => "over",
    };
    format!(
        "{label}, {width}x{height}, {} frames, {profile}: p50 {}, p99 {}, max {} ({verdict} the {} target); ratatui baseline: `cargo run --release --example ratatui_dashboard`",
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
    use std::time::Duration;

    use lunatui_bench::ceiling::FrameTimes;
    use lunatui_core::ClusterTable;

    use lunatui_core::Grid;

    use super::{Model, Options, parse_args, render_frame, summary};

    #[test]
    fn summary_reports_size_percentiles_and_target() {
        let times = FrameTimes {
            samples: 240,
            p50: Duration::from_micros(500),
            p99: Duration::from_micros(1_250),
            max: Duration::from_millis(2),
        };
        assert!(summary(200, 60, times, "release build", "lunatui live_dashboard")
            .contains("p50 0.500 ms"));
    }

    #[test]
    fn parse_args_accepts_explicit_geometry() {
        let options = parse_args(vec![
            "--width".into(),
            "80".into(),
            "--height".into(),
            "24".into(),
            "--frames".into(),
            "3".into(),
        ])
        .expect("parse");
        assert_eq!(
            options,
            Options {
                width: 80,
                height: 24,
                frames: 3,
            }
        );
    }

    #[test]
    fn render_frame_fills_the_grid() {
        let model = Model::new();
        let mut grid = Grid::new(40, 10);
        let mut clusters = ClusterTable::new();
        render_frame(&model, &mut grid, &mut clusters, 40, 10);
        assert!(grid.get(1, 0).is_some());
        assert!(grid.get(1, 9).is_some());
    }
}
