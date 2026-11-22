// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::io;
use std::process::ExitCode;
use std::time::Duration;

use lunatui_bench::calibration::{frame, truecolor_frame_bytes, write_frame};
use lunatui_bench::ceiling::{FRAME_CEILING, FRAME_TARGET, FrameTimes, Verdict, sample};
#[cfg(feature = "graphite-pixels")]
use lunatui_bench::graphite_cases::{GRAPHITE_CASES, GraphiteRun, Repaint};

const SAMPLES: usize = 1000;

#[cfg(feature = "graphite-pixels")]
const GRAPHITE_WINDOW: (u16, u16) = (185, 51);
#[cfg(feature = "graphite-pixels")]
const GRAPHITE_SAMPLES: usize = 200;

const WORST_CASES: [(&str, usize, usize); 2] = [
    ("truecolor repaint write, 400x120 (stand-in)", 400, 120),
    ("truecolor repaint write, 800x240 (stand-in)", 800, 240),
];

fn main() -> io::Result<ExitCode> {
    println!(
        "{:<45}{:>8}{:>11}{:>11}{:>11}{:>11}  target {}, ceiling {}",
        "worst case",
        "samples",
        "p50",
        "p95",
        "p99",
        "max",
        millis(FRAME_TARGET),
        millis(FRAME_CEILING)
    );
    let mut over_ceiling = false;
    for (name, cols, rows) in WORST_CASES {
        let times = time_truecolor_write(cols, rows)?;
        over_ceiling |= report(name, &times, true);
    }
    #[cfg(feature = "graphite-pixels")]
    for case in &GRAPHITE_CASES {
        let times = time_graphite(case)?;
        let gated = case.repaint == Repaint::Changed;
        let name = format!(
            "{}, {}x{}{}",
            case.name,
            GRAPHITE_WINDOW.0,
            GRAPHITE_WINDOW.1,
            if gated { "" } else { " (reported)" }
        );
        over_ceiling |= report(&name, &times, gated);
    }
    if over_ceiling {
        eprintln!(
            "frame-time ceiling exceeded: a worst-case frame took longer than {}",
            millis(FRAME_CEILING)
        );
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn report(name: &str, times: &FrameTimes, gated: bool) -> bool {
    let verdict = times.verdict();
    println!(
        "{name:<45}{:>8}{:>11}{:>11}{:>11}{:>11}  {}",
        times.samples,
        millis(times.p50),
        millis(times.p95),
        millis(times.p99),
        millis(times.max),
        verdict_label(verdict)
    );
    gated && verdict == Verdict::OverCeiling
}

#[cfg(feature = "graphite-pixels")]
fn time_graphite(
    case: &'static lunatui_bench::graphite_cases::GraphiteCase,
) -> io::Result<FrameTimes> {
    let (cols, rows) = GRAPHITE_WINDOW;
    let mut run = GraphiteRun::new(case, cols, rows).map_err(io::Error::other)?;
    run.frame()?;
    let samples = sample(GRAPHITE_SAMPLES, || run.frame())?;
    FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no frame-time samples"))
}

fn verdict_label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::WithinTarget => "within target",
        Verdict::OverTarget => "over target",
        Verdict::OverCeiling => "OVER CEILING",
    }
}

fn time_truecolor_write(cols: usize, rows: usize) -> io::Result<FrameTimes> {
    let len = truecolor_frame_bytes(cols, rows);
    let repaint = frame(len);
    let mut out = Vec::with_capacity(len);
    let samples = sample(SAMPLES, || {
        out.clear();
        write_frame(&mut out, black_box(&repaint))
    })?;
    FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no frame-time samples"))
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}
