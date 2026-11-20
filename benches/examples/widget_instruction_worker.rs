// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;
use std::process::ExitCode;

use lunatui_bench::widget_suite::{CASES, Case, LunatuiRun, Parity, pin_color_output};

const WARMUP: u32 = 200;
const FRAMES: u32 = 32;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--list" || flag == "--list-gated" => {
            for case in CASES {
                if case.parity == Parity::Exact && (flag == "--list" || case.gated) {
                    println!("{}", case.name);
                }
            }
            ExitCode::SUCCESS
        }
        [name, width, height] => {
            let result = parameters(name, width, height)
                .and_then(|(case, width, height)| worker(case, width, height));
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

fn parameters(name: &str, width: &str, height: &str) -> Result<(&'static Case, u16, u16), String> {
    let case = CASES
        .iter()
        .find(|case| case.name == name && case.parity == Parity::Exact)
        .ok_or("unknown or non-parity instruction case")?;
    let width = width.parse::<u16>().map_err(|error| error.to_string())?;
    let height = height.parse::<u16>().map_err(|error| error.to_string())?;
    if width == 0 || height == 0 {
        return Err("dimensions must be positive".to_owned());
    }
    Ok((case, width, height))
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
