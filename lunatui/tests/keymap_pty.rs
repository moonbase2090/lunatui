// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(unix)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const BASE_HINT: &str = "Press F1 for shortcuts";
const OVERLAY_TITLE: &str = "Show shortcuts";
const PALETTE_ROW: &str = "> Show shortcuts";
const F1_LEGACY: &[u8] = b"\x1bOP";
const CTRL_SHIFT_P_KITTY: &[u8] = b"\x1b[112;6u";
const TIMEOUT: Duration = Duration::from_secs(5);

fn visible_text(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        out.push(b' ');
        index += 1;
        match bytes.get(index) {
            Some(b'[') => {
                index += 1;
                while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) {
                    index += 1;
                }
                index += 1;
            }
            Some(b'O') => index += 2,
            _ => index += 1,
        }
    }
    String::from_utf8_lossy(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn example_path() -> io::Result<PathBuf> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let built = Command::new(cargo)
        .args([
            "build",
            "--quiet",
            "-p",
            "lunatui",
            "--example",
            "keymap",
            "--features",
            "keymap",
        ])
        .status()?;
    if !built.success() {
        return Err(io::Error::other("cargo build of the keymap example failed"));
    }
    let exe = std::env::current_exe()?;
    let debug = exe
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("no target/debug directory"))?;
    Ok(debug.join("examples").join("keymap"))
}

struct Pty {
    child: Child,
    master: File,
    output: Arc<Mutex<Vec<u8>>>,
}

impl Pty {
    fn spawn() -> io::Result<Self> {
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
            return Err(io::Error::last_os_error());
        }

        let mut command = Command::new(example_path()?);
        let master_fd = master;
        let slave_fd = slave;
        // SAFETY: `login_tty` runs in the child between `fork` and `exec` with a valid slave fd.
        unsafe {
            command.pre_exec(move || {
                libc::close(master_fd);
                if libc::login_tty(slave_fd) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let spawned = command.spawn();
        // SAFETY: The parent drops its slave fd so the master sees EOF when the child exits.
        unsafe {
            libc::close(slave);
        }
        let child = match spawned {
            Ok(child) => child,
            Err(error) => {
                // SAFETY: `master` is still open in this process.
                unsafe { libc::close(master) };
                return Err(error);
            }
        };

        let master_file = {
            // SAFETY: `master` is an open fd returned by `openpty`, owned by this function.
            unsafe { File::from_raw_fd(master) }
        };
        let mut reader = master_file.try_clone()?;
        let output = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&output);
        thread::spawn(move || {
            let mut scratch = [0u8; 512];
            while let Ok(count) = reader.read(&mut scratch) {
                if count == 0 {
                    break;
                }
                if let Ok(mut buffer) = sink.lock() {
                    buffer.extend_from_slice(&scratch[..count]);
                }
            }
        });
        Ok(Self {
            child,
            master: master_file,
            output,
        })
    }

    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.master.write_all(bytes)
    }

    fn output_contains(&self, needle: &str) -> bool {
        self.output
            .lock()
            .is_ok_and(|buffer| visible_text(&buffer).contains(needle))
    }

    fn wait_for(&self, needle: &str) -> bool {
        let deadline = Instant::now() + TIMEOUT;
        while Instant::now() < deadline {
            if self.output_contains(needle) {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn wait_exit(&mut self) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + TIMEOUT;
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait()? {
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(20));
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "example did not exit",
        ))
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn legacy_f1_opens_shortcut_overlay() -> io::Result<()> {
    let mut pty = Pty::spawn()?;
    assert!(pty.wait_for(BASE_HINT), "base screen never drew");
    assert!(!pty.output_contains(OVERLAY_TITLE));
    pty.send(F1_LEGACY)?;
    assert!(
        pty.wait_for(OVERLAY_TITLE),
        "F1 did not open the shortcut overlay"
    );
    Ok(())
}

#[test]
fn kitty_ctrl_shift_p_opens_palette() -> io::Result<()> {
    let mut pty = Pty::spawn()?;
    assert!(pty.wait_for(BASE_HINT), "base screen never drew");
    pty.send(CTRL_SHIFT_P_KITTY)?;
    assert!(
        pty.wait_for(PALETTE_ROW),
        "Ctrl+Shift+P did not open the palette with the first command selected"
    );
    Ok(())
}

#[test]
fn plain_slash_does_not_open_overlay() -> io::Result<()> {
    let mut pty = Pty::spawn()?;
    assert!(pty.wait_for(BASE_HINT), "base screen never drew");
    pty.send(b"/")?;
    thread::sleep(Duration::from_millis(300));
    assert!(
        !pty.output_contains(OVERLAY_TITLE),
        "a plain slash must not open the overlay"
    );
    Ok(())
}

#[test]
fn q_quits_the_example() -> io::Result<()> {
    let mut pty = Pty::spawn()?;
    assert!(pty.wait_for(BASE_HINT), "base screen never drew");
    pty.send(b"q")?;
    let status = pty.wait_exit()?;
    assert!(status.success(), "example exited with {status}");
    Ok(())
}
