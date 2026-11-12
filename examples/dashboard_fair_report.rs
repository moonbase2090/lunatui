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
    gate: bool,
    max_ratio: f64,
}

const DEFAULT_MAX_RATIO: f64 = 1.05;
const MIN_GATE_RUNS: u32 = 5;

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
    if options.gate {
        check_timing_gate(&luna_runs, &rat_runs, options.max_ratio)?;
        println!(
            "dashboard timing gate: PASS (limit {:.3}x)",
            options.max_ratio
        );
    }
    Ok(())
}

fn check_timing_gate(
    luna: &[FairRunStats],
    rat: &[FairRunStats],
    max_ratio: f64,
) -> io::Result<()> {
    if luna.is_empty()
        || rat.is_empty()
        || luna.iter().chain(rat).any(|run| {
            run.median.is_zero() || run.bytes_per_frame == 0 || run.writes_per_frame == 0
        })
    {
        return Err(io::Error::other(
            "dashboard timing gate: missing measured work",
        ));
    }
    let median = |runs: &[FairRunStats]| {
        let mut times: Vec<_> = runs.iter().map(|run| run.median).collect();
        times.sort_unstable();
        times[times.len() / 2]
    };
    let ratio = median(luna).as_secs_f64() / median(rat).as_secs_f64();
    println!("dashboard timing gate: median-of-medians ratio={ratio:.3}x limit={max_ratio:.3}x");
    if ratio > max_ratio {
        Err(io::Error::other(format!(
            "dashboard timing gate: FAIL ({ratio:.3}x exceeds {max_ratio:.3}x)"
        )))
    } else {
        Ok(())
    }
}

fn run_side(example: &str, options: Options) -> io::Result<FairRunStats> {
    let mut cmd = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cmd.args([
        "run",
        "-q",
        "--release",
        "--locked",
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
    let output = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()?;
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
        "run {run}: {side} median={} p95={} p99={} wall_ms={:.1} cpu_ms={:.1} rss_kib={} bytes/frame={} writes/frame={} allocs/frame≈{}",
        micros(stats.median),
        micros(stats.p95),
        micros(stats.p99),
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
    let mut p99s: Vec<_> = runs.iter().map(|r| r.p99).collect();
    medians.sort_unstable();
    p95s.sort_unstable();
    p99s.sort_unstable();
    let mid = runs.len() / 2;
    let alloc_sum: u64 = runs.iter().map(|r| r.allocs_per_frame_tenths).sum();
    let alloc_avg = alloc_sum / u64::try_from(runs.len().max(1)).unwrap_or(1);
    let bytes_avg = runs.iter().map(|r| r.bytes_per_frame).sum::<usize>() / runs.len().max(1);
    let writes_avg = runs.iter().map(|r| r.writes_per_frame).sum::<usize>() / runs.len().max(1);
    let rss_max = runs.iter().map(|r| r.rss_kib).max().unwrap_or(0);
    println!(
        "{label}: across {} runs median-of-medians={} median-of-p95={} median-of-p99={} avg_bytes/frame={} avg_writes/frame={} avg_allocs/frame≈{} max_rss_kib={}",
        runs.len(),
        micros(medians[mid]),
        micros(p95s[mid]),
        micros(p99s[mid]),
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
    let mut gate = false;
    let mut max_ratio = DEFAULT_MAX_RATIO;
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
            "--gate" => gate = true,
            "--max-ratio" => {
                i += 1;
                max_ratio = parse_num(args.get(i), "--max-ratio")?;
                if !max_ratio.is_finite() || max_ratio <= 0.0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--max-ratio must be finite and greater than zero",
                    ));
                }
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
    if gate && runs < MIN_GATE_RUNS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--gate requires at least 5 runs",
        ));
    }
    Ok(Options {
        width,
        height,
        frames,
        runs,
        gate,
        max_ratio,
    })
}

fn parse_num<T: std::str::FromStr + Default + PartialEq>(
    value: Option<&String>,
    flag: &str,
) -> io::Result<T> {
    let parsed: T = value
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
        })?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))?;
    if parsed == T::default() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{flag} must be greater than zero"),
        ));
    }
    Ok(parsed)
}

fn micros(d: Duration) -> String {
    format!("{:.1} µs", d.as_secs_f64() * 1_000_000.0)
}

fn tenths(value: u64) -> String {
    format!("{}.{}", value / 10, value % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(nanos: u64) -> FairRunStats {
        FairRunStats {
            median: Duration::from_nanos(nanos),
            p95: Duration::from_nanos(nanos),
            p99: Duration::from_nanos(nanos),
            wall: Duration::from_millis(1),
            cpu: Duration::from_millis(1),
            bytes_per_frame: 100,
            writes_per_frame: 1,
            allocs_per_frame_tenths: 0,
            rss_kib: 100,
        }
    }

    #[test]
    fn timing_gate_accepts_the_five_percent_boundary() -> io::Result<()> {
        check_timing_gate(&[stats(105_000)], &[stats(100_000)], 1.05)
    }

    #[test]
    fn timing_gate_rejects_one_nanosecond_over_the_boundary() {
        let error = check_timing_gate(&[stats(105_001)], &[stats(100_000)], 1.05)
            .expect_err("a slowdown past the tolerance must fail");
        assert!(error.to_string().contains("dashboard timing gate: FAIL"));
    }

    #[test]
    fn timing_gate_uses_medians_instead_of_one_outlier() -> io::Result<()> {
        let luna = [
            stats(9_000_000),
            stats(104_000),
            stats(102_000),
            stats(103_000),
            stats(101_000),
        ];
        let rat: Vec<_> = (0..5).map(|_| stats(100_000)).collect();
        check_timing_gate(&luna, &rat, 1.05)
    }

    #[test]
    fn timing_gate_rejects_missing_work() {
        for luna in [
            vec![],
            vec![stats(0)],
            vec![FairRunStats {
                bytes_per_frame: 0,
                ..stats(100)
            }],
            vec![FairRunStats {
                writes_per_frame: 0,
                ..stats(100)
            }],
        ] {
            let error = check_timing_gate(&luna, &[stats(100)], 1.05)
                .expect_err("empty or stubbed work must fail");
            assert_eq!(
                error.to_string(),
                "dashboard timing gate: missing measured work"
            );
        }
        assert!(check_timing_gate(&[stats(100)], &[], 1.05).is_err());
        assert!(check_timing_gate(&[stats(100)], &[stats(0)], 1.05).is_err());
    }

    #[test]
    fn gate_requires_five_runs_but_reporting_can_use_one() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        assert!(parse_options(&args(&["--gate", "--runs", "4"])).is_err());
        assert!(parse_options(&args(&["--runs", "1"])).is_ok());
        let options = parse_options(&args(&["--gate"])).expect("default gate options");
        assert!(options.gate);
        assert_eq!(options.runs, 5);
        assert_eq!(options.max_ratio, 1.05);
    }

    #[test]
    fn ratio_must_be_finite_and_positive() {
        for value in ["NaN", "inf", "-inf", "0", "-1", "no"] {
            assert!(parse_options(&["--max-ratio".into(), value.into()]).is_err());
        }
        let options = parse_options(&["--max-ratio".into(), "1.2".into()]).expect("valid ratio");
        assert_eq!(options.max_ratio, 1.2);
    }
}
