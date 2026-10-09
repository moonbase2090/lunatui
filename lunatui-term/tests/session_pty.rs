// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg(unix)]
#![doc = include_str!("../README.md")]

use std::fs::File;
use std::io::{self, Read};
use std::os::unix::io::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[test]
fn session_enters_alt_screen_and_restores_on_drop() {
    let output = run_probe_in_pty("normal");
    assert!(
        output.windows(8).any(|w| w == b"\x1b[?1049h"),
        "expected alternate-screen enter"
    );
    assert!(
        output.windows(8).any(|w| w == b"\x1b[?1049l"),
        "expected alternate-screen leave"
    );
    assert!(
        output.windows(5).any(|w| w == b"probe"),
        "expected frame bytes"
    );
}

#[test]
fn session_restores_after_panic() {
    let output = run_probe_in_pty("panic");
    assert!(
        output.windows(8).any(|w| w == b"\x1b[?1049l"),
        "expected alternate-screen leave after panic"
    );
}

fn run_probe_in_pty(mode: &str) -> Vec<u8> {
    let probe = match std::env::var_os("CARGO_BIN_EXE_session-pty-probe") {
        Some(path) => std::path::PathBuf::from(path),
        None => return Vec::new(),
    };
    let mut master = 0;
    let mut slave = 0;
    let open = {
        // SAFETY: `openpty` writes valid fds to `master` and `slave` when it returns zero.
        unsafe {
            libc::openpty(
                &raw mut master,
                &raw mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    };
    if open != 0 {
        return Vec::new();
    }

    let mut command = Command::new(probe);
    command.arg(mode);
    // SAFETY: `login_tty` runs in the child between `fork` and `exec` with a valid slave fd.
    unsafe {
        command.pre_exec(move || {
            if libc::login_tty(slave) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let Ok(mut child) = command.spawn() else {
        return Vec::new();
    };
    let master_file = {
        // SAFETY: `master` is an open fd returned by `openpty`.
        unsafe { File::from_raw_fd(master) }
    };
    let reader = thread::spawn(move || drain_master(master_file));
    let Ok(status) = child.wait() else {
        return Vec::new();
    };
    thread::sleep(Duration::from_millis(20));
    let buf: Vec<u8> = reader.join().unwrap_or_default();
    if !status.success() {
        return buf;
    }
    buf
}

fn drain_master(mut master_file: File) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 512];
    loop {
        match master_file.read(&mut scratch) {
            Ok(0) | Err(_) => break,
            Ok(count) => buf.extend_from_slice(&scratch[..count]),
        }
    }
    buf
}
