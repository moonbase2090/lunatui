// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use lunatui_bench::scenes::lunatui::dashboard as luna_dash;
use lunatui_bench::scenes::ratatui::dashboard::{self as rat_dash, Model};
use lunatui_bench::{AllocStats, CountingAllocator};
use lunatui_core::{ClusterTable, Grid};
use lunatui_term::{ColorMode, Encoder};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect as RatRect;
use ratatui::{Terminal, TerminalOptions, Viewport};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[derive(Clone, Copy)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
    runs: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Lunatui,
    Ratatui,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(side) = parse_child_side(&args)? {
        let options = parse_options(&args)?;
        let stats = match side {
            Side::Lunatui => bench_lunatui(options),
            Side::Ratatui => bench_ratatui(options)?,
        };
        println!("{}", format_child_line(side, &stats));
        return Ok(());
    }
    let options = parse_options(&args)?;
    println!(
        "dashboard_parity_bench {}x{} frames={} runs={} ({})",
        options.width,
        options.height,
        options.frames,
        options.runs,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    println!(
        "scope: lunatui scenes::lunatui::dashboard::render + encode + sink vs ratatui dashboard::render + Crossterm sink; RSS measured in separate child processes"
    );
    let exe = env::current_exe()?;
    let mut luna_runs = Vec::new();
    let mut rat_runs = Vec::new();
    for run in 1..=options.runs {
        let luna = run_child(&exe, Side::Lunatui, options)?;
        let rat = run_child(&exe, Side::Ratatui, options)?;
        println!(
            "run {run}: lunatui median={} p95={} wall_ms={:.1} rss_kib={} bytes/frame≈{} allocs/frame≈{}",
            micros(luna.median),
            micros(luna.p95),
            luna.wall.as_secs_f64() * 1000.0,
            luna.rss_kib,
            luna.bytes_per_frame,
            tenths(luna.allocs_tenths),
        );
        println!(
            "run {run}: ratatui median={} p95={} wall_ms={:.1} rss_kib={} bytes/frame≈{} allocs/frame≈{}",
            micros(rat.median),
            micros(rat.p95),
            rat.wall.as_secs_f64() * 1000.0,
            rat.rss_kib,
            rat.bytes_per_frame,
            tenths(rat.allocs_tenths),
        );
        luna_runs.push(luna);
        rat_runs.push(rat);
    }
    println!("--- aggregate ---");
    print_agg("lunatui", &luna_runs);
    print_agg("ratatui", &rat_runs);
    println!(
        "note: lunatui times scenes::lunatui::dashboard (same painter as examples/lunatui_dashboard); ratatui times scenes::ratatui::dashboard::render. RSS is peak resident set of each child process alone."
    );
    Ok(())
}

fn parse_child_side(args: &[String]) -> io::Result<Option<Side>> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--child" {
            let side = args.get(i + 1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "--child needs a side")
            })?;
            return match side.as_str() {
                "lunatui" => Ok(Some(Side::Lunatui)),
                "ratatui" => Ok(Some(Side::Ratatui)),
                other => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown --child side: {other}"),
                )),
            };
        }
        i += 1;
    }
    Ok(None)
}

fn parse_options(args: &[String]) -> io::Result<Options> {
    let mut width = 80_u16;
    let mut height = 24_u16;
    let mut frames = 240_u32;
    let mut runs = 5_u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--child" => {
                i += 2;
                continue;
            }
            "--width" => {
                i += 1;
                width = parse_num(args.get(i), "--width")?;
            }
            "--height" => {
                i += 1;
                height = parse_num(args.get(i), "--height")?;
            }
            "--frames" => {
                i += 1;
                frames = parse_num(args.get(i), "--frames")?;
            }
            "--runs" => {
                i += 1;
                runs = parse_num(args.get(i), "--runs")?;
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
        width,
        height,
        frames,
        runs,
    })
}

fn parse_num<T: std::str::FromStr>(value: Option<&String>, flag: &str) -> io::Result<T> {
    value
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
        })?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

#[derive(Clone, Debug)]
struct RunStats {
    median: Duration,
    p95: Duration,
    wall: Duration,
    bytes_per_frame: usize,
    allocs_tenths: u64,
    rss_kib: usize,
}

fn run_child(exe: &std::path::Path, side: Side, options: Options) -> io::Result<RunStats> {
    let side_flag = match side {
        Side::Lunatui => "lunatui",
        Side::Ratatui => "ratatui",
    };
    let output = Command::new(exe)
        .args([
            "--child",
            side_flag,
            "--width",
            &options.width.to_string(),
            "--height",
            &options.height.to_string(),
            "--frames",
            &options.frames.to_string(),
            "--runs",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "child {side_flag} failed with {}",
            output.status
        )));
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let line = line
        .lines()
        .rev()
        .find(|l| l.starts_with("SIDE="))
        .ok_or_else(|| io::Error::other("child printed no SIDE= line"))?;
    parse_child_line(line)
}

fn format_child_line(side: Side, stats: &RunStats) -> String {
    let side = match side {
        Side::Lunatui => "lunatui",
        Side::Ratatui => "ratatui",
    };
    format!(
        "SIDE={side} median_ns={} p95_ns={} wall_ns={} bytes_per_frame={} allocs_tenths={} rss_kib={}",
        stats.median.as_nanos(),
        stats.p95.as_nanos(),
        stats.wall.as_nanos(),
        stats.bytes_per_frame,
        stats.allocs_tenths,
        stats.rss_kib,
    )
}

fn parse_child_line(line: &str) -> io::Result<RunStats> {
    let mut median_ns = None;
    let mut p95_ns = None;
    let mut wall_ns = None;
    let mut bytes_per_frame = None;
    let mut allocs_tenths = None;
    let mut rss_kib = None;
    for part in line.split_whitespace() {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        match k {
            "median_ns" => median_ns = Some(parse_u128(v)?),
            "p95_ns" => p95_ns = Some(parse_u128(v)?),
            "wall_ns" => wall_ns = Some(parse_u128(v)?),
            "bytes_per_frame" => bytes_per_frame = Some(parse_usize(v)?),
            "allocs_tenths" => allocs_tenths = Some(parse_u64(v)?),
            "rss_kib" => rss_kib = Some(parse_usize(v)?),
            _ => {}
        }
    }
    Ok(RunStats {
        median: Duration::from_nanos(
            u64::try_from(median_ns.ok_or_else(missing)?).unwrap_or(u64::MAX),
        ),
        p95: Duration::from_nanos(u64::try_from(p95_ns.ok_or_else(missing)?).unwrap_or(u64::MAX)),
        wall: Duration::from_nanos(u64::try_from(wall_ns.ok_or_else(missing)?).unwrap_or(u64::MAX)),
        bytes_per_frame: bytes_per_frame.ok_or_else(missing)?,
        allocs_tenths: allocs_tenths.ok_or_else(missing)?,
        rss_kib: rss_kib.ok_or_else(missing)?,
    })
}

fn missing() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "incomplete child stats line")
}

fn parse_u128(v: &str) -> io::Result<u128> {
    v.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad u128"))
}

fn parse_u64(v: &str) -> io::Result<u64> {
    v.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad u64"))
}

fn parse_usize(v: &str) -> io::Result<usize> {
    v.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad usize"))
}

fn bench_lunatui(options: Options) -> RunStats {
    let mut model = Model::new();
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut samples = Vec::new();
    let mut output = Vec::new();
    let mut bytes = 0_usize;
    let wall_start = Instant::now();
    let ((), alloc) = ALLOC.measure(|| {
        for tick in 0..options.frames {
            model.tick = tick;
            model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
            let start = Instant::now();
            luna_dash::render(
                &model,
                options.width,
                options.height,
                &mut next,
                &mut clusters,
            );
            let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
            let _ = output.write_all(payload);
            bytes = bytes.saturating_add(payload.len());
            samples.push(start.elapsed());
            std::mem::swap(&mut prev, &mut next);
        }
    });
    summarize(
        &samples,
        wall_start.elapsed(),
        bytes,
        alloc,
        options.frames,
        rss_kib(),
    )
}

fn bench_ratatui(options: Options) -> io::Result<RunStats> {
    let mut model = Model::new();
    let total = Arc::new(AtomicUsize::new(0));
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(CountSink {
            total: Arc::clone(&total),
        }),
        TerminalOptions {
            viewport: Viewport::Fixed(RatRect::new(0, 0, options.width, options.height)),
        },
    )?;
    let mut samples = Vec::new();
    let wall_start = Instant::now();
    let ((), alloc) = ALLOC.measure(|| {
        for tick in 0..options.frames {
            model.tick = tick;
            model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
            let start = Instant::now();
            let _ = terminal.draw(|frame| {
                rat_dash::render(&model, frame.area(), frame.buffer_mut());
            });
            samples.push(start.elapsed());
        }
    });
    let bytes = total.load(Ordering::Relaxed);
    Ok(summarize(
        &samples,
        wall_start.elapsed(),
        bytes,
        alloc,
        options.frames,
        rss_kib(),
    ))
}

fn summarize(
    samples: &[Duration],
    wall: Duration,
    bytes: usize,
    alloc: AllocStats,
    frames: u32,
    rss_kib: usize,
) -> RunStats {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let frames_usize = usize::try_from(frames).unwrap_or(1).max(1);
    let allocs_tenths = (alloc.allocations.saturating_mul(10)) / frames_usize;
    RunStats {
        median: percentile(&sorted, 50),
        p95: percentile(&sorted, 95),
        wall,
        bytes_per_frame: bytes / frames_usize,
        allocs_tenths: u64::try_from(allocs_tenths).unwrap_or(u64::MAX),
        rss_kib,
    }
}

fn percentile(sorted: &[Duration], pct: usize) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() - 1) * pct) / 100;
    sorted[idx]
}

fn print_agg(label: &str, runs: &[RunStats]) {
    let mut medians: Vec<_> = runs.iter().map(|r| r.median).collect();
    let mut p95s: Vec<_> = runs.iter().map(|r| r.p95).collect();
    medians.sort_unstable();
    p95s.sort_unstable();
    let mid = runs.len() / 2;
    let alloc_sum: u64 = runs.iter().map(|r| r.allocs_tenths).sum();
    let alloc_avg = alloc_sum / u64::try_from(runs.len().max(1)).unwrap_or(1);
    let bytes_avg = runs.iter().map(|r| r.bytes_per_frame).sum::<usize>() / runs.len().max(1);
    let rss_max = runs.iter().map(|r| r.rss_kib).max().unwrap_or(0);
    println!(
        "{label}: across {} runs median-of-medians={} median-of-p95={} avg_bytes/frame≈{} avg_allocs/frame≈{} max_rss_kib={}",
        runs.len(),
        micros(medians[mid]),
        micros(p95s[mid]),
        bytes_avg,
        tenths(alloc_avg),
        rss_max,
    );
}

fn micros(d: Duration) -> String {
    format!("{:.1} µs", d.as_secs_f64() * 1_000_000.0)
}

fn tenths(value: u64) -> String {
    format!("{}.{}", value / 10, value % 10)
}

fn rss_kib() -> usize {
    let pid = std::process::id();
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<usize>()
                .ok()
        })
        .unwrap_or(0)
}

struct CountSink {
    total: Arc<AtomicUsize>,
}

impl Write for CountSink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.total.fetch_add(data.len(), Ordering::Relaxed);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
