// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io;
use std::process::Command;
use std::time::Duration;

use lunatui_bench::dashboard_fair::{FairRunStats, parse_report_line};

#[derive(Clone, Copy)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
    runs: u32,
}

fn main() -> io::Result<()> {
    let options = parse_options(&env::args().skip(1).collect::<Vec<_>>())?;
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!(
        "dashboard_fair_report {}x{} frames={} runs={} ({})",
        options.width, options.height, options.frames, options.runs, profile
    );
    println!(
        "scope: separate release binaries dashboard_fair_lunatui (native widgets + encode) vs dashboard_fair_ratatui (ratatui 0.30 Terminal + Crossterm); identical Model/ticks; child RSS per process"
    );
    let mut luna_runs = Vec::new();
    let mut rat_runs = Vec::new();
    for run in 1..=options.runs {
        let luna = run_side("dashboard_fair_lunatui", options)?;
        let rat = run_side("dashboard_fair_ratatui", options)?;
        print_run("lunatui", run, &luna);
        print_run("ratatui", run, &rat);
        luna_runs.push(luna);
        rat_runs.push(rat);
    }
    println!("--- aggregate ---");
    print_agg("lunatui", &luna_runs);
    print_agg("ratatui", &rat_runs);
    Ok(())
}

fn run_side(example: &str, options: Options) -> io::Result<FairRunStats> {
    let mut cmd = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cmd.args([
        "run",
        "-q",
        "--release",
        "-p",
        "lunatui",
        "--example",
        example,
        "--",
        "--width",
        &options.width.to_string(),
        "--height",
        &options.height.to_string(),
        "--frames",
        &options.frames.to_string(),
    ]);
    let output = cmd.stdout(std::process::Stdio::piped()).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{example} failed with {}",
            output.status
        )));
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let line = line
        .lines()
        .rev()
        .find(|l| l.starts_with("SIDE="))
        .ok_or_else(|| io::Error::other(format!("{example} printed no SIDE= line")))?;
    parse_report_line(line)
}

fn print_run(side: &str, run: u32, stats: &FairRunStats) {
    println!(
        "run {run}: {side} median={} p95={} wall_ms={:.1} cpu_ms={:.1} rss_kib={} bytes/frame={} writes/frame={} allocs/frame≈{}",
        micros(stats.median),
        micros(stats.p95),
        stats.wall.as_secs_f64() * 1000.0,
        stats.cpu.as_secs_f64() * 1000.0,
        stats.rss_kib,
        stats.bytes_per_frame,
        stats.writes_per_frame,
        tenths(stats.allocs_per_frame_tenths),
    );
}

fn print_agg(label: &str, runs: &[FairRunStats]) {
    let mut medians: Vec<_> = runs.iter().map(|r| r.median).collect();
    let mut p95s: Vec<_> = runs.iter().map(|r| r.p95).collect();
    medians.sort_unstable();
    p95s.sort_unstable();
    let mid = runs.len() / 2;
    let alloc_sum: u64 = runs.iter().map(|r| r.allocs_per_frame_tenths).sum();
    let alloc_avg = alloc_sum / u64::try_from(runs.len().max(1)).unwrap_or(1);
    let bytes_avg = runs.iter().map(|r| r.bytes_per_frame).sum::<usize>() / runs.len().max(1);
    let writes_avg = runs.iter().map(|r| r.writes_per_frame).sum::<usize>() / runs.len().max(1);
    let rss_max = runs.iter().map(|r| r.rss_kib).max().unwrap_or(0);
    println!(
        "{label}: across {} runs median-of-medians={} median-of-p95={} avg_bytes/frame={} avg_writes/frame={} avg_allocs/frame≈{} max_rss_kib={}",
        runs.len(),
        micros(medians[mid]),
        micros(p95s[mid]),
        bytes_avg,
        writes_avg,
        tenths(alloc_avg),
        rss_max,
    );
}

fn parse_options(args: &[String]) -> io::Result<Options> {
    let mut width = 80_u16;
    let mut height = 24_u16;
    let mut frames = 240_u32;
    let mut runs = 5_u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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

fn micros(d: Duration) -> String {
    format!("{:.1} µs", d.as_secs_f64() * 1_000_000.0)
}

fn tenths(value: u64) -> String {
    format!("{}.{}", value / 10, value % 10)
}
