// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod bench;
mod check;
mod gen_width;
mod git;
mod signoff;

use std::env;
use std::process::ExitCode;

const USAGE: &str =
    "usage: cargo xtask bench | check | gen-width | signoff <base> <head> <labeled>";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["bench"] => bench::run(),
        ["check"] => check::run(),
        ["gen-width"] => gen_width::run(),
        ["signoff", base, head, labeled] => signoff::run(base, head, *labeled == "true"),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
