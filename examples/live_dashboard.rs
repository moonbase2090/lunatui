// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style, width as display_width};
use lunatui_runtime::{SchedulePoll, Scheduler};
use lunatui_term::{ColorMode, Encoder};
#[cfg(unix)]
use lunatui_term::{Session, SessionOptions, SyncOutput};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer as RatatuiBuffer;
use ratatui::layout::Rect as RatatuiRect;
use ratatui::style::{Color as RatatuiColor, Modifier, Style as RatatuiStyle};
use ratatui::{Terminal, TerminalOptions, Viewport};

const FRAMES: u32 = 240;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const LOG_LINES: usize = 200;
const DEFAULT_WIDTH: u16 = 200;
const DEFAULT_HEIGHT: u16 = 60;

#[derive(Clone, Debug)]
struct Model {
    tick: u32,
    scroll: usize,
    clock: String,
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
            clock: format_clock(0),
            log,
        }
    }

    fn advance(&mut self, tick: u32) {
        self.tick = tick;
        self.scroll =
            usize::try_from(tick / 4 % u32::try_from(LOG_LINES).unwrap_or(1)).unwrap_or(0);
        self.clock = format_clock(tick);
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let snapshot = args.iter().any(|arg| arg == "--snapshot");
    let flag_args: Vec<String> = args
        .iter()
        .filter(|arg| *arg != "--snapshot")
        .cloned()
        .collect();
    let options = parse_args(&flag_args)?;
    if snapshot {
        print_snapshot(&options);
        return Ok(());
    }
    if options.parity_report {
        print_parity_report(&options)?;
        return Ok(());
    }
    let times = if !options.bench && io::stdout().is_terminal() {
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
            "lunatui live_dashboard (Grid + Encoder::encode_diff (diff + encode) + write)",
        )
    );
    let ratatui_times = run_ratatui_bench(&options)?
        .ok_or_else(|| io::Error::other("ratatui baseline drew no frames"))?;
    println!(
        "{}",
        summary(
            options.width,
            options.height,
            ratatui_times,
            build_profile(),
            "ratatui 0.30.2 live_dashboard (ratatui_dashboard baseline; Buffer + diff + encode + write)",
        )
    );
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
    bench: bool,
    parity_report: bool,
}

fn parse_args(args: &[String]) -> io::Result<Options> {
    let mut width = None;
    let mut height = None;
    let mut frames = FRAMES;
    let mut bench = false;
    let mut parity_report = false;
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
            "--bench" => bench = true,
            "--parity-report" => parity_report = true,
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
        bench,
        parity_report,
    })
}

static ALLOC: CountingAllocator = CountingAllocator::system();

struct ParityRow {
    runtime: &'static str,
    times: FrameTimes,
    alloc: AllocStats,
    write: WriteStats,
    bytes_per_frame: u64,
}

fn print_parity_report(options: &Options) -> io::Result<()> {
    let lunatui = measure_lunatui_parity(options)?;
    let ratatui = measure_ratatui_parity(options)?;
    let rss_kb = max_rss_kb();
    println!(
        "dashboard parity {}x{}, {} scripted frames, {}",
        options.width,
        options.height,
        options.frames,
        build_profile()
    );
    if let Some(rss) = rss_kb {
        println!("process max RSS (approx): {rss} KiB");
    }
    for row in [lunatui, ratatui] {
        println!(
            "{}\tframe_ms_p50={}\tframe_ms_p95={}\tframe_ms_p99={}\tframe_ms_max={}\tallocs_per_frame={:.1}\tbytes_per_frame={}\twrite_calls_per_frame={:.1}",
            row.runtime,
            millis(row.times.p50),
            millis(row.times.p95),
            millis(row.times.p99),
            millis(row.times.max),
            f64::from(u32::try_from(row.alloc.allocations).unwrap_or(u32::MAX))
                / f64::from(options.frames),
            row.bytes_per_frame,
            f64::from(u32::try_from(row.write.writes).unwrap_or(u32::MAX))
                / f64::from(options.frames),
        );
    }
    Ok(())
}

fn measure_lunatui_parity(options: &Options) -> io::Result<ParityRow> {
    let mut model = Model::new();
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut samples = Vec::new();
    let (inner, alloc) = ALLOC.measure(|| -> io::Result<((), WriteStats)> {
        let mut output = CountingWriter::new(io::sink());
        for tick in 0..options.frames {
            model.advance(tick);
            let start = Instant::now();
            render_frame(
                &model,
                &mut next,
                &mut clusters,
                options.width,
                options.height,
            );
            let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
            output.write_all(payload)?;
            samples.push(start.elapsed());
            prev = next;
            next = Grid::new(options.width, options.height);
        }
        Ok(((), output.stats()))
    });
    let ((), write) = inner?;
    let times = FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no samples"))?;
    Ok(ParityRow {
        runtime: "lunatui",
        times,
        alloc,
        write,
        bytes_per_frame: u64::try_from(write.bytes).unwrap_or(0) / u64::from(options.frames.max(1)),
    })
}

fn measure_ratatui_parity(options: &Options) -> io::Result<ParityRow> {
    let area = RatatuiRect::new(0, 0, options.width, options.height);
    let mut samples = Vec::new();
    let mut out = CountingWriter::new(io::sink());
    let (inner, alloc) = ALLOC.measure(|| -> io::Result<()> {
        let backend = CrosstermBackend::new(&mut out);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(area),
            },
        )?;
        let mut model = Model::new();
        for tick in 0..options.frames {
            model.advance(tick);
            let start = Instant::now();
            terminal
                .draw(|frame| render_ratatui_frame(&model, frame.area(), frame.buffer_mut()))?;
            samples.push(start.elapsed());
        }
        Ok(())
    });
    inner?;
    let write = out.stats();
    let times =
        FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no ratatui samples"))?;
    Ok(ParityRow {
        runtime: "ratatui_0.30.2",
        times,
        alloc,
        write,
        bytes_per_frame: u64::try_from(write.bytes).unwrap_or(0) / u64::from(options.frames.max(1)),
    })
}

#[cfg(unix)]
fn max_rss_kb() -> Option<u64> {
    use std::mem::MaybeUninit;

    let mut usage = MaybeUninit::uninit();
    let usage = {
        // SAFETY: `getrusage` fully initializes `usage` when it returns zero.
        unsafe {
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
                return None;
            }
            usage.assume_init()
        }
    };
    #[cfg(target_os = "macos")]
    {
        Some(u64::try_from(usage.ru_maxrss).unwrap_or(0) / 1024)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(u64::try_from(usage.ru_maxrss).unwrap_or(0))
    }
}

#[cfg(not(unix))]
fn max_rss_kb() -> Option<u64> {
    None
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
    let mut output = Vec::new();
    for tick in 0..options.frames {
        model.advance(tick);
        let start = Instant::now();
        render_frame(
            &model,
            &mut next,
            &mut clusters,
            options.width,
            options.height,
        );
        let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
        output.write_all(payload)?;
        samples.push(start.elapsed());
        prev = next;
        next = Grid::new(options.width, options.height);
    }
    FrameTimes::from_samples(&samples)
        .ok_or_else(|| io::Error::other("no samples"))
        .map(Some)
}

fn run_ratatui_bench(options: &Options) -> io::Result<Option<FrameTimes>> {
    let area = RatatuiRect::new(0, 0, options.width, options.height);
    let backend = CrosstermBackend::new(Vec::new());
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )?;
    let mut model = Model::new();
    let mut samples = Vec::new();
    for tick in 0..options.frames {
        model.advance(tick);
        let start = Instant::now();
        terminal.draw(|frame| render_ratatui_frame(&model, frame.area(), frame.buffer_mut()))?;
        samples.push(start.elapsed());
    }
    FrameTimes::from_samples(&samples)
        .ok_or_else(|| io::Error::other("no ratatui baseline samples"))
        .map(Some)
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
                mouse_tracking: lunatui_term::MouseTracking::Disabled,
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
                    model.advance(tick);
                    let start = Instant::now();
                    render_frame(
                        &model,
                        &mut next,
                        &mut clusters,
                        options.width,
                        options.height,
                    );
                    let payload =
                        encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
                    let present_start = epoch.elapsed();
                    session.present_frame(payload)?;
                    let present_done = epoch.elapsed();
                    samples.push(start.elapsed());
                    prev = next;
                    next = Grid::new(options.width, options.height);
                    sched.after_present(present_start, present_done.saturating_sub(present_start));
                    tick += 1;
                }
            }
        }
        Ok(FrameTimes::from_samples(&samples))
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
    grid.put_str(1, 0, header_style, "lunatui live dashboard", clusters);
    let clock_x = width.saturating_sub(u16::try_from(display_width(&model.clock) + 2).unwrap_or(2));
    grid.put_str(clock_x, 0, header_style, &model.clock, clusters);

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
        grid.put_str(
            1,
            2,
            Style::new().fg(accent).bg(bg),
            "scrolling log",
            clusters,
        );
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
    grid.put_str(
        1,
        height - 1,
        Style::new().fg(accent).bg(bg),
        &status,
        clusters,
    );
}

fn render_ratatui_frame(model: &Model, area: RatatuiRect, buffer: &mut RatatuiBuffer) {
    let bg = RatatuiColor::Rgb(0x1a, 0x1b, 0x26);
    let accent = RatatuiColor::Rgb(0x7a, 0xa2, 0xf7);
    let dim = RatatuiColor::Rgb(0x56, 0x5f, 0x89);
    buffer.set_style(area, RatatuiStyle::default().bg(bg));
    let header_style = RatatuiStyle::default()
        .fg(accent)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    buffer.set_string(1, 0, "lunatui live dashboard", header_style);
    let clock_x = area
        .width
        .saturating_sub(u16::try_from(display_width(&model.clock) + 2).unwrap_or(2));
    buffer.set_string(clock_x, 0, &model.clock, header_style);

    let counter = format!(
        "tick {:>5}  scroll {:>4}  events {:>8}",
        model.tick,
        model.scroll,
        model.tick * 37 + u32::try_from(model.scroll).unwrap_or(0)
    );
    buffer.set_string(1, 1, counter, RatatuiStyle::default().fg(dim).bg(bg));

    let log_top = 3_u16;
    let log_bottom = area.height.saturating_sub(2);
    if log_top < log_bottom {
        buffer.set_string(
            1,
            2,
            "scrolling log",
            RatatuiStyle::default().fg(accent).bg(bg),
        );
        let visible = usize::from(log_bottom - log_top);
        let blank = " ".repeat(usize::from(area.width.saturating_sub(1)));
        let log_style = RatatuiStyle::default().fg(RatatuiColor::Reset).bg(bg);
        for row in 0..visible {
            let line_index = (model.scroll + row) % model.log.len();
            let line = &model.log[line_index];
            let y = log_top + u16::try_from(row).unwrap_or(0);
            buffer.set_string(1, y, &blank, log_style);
            buffer.set_string(1, y, line, log_style);
        }
    }

    let spinner = SPINNER[usize::try_from(model.tick % 10).unwrap_or(0)];
    let status = format!("{spinner}  dogfood · compare to ratatui_dashboard baseline");
    buffer.set_string(
        1,
        area.height - 1,
        status,
        RatatuiStyle::default().fg(accent).bg(bg),
    );
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn format_clock(tick: u32) -> String {
    let seconds = u64::from(tick) / 60;
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
    let ok =
        // SAFETY: TIOCGWINSZ reads the tty geometry for stdout when it is a terminal.
        unsafe { libc::ioctl(stdout.as_raw_fd(), libc::TIOCGWINSZ, &raw mut winsize) == 0 };
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
        "{label}, {width}x{height}, {} frames, {profile}: p50 {}, p95 {}, p99 {}, max {} ({verdict} the {} target)",
        times.samples,
        millis(times.p50),
        millis(times.p95),
        millis(times.p99),
        millis(times.max),
        millis(FRAME_TARGET),
    )
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

fn print_snapshot(options: &Options) {
    let mut model = Model::new();
    model.advance(42);
    model.scroll = 3;
    let mut grid = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    render_frame(
        &model,
        &mut grid,
        &mut clusters,
        options.width,
        options.height,
    );
    for y in 0..options.height {
        let mut line = String::new();
        for x in 0..options.width {
            let ch = match grid.get(x, y) {
                Some(cell) => char::from_u32(cell.glyph().bits()).unwrap_or(' '),
                None => ' ',
            };
            line.push(ch);
        }
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lunatui_bench::ceiling::FrameTimes;
    use lunatui_core::{Cell, ClusterTable, Grid};
    use ratatui::buffer::Buffer as RatatuiBuffer;
    use ratatui::layout::Rect as RatatuiRect;

    use super::{Model, Options, parse_args, render_frame, render_ratatui_frame, summary};

    #[test]
    fn summary_reports_size_percentiles_and_target() {
        let times = FrameTimes {
            samples: 240,
            p50: Duration::from_micros(500),
            p95: Duration::from_micros(1_000),
            p99: Duration::from_micros(1_250),
            max: Duration::from_millis(2),
        };
        let text = summary(200, 60, times, "release build", "lunatui live_dashboard");
        assert!(text.contains("p50 0.500 ms"));
        assert!(text.contains("p95 1.000 ms"));
        assert!(text.contains("p99 1.250 ms"));
        assert!(text.contains("max 2.000 ms"));
        assert!(text.contains("within the 8.000 ms target"));
    }

    #[test]
    fn parse_args_accepts_explicit_geometry() {
        let args = [
            "--width".to_string(),
            "80".to_string(),
            "--height".to_string(),
            "24".to_string(),
            "--frames".to_string(),
            "3".to_string(),
        ];
        assert_eq!(
            parse_args(&args).ok(),
            Some(Options {
                width: 80,
                height: 24,
                frames: 3,
                bench: false,
                parity_report: false,
            })
        );
    }

    #[test]
    fn render_frame_writes_header_text() {
        use lunatui_core::GlyphSlot;

        let model = Model::new();
        let mut grid = Grid::new(40, 10);
        let mut clusters = ClusterTable::new();
        render_frame(&model, &mut grid, &mut clusters, 40, 10);
        assert_eq!(
            grid.get(1, 0).map(Cell::glyph),
            GlyphSlot::inline_scalar('l')
        );
        let space = GlyphSlot::inline_scalar(' ');
        let status_bits = grid.get(1, 9).map(|cell| cell.glyph().bits());
        assert!(matches!(
            (status_bits, space),
            (Some(bits), Some(space_glyph)) if bits != space_glyph.bits()
        ));
    }

    #[test]
    fn ratatui_frame_clears_log_tails_after_the_log_wraps() {
        let area = RatatuiRect::new(0, 0, 80, 24);
        let mut reused = RatatuiBuffer::empty(area);
        let mut model = Model::new();

        model.advance(796);
        render_ratatui_frame(&model, area, &mut reused);
        model.advance(800);
        render_ratatui_frame(&model, area, &mut reused);

        let mut fresh = RatatuiBuffer::empty(area);
        render_ratatui_frame(&model, area, &mut fresh);
        assert_eq!(reused, fresh);
    }
}
