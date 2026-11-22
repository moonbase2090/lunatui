// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;
use std::process::ExitCode;

#[cfg(feature = "graphite-pixels")]
use lunatui_bench::graphite_cases::{GRAPHITE_CASES, GraphiteRun, graphite_case};
use lunatui_bench::widget_suite::{CASES, Case, LunatuiRun, Parity, pin_color_output};

const WARMUP: u32 = 200;
const FRAMES: u32 = 32;
#[cfg(feature = "graphite-pixels")]
const GRAPHITE_WARMUP: u32 = 2;
#[cfg(feature = "graphite-pixels")]
const GRAPHITE_FRAMES: u32 = 8;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--list" || flag == "--list-gated" => {
            for case in CASES {
                if case.parity == Parity::Exact && (flag == "--list" || case.gated) {
                    println!("{}", case.name);
                }
            }
            #[cfg(feature = "graphite-pixels")]
            for case in &GRAPHITE_CASES {
                println!("{}", case.name);
            }
            ExitCode::SUCCESS
        }
        [name, width, height] => {
            let result =
                dimensions(width, height).and_then(|(width, height)| run(name, width, height));
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("widget instruction worker: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("usage: widget_instruction_worker CASE WIDTH HEIGHT | --list | --list-gated");
            ExitCode::FAILURE
        }
    }
}

fn dimensions(width: &str, height: &str) -> Result<(u16, u16), String> {
    let width = width.parse::<u16>().map_err(|error| error.to_string())?;
    let height = height.parse::<u16>().map_err(|error| error.to_string())?;
    if width == 0 || height == 0 {
        return Err("dimensions must be positive".to_owned());
    }
    Ok((width, height))
}

fn run(name: &str, width: u16, height: u16) -> Result<(), String> {
    #[cfg(feature = "graphite-pixels")]
    if let Some(case) = graphite_case(name) {
        return graphite_worker(GraphiteRun::new(case, width, height)?);
    }
    let case = CASES
        .iter()
        .find(|case| case.name == name && case.parity == Parity::Exact)
        .ok_or("unknown or non-parity instruction case")?;
    worker(case, width, height)
}

fn worker(case: &'static Case, width: u16, height: u16) -> Result<(), String> {
    pin_color_output();
    let mut run = LunatuiRun::new(case, width, height);
    for _ in 0..WARMUP {
        run.frame().map_err(|error| error.to_string())?;
    }
    instruction_frames(&mut run).map_err(|error| error.to_string())?;
    std::hint::black_box((run.tick, run.counters().bytes(), run.counters().writes()));
    if run.tick != WARMUP + FRAMES {
        return Err("the complete frame batch did not run".to_owned());
    }
    Ok(())
}

#[inline(never)]
fn instruction_frames(run: &mut LunatuiRun) -> io::Result<()> {
    for _ in 0..FRAMES {
        std::hint::black_box(run.frame())?;
    }
    Ok(())
}

#[cfg(feature = "graphite-pixels")]
fn graphite_worker(mut run: GraphiteRun) -> Result<(), String> {
    for _ in 0..GRAPHITE_WARMUP {
        run.frame().map_err(|error| error.to_string())?;
    }
    graphite_instruction_frames(&mut run).map_err(|error| error.to_string())?;
    let stats = run.stats();
    std::hint::black_box((run.tick(), stats.bytes, stats.writes));
    if run.tick() != GRAPHITE_WARMUP + GRAPHITE_FRAMES || stats.writes == 0 {
        return Err("the complete Graphite frame batch did not run".to_owned());
    }
    Ok(())
}

#[cfg(feature = "graphite-pixels")]
#[inline(never)]
fn graphite_instruction_frames(run: &mut GraphiteRun) -> io::Result<()> {
    for _ in 0..GRAPHITE_FRAMES {
        std::hint::black_box(run.frame())?;
    }
    Ok(())
}
