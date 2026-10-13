// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg(unix)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_term::{PrivateMode, ProbeEnv, ProbeReply, QUERIES, RecordedTerminal, probe_fd};

fn env_256() -> ProbeEnv {
    ProbeEnv {
        term: String::from("xterm-256color"),
        ..ProbeEnv::default()
    }
}

#[test]
fn probe_waits_out_the_budget_when_no_reply_arrives() -> io::Result<()> {
    let (app, peer) = UnixStream::pair()?;
    let started = Instant::now();
    let reply = probe_fd(app.as_fd(), &env_256())?;
    let elapsed = started.elapsed();
    drop(peer);
    assert!(!reply.saw_device_attributes);
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Unreported);
    assert_eq!(
        reply.caps.modes.synchronized_output,
        PrivateMode::Unreported
    );
    assert!(
        elapsed >= Duration::from_millis(150),
        "budget elapsed too soon: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(800),
        "budget elapsed too late: {elapsed:?}"
    );
    Ok(())
}

#[test]
fn probe_reads_a_reply_that_arrives_inside_the_budget() -> io::Result<()> {
    let (app, mut peer) = UnixStream::pair()?;
    let fixture = RecordedTerminal::Xterm.replies().to_vec();
    let writer = thread::spawn(move || -> io::Result<()> {
        thread::sleep(Duration::from_millis(50));
        peer.write_all(&fixture)?;
        Ok(())
    });
    let started = Instant::now();
    let reply = probe_fd(app.as_fd(), &env_256())?;
    let elapsed = started.elapsed();
    writer
        .join()
        .map_err(|_| io::Error::other("reply thread panicked"))??;
    assert!(reply.saw_device_attributes);
    assert_eq!(reply.caps.identity.program, "xterm");
    assert_eq!(reply.caps.identity.version, "384");
    assert!(
        elapsed < Duration::from_millis(800),
        "reply probe took {elapsed:?}"
    );
    Ok(())
}

#[test]
fn probe_on_a_pty_enables_mode_2027_for_the_kitty_reply() -> io::Result<()> {
    let (reply, output) = exchange(RecordedTerminal::Kitty.replies())?;
    assert!(reply.saw_device_attributes);
    assert_eq!(reply.caps.identity.program, "kitty");
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Reset);
    assert_eq!(
        reply.caps.grapheme_enable(),
        Some(b"\x1b[?2027h".as_slice())
    );
    assert!(contains(&output, b"\x1b[?2026$p"));
    assert!(contains(&output, b"\x1b[?2027$p"));
    assert!(contains(&output, b"\x1b[?2027h"));
    assert!(output.starts_with(QUERIES));
    Ok(())
}

#[test]
fn probe_on_a_pty_does_not_enable_mode_2027_when_it_is_already_set() -> io::Result<()> {
    let (reply, output) = exchange(RecordedTerminal::WezTerm.replies())?;
    assert_eq!(reply.caps.identity.program, "WezTerm");
    assert_eq!(reply.caps.modes.grapheme_clusters, PrivateMode::Set);
    assert_eq!(reply.caps.grapheme_enable(), None);
    assert!(output.starts_with(QUERIES));
    assert!(!contains(&output, b"\x1b[?2027h"));
    Ok(())
}

fn exchange(fixture: &[u8]) -> io::Result<(ProbeReply, Vec<u8>)> {
    let (master, slave) = open_pty()?;
    set_nonblocking(&master)?;
    let fixture = fixture.to_vec();
    let reader = thread::spawn(move || -> io::Result<Vec<u8>> {
        let mut master = master;
        let mut got = Vec::new();
        let mut sent = false;
        let mut sent_at = None;
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(2) {
            let mut buf = [0u8; 256];
            match master.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => got.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    if !sent && got.starts_with(QUERIES) {
                        master.write_all(&fixture)?;
                        sent = true;
                        sent_at = Some(Instant::now());
                    }
                    if sent_at.is_some_and(|at| at.elapsed() > Duration::from_millis(300)) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) if err.raw_os_error() == Some(libc::EIO) => break,
                Err(err) => return Err(err),
            }
        }
        Ok(got)
    });
    let result = probe_fd(slave.as_fd(), &env_256());
    drop(slave);
    let output = reader
        .join()
        .map_err(|_| io::Error::other("pty thread panicked"))??;
    Ok((result?, output))
}

fn open_pty() -> io::Result<(File, File)> {
    let mut master = 0;
    let mut slave = 0;
    let rc = {
        // SAFETY: `openpty` writes open fds to `master` and `slave` when it returns zero.
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
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    let master = {
        // SAFETY: `master` was opened by `openpty` and is not owned elsewhere.
        unsafe { File::from_raw_fd(master) }
    };
    let slave = {
        // SAFETY: `slave` was opened by `openpty` and is not owned elsewhere.
        unsafe { File::from_raw_fd(slave) }
    };
    Ok((master, slave))
}

fn set_nonblocking(file: &File) -> io::Result<()> {
    let fd = file.as_fd().as_raw_fd();
    let flags = {
        // SAFETY: `fd` is an open pty master. `F_GETFL` reads its status flags.
        unsafe { libc::fcntl(fd, libc::F_GETFL) }
    };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    let rc = {
        // SAFETY: `fd` is the same open pty master. `F_SETFL` sets the non-blocking flag.
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) }
    };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
