// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../../README.md")]

use std::env;
use std::io;
use std::process;

use lunatui_term::{Session, SessionOptions, SyncOutput};

fn main() {
    let Some(mode) = env::args().nth(1) else {
        process::exit(1);
    };
    let code = match mode.as_str() {
        "normal" => run_normal(),
        "panic" => run_panic(),
        _ => 1,
    };
    process::exit(code);
}

fn run_normal() -> i32 {
    let stdout = io::stdout();
    let Ok(mut session) = Session::enter_with_options(stdout, SessionOptions::default()) else {
        return 2;
    };
    if session.present_frame(b"probe").is_err() {
        return 3;
    }
    0
}

fn run_panic() -> i32 {
    let stdout = io::stdout();
    let Ok(_session) = Session::enter_with_options(
        stdout,
        SessionOptions {
            sync_output: SyncOutput::Disabled,
        },
    ) else {
        return 2;
    };
    #[expect(clippy::panic, reason = "PTY integration test exercises panic restore")]
    {
        panic!("session probe panic");
    }
}
