// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::process::ExitCode;
use std::time::{Duration, Instant};

use lunatui_bench::CountingAllocator;
use lunatui_bench::widget_suite::{CASES, Case, LunatuiRun, Parity, RatatuiRun, SIZES};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const WARMUP: u32 = 200;
const SAMPLES: usize = 41;
const BATCH: u32 = 50;
const ALLOC_FRAMES: u32 = 100;
const DEFAULT_MAX_RATIO: f64 = 1.05;

struct Side {
    render: Duration,
    frame: Duration,
    allocs_per_frame: f64,
    bytes_per_frame: usize,
    writes_per_frame: usize,
}

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn sample(mut work: impl FnMut()) -> Duration {
    for _ in 0..WARMUP {
        work();
    }
    let samples = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..BATCH {
                work();
            }
            start.elapsed() / BATCH
        })
        .collect();
    median(samples)
}

fn lunatui_side(case: &'static Case, width: u16, height: u16) -> Side {
    let mut render_run = LunatuiRun::new(case, width, height);
    let render = sample(|| render_run.render());
    let mut frame_run = LunatuiRun::new(case, width, height);
    let frame = sample(|| {
        let _ = frame_run.frame();
    });
    let mut counted = LunatuiRun::new(case, width, height);
    for _ in 0..WARMUP {
        let _ = counted.frame();
    }
    let counters = counted.counters();
    let (bytes0, writes0) = (counters.bytes(), counters.writes());
    let ((), stats) = ALLOC.measure(|| {
        for _ in 0..ALLOC_FRAMES {
            let _ = counted.frame();
        }
    });
    Side {
        render,
        frame,
        allocs_per_frame: allocs(stats.allocations),
        bytes_per_frame: (counters.bytes() - bytes0) / usize::try_from(ALLOC_FRAMES).unwrap_or(1),
        writes_per_frame: (counters.writes() - writes0)
            / usize::try_from(ALLOC_FRAMES).unwrap_or(1),
    }
}

fn ratatui_side(case: &'static Case, width: u16, height: u16) -> std::io::Result<Side> {
    let mut render_run = RatatuiRun::new(case, width, height)?;
    let render = sample(|| render_run.render());
    let mut frame_run = RatatuiRun::new(case, width, height)?;
    let frame = sample(|| {
        let _ = frame_run.frame();
    });
    let mut counted = RatatuiRun::new(case, width, height)?;
    for _ in 0..WARMUP {
        counted.frame()?;
    }
    let counters = counted.counters();
    let (bytes0, writes0) = (counters.bytes(), counters.writes());
    let (result, stats) = ALLOC.measure(|| -> std::io::Result<()> {
        for _ in 0..ALLOC_FRAMES {
            counted.frame()?;
        }
        Ok(())
    });
    result?;
    Ok(Side {
        render,
        frame,
        allocs_per_frame: allocs(stats.allocations),
        bytes_per_frame: (counters.bytes() - bytes0) / usize::try_from(ALLOC_FRAMES).unwrap_or(1),
        writes_per_frame: (counters.writes() - writes0)
            / usize::try_from(ALLOC_FRAMES).unwrap_or(1),
    })
}

fn allocs(total: usize) -> f64 {
    f64::from(u32::try_from(total).unwrap_or(u32::MAX)) / f64::from(ALLOC_FRAMES)
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}

fn max_ratio(args: &[String]) -> f64 {
    args.windows(2)
        .find(|pair| pair[0] == "--max-ratio")
        .and_then(|pair| pair[1].parse().ok())
        .unwrap_or(DEFAULT_MAX_RATIO)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let gate = args.iter().any(|arg| arg == "--gate");
    let limit = max_ratio(&args);
    println!(
        "widget_suite: median of {SAMPLES} samples of {BATCH} iterations, release, per case and size"
    );
    println!(
        "{:<10} {:>7} {:>6} {:>10} {:>10} {:>6} {:>10} {:>10} {:>6} {:>8} {:>8} {:>6} {:>6}",
        "case",
        "size",
        "parity",
        "render lu",
        "render rt",
        "ratio",
        "frame lu",
        "frame rt",
        "ratio",
        "alloc lu",
        "alloc rt",
        "wr lu",
        "wr rt"
    );
    let mut failures = Vec::new();
    for case in CASES {
        for (width, height) in SIZES {
            let lunatui = lunatui_side(case, width, height);
            let ratatui = match ratatui_side(case, width, height) {
                Ok(side) => side,
                Err(error) => {
                    eprintln!("{} {width}x{height}: {error}", case.name);
                    return ExitCode::FAILURE;
                }
            };
            let render_ratio = micros(lunatui.render) / micros(ratatui.render);
            let frame_ratio = micros(lunatui.frame) / micros(ratatui.frame);
            let exact = case.parity == Parity::Exact;
            let gated = exact && case.gated;
            println!(
                "{:<10} {:>7} {:>6} {:>8.1}us {:>8.1}us {:>6.2} {:>8.1}us {:>8.1}us {:>6.2} {:>8.1} {:>8.1} {:>6} {:>6}",
                case.name,
                format!("{width}x{height}"),
                if !exact {
                    "doc"
                } else if case.gated {
                    "gated"
                } else {
                    "track"
                },
                micros(lunatui.render),
                micros(ratatui.render),
                render_ratio,
                micros(lunatui.frame),
                micros(ratatui.frame),
                frame_ratio,
                lunatui.allocs_per_frame,
                ratatui.allocs_per_frame,
                lunatui.writes_per_frame,
                ratatui.writes_per_frame,
            );
            let _ = (lunatui.bytes_per_frame, ratatui.bytes_per_frame);
            if gated {
                if frame_ratio > limit {
                    failures.push(format!(
                        "{} {width}x{height} frame ratio {frame_ratio:.2} > {limit:.2}",
                        case.name
                    ));
                }
                if lunatui.allocs_per_frame > ratatui.allocs_per_frame {
                    failures.push(format!(
                        "{} {width}x{height} allocations {:.1} > {:.1}",
                        case.name, lunatui.allocs_per_frame, ratatui.allocs_per_frame
                    ));
                }
            }
        }
    }
    if failures.is_empty() {
        return ExitCode::SUCCESS;
    }
    for failure in &failures {
        eprintln!("over the bar: {failure}");
    }
    if gate {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
