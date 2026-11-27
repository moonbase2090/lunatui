// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../../README.md")]

use std::env;
use std::io;
use std::process;

use lunatui_term::{MouseTracking, Session, SessionOptions, SyncOutput};

fn main() {
    let Some(mode) = env::args().nth(1) else {
        process::exit(1);
    };
    let code = std::panic::catch_unwind(|| match mode.as_str() {
        "normal" => run_normal(),
        "panic" => run_panic(),
        "mouse" => run_mouse(MouseTracking::Sgr),
        "mouse-pixels" => run_mouse(MouseTracking::SgrPixels),
        "buffered" => run_buffered(),
        #[cfg(unix)]
        "flush-error" => run_flush_error(),
        _ => 1,
    })
    .unwrap_or(101);
    #[cfg(unix)]
    process::exit(if drain_output().is_err() && code == 0 {
        4
    } else {
        code
    });
    #[cfg(not(unix))]
    process::exit(code);
}

#[cfg(unix)]
fn drain_output() -> rustix::io::Result<()> {
    loop {
        match rustix::termios::tcdrain(io::stdout()) {
            Err(rustix::io::Errno::INTR) => {}
            result => return result,
        }
    }
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

fn run_mouse(tracking: MouseTracking) -> i32 {
    let stdout = io::stdout();
    let Ok(mut session) = Session::enter_with_options(
        stdout,
        SessionOptions::default()
            .sync_output(SyncOutput::Disabled)
            .mouse_tracking(tracking),
    ) else {
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
        SessionOptions::default()
            .sync_output(SyncOutput::Disabled)
            .mouse_tracking(MouseTracking::Disabled),
    ) else {
        return 2;
    };
    #[expect(clippy::panic, reason = "PTY integration test exercises panic restore")]
    {
        panic!("session probe panic");
    }
}

fn run_buffered() -> i32 {
    let stdout = io::BufWriter::new(io::stdout());
    let Ok(session) = Session::enter_with_options(stdout, SessionOptions::default()) else {
        return 2;
    };
    let mut session = std::mem::ManuallyDrop::new(session);
    if session.present_frame(b"buffered-frame").is_err() {
        return 3;
    }
    0
}

#[cfg(unix)]
fn run_flush_error() -> i32 {
    use std::io::Write;
    struct FailFlush;
    impl Write for FailFlush {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }
    let Ok(mut session) = Session::enter_with_options(FailFlush, SessionOptions::default()) else {
        return 2;
    };
    if let Err(lunatui_term::SessionError::Io(error)) = session.present_frame(b"probe")
        && error.kind() == io::ErrorKind::BrokenPipe
    {
        println!("flush-error-observed");
        return 0;
    }
    3
}
