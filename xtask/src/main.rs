// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]
#![expect(
    dead_code,
    reason = "M1 red commit: the check stubs are implemented in the next commit"
)]

mod bench;
mod check;
mod git;
mod signoff;

use std::env;
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask bench | check | signoff <base> <head> <labeled>";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["bench"] => bench::run(),
        ["check"] => check::run(),
        ["signoff", base, head, labeled] => signoff::run(base, head, *labeled == "true"),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
