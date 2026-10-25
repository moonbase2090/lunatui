// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Grid};
use lunatui_term::{ColorMode, Encoder};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect as RatRect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::scenes::lunatui::dashboard as luna_dash;
use crate::scenes::ratatui::dashboard::{self as rat_dash, Model, advance_scripted};
use crate::{AllocStats, CountingAllocator, CountingWriter, WriteStats};

/// Terminal size and frame count for a fair dashboard run.
#[derive(Clone, Copy, Debug)]
pub struct FairOptions {
    /// Viewport width in cells.
    pub width: u16,
    /// Viewport height in cells.
    pub height: u16,
    /// Scripted animation ticks.
    pub frames: u32,
}

/// Aggregated metrics from one fair dashboard process run.
#[derive(Clone, Debug)]
pub struct FairRunStats {
    /// Median frame draw time.
    pub median: Duration,
    /// 95th percentile frame draw time.
    pub p95: Duration,
    /// Wall time for the measured loop.
    pub wall: Duration,
    /// Process CPU time (user + system) over the run.
    pub cpu: Duration,
    /// Bytes written to the terminal sink per frame.
    pub bytes_per_frame: usize,
    /// `write()` calls per frame.
    pub writes_per_frame: usize,
    /// Allocations per frame times ten (one decimal).
    pub allocs_per_frame_tenths: u64,
    /// Peak RSS of this process in KiB.
    pub rss_kib: usize,
}

/// Measures lunatui native dashboard paint + encode to a counting sink.
///
/// # Errors
///
/// Returns an error if writing to the sink fails.
pub fn bench_lunatui(alloc: &CountingAllocator, options: FairOptions) -> io::Result<FairRunStats> {
    let mut model = Model::new();
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut samples = Vec::new();
    let cpu_start = process_cpu_time();
    let wall_start = Instant::now();
    let (inner, alloc_stats) = alloc.measure(|| -> io::Result<WriteStats> {
        let mut output = CountingWriter::new(io::sink());
        for tick in 0..options.frames {
            advance_scripted(&mut model, tick);
            let start = Instant::now();
            luna_dash::render(
                &model,
                options.width,
                options.height,
                &mut next,
                &mut clusters,
            );
            let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
            output.write_all(payload)?;
            samples.push(start.elapsed());
            std::mem::swap(&mut prev, &mut next);
        }
        Ok(output.stats())
    });
    let write = inner?;
    let cpu = process_cpu_time().saturating_sub(cpu_start);
    Ok(summarize(
        &samples,
        wall_start.elapsed(),
        cpu,
        write,
        alloc_stats,
        options.frames,
        rss_kib(),
    ))
}

/// Measures stock ratatui `Terminal` + Crossterm backend on a counting sink.
///
/// # Errors
///
/// Returns an error if terminal setup or drawing fails.
pub fn bench_ratatui(alloc: &CountingAllocator, options: FairOptions) -> io::Result<FairRunStats> {
    let mut model = Model::new();
    let mut output = CountingWriter::new(io::sink());
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(io::LineWriter::new(&mut output)),
        TerminalOptions {
            viewport: Viewport::Fixed(RatRect::new(0, 0, options.width, options.height)),
        },
    )?;
    let mut samples = Vec::new();
    let cpu_start = process_cpu_time();
    let wall_start = Instant::now();
    let (inner, alloc_stats) = alloc.measure(|| -> io::Result<()> {
        for tick in 0..options.frames {
            advance_scripted(&mut model, tick);
            let start = Instant::now();
            terminal.draw(|frame| {
                rat_dash::render(&model, frame.area(), frame.buffer_mut());
            })?;
            samples.push(start.elapsed());
        }
        Ok(())
    });
    inner?;
    drop(terminal);
    let write = output.stats();
    let cpu = process_cpu_time().saturating_sub(cpu_start);
    Ok(summarize(
        &samples,
        wall_start.elapsed(),
        cpu,
        write,
        alloc_stats,
        options.frames,
        rss_kib(),
    ))
}

fn summarize(
    samples: &[Duration],
    wall: Duration,
    cpu: Duration,
    write: WriteStats,
    alloc: AllocStats,
    frames: u32,
    rss_kib: usize,
) -> FairRunStats {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let frames_usize = usize::try_from(frames).unwrap_or(1).max(1);
    let allocs_tenths = (alloc.allocations.saturating_mul(10)) / frames_usize;
    FairRunStats {
        median: percentile(&sorted, 50),
        p95: percentile(&sorted, 95),
        wall,
        cpu,
        bytes_per_frame: write.bytes / frames_usize,
        writes_per_frame: write.writes / frames_usize,
        allocs_per_frame_tenths: u64::try_from(allocs_tenths).unwrap_or(u64::MAX),
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

/// Machine-readable one-line summary for orchestrator subprocesses.
#[must_use]
pub fn format_report_line(side: &str, stats: &FairRunStats) -> String {
    format!(
        "SIDE={side} median_ns={} p95_ns={} wall_ns={} cpu_ns={} bytes_per_frame={} writes_per_frame={} allocs_tenths={} rss_kib={}",
        stats.median.as_nanos(),
        stats.p95.as_nanos(),
        stats.wall.as_nanos(),
        stats.cpu.as_nanos(),
        stats.bytes_per_frame,
        stats.writes_per_frame,
        stats.allocs_per_frame_tenths,
        stats.rss_kib,
    )
}

/// Parses a line emitted by [`format_report_line`].
///
/// # Errors
///
/// Returns an error if a field is missing or not a number.
pub fn parse_report_line(line: &str) -> io::Result<FairRunStats> {
    let mut median_ns = None;
    let mut p95_ns = None;
    let mut wall_ns = None;
    let mut cpu_ns = None;
    let mut bytes_per_frame = None;
    let mut writes_per_frame = None;
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
            "cpu_ns" => cpu_ns = Some(parse_u128(v)?),
            "bytes_per_frame" => bytes_per_frame = Some(parse_usize(v)?),
            "writes_per_frame" => writes_per_frame = Some(parse_usize(v)?),
            "allocs_tenths" => allocs_tenths = Some(parse_u64(v)?),
            "rss_kib" => rss_kib = Some(parse_usize(v)?),
            _ => {}
        }
    }
    Ok(FairRunStats {
        median: Duration::from_nanos(
            u64::try_from(median_ns.ok_or_else(missing)?).unwrap_or(u64::MAX),
        ),
        p95: Duration::from_nanos(u64::try_from(p95_ns.ok_or_else(missing)?).unwrap_or(u64::MAX)),
        wall: Duration::from_nanos(u64::try_from(wall_ns.ok_or_else(missing)?).unwrap_or(u64::MAX)),
        cpu: Duration::from_nanos(u64::try_from(cpu_ns.ok_or_else(missing)?).unwrap_or(u64::MAX)),
        bytes_per_frame: bytes_per_frame.ok_or_else(missing)?,
        writes_per_frame: writes_per_frame.ok_or_else(missing)?,
        allocs_per_frame_tenths: allocs_tenths.ok_or_else(missing)?,
        rss_kib: rss_kib.ok_or_else(missing)?,
    })
}

fn missing() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "incomplete fair bench stats line",
    )
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

#[cfg(unix)]
fn process_cpu_time() -> Duration {
    use std::mem::MaybeUninit;

    let mut usage = MaybeUninit::uninit();
    let usage = {
        // SAFETY: `getrusage` fully initializes `usage` when it returns zero.
        unsafe {
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
                return Duration::ZERO;
            }
            usage.assume_init()
        }
    };
    let secs = usage.ru_utime.tv_sec + usage.ru_stime.tv_sec;
    let usec = usage.ru_utime.tv_usec + usage.ru_stime.tv_usec;
    let secs = u64::try_from(secs).unwrap_or(0);
    let nanos = u32::try_from(usec).unwrap_or(0) * 1000;
    Duration::new(secs, nanos)
}

#[cfg(not(unix))]
fn process_cpu_time() -> Duration {
    Duration::ZERO
}

#[cfg(unix)]
fn rss_kib() -> usize {
    use std::mem::MaybeUninit;

    let mut usage = MaybeUninit::uninit();
    let usage = {
        // SAFETY: `getrusage` fully initializes `usage` when it returns zero.
        unsafe {
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
                return 0;
            }
            usage.assume_init()
        }
    };
    let max_rss = usize::try_from(usage.ru_maxrss).unwrap_or(0);
    if cfg!(target_os = "macos") {
        max_rss / 1024
    } else {
        max_rss
    }
}

#[cfg(not(unix))]
fn rss_kib() -> usize {
    0
}
