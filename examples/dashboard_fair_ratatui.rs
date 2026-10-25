// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io;

use lunatui_bench::CountingAllocator;
use lunatui_bench::dashboard_fair::{FairOptions, bench_ratatui, format_report_line};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn main() -> io::Result<()> {
    let options = parse_options(&env::args().skip(1).collect::<Vec<_>>())?;
    let stats = bench_ratatui(&ALLOC, options)?;
    println!("{}", format_report_line("ratatui", &stats));
    Ok(())
}

fn parse_options(args: &[String]) -> io::Result<FairOptions> {
    let mut width = 80_u16;
    let mut height = 24_u16;
    let mut frames = 240_u32;
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
            flag => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag: {flag}"),
                ));
            }
        }
        i += 1;
    }
    Ok(FairOptions {
        width,
        height,
        frames,
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
