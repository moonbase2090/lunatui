// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::time::Instant;

use rustix::io::Errno;
use rustix::termios::OptionalActions;
use rustix::termios::{Termios, isatty, tcgetattr, tcsetattr};

use super::parse::{from_replies, has_device_attributes};
use super::{PROBE_BUDGET, ProbeEnv, ProbeReply, QUERIES};

/// Writes [`QUERIES`] to `fd`, reads until device attributes or [`PROBE_BUDGET`], and enables mode 2027 when it is reset.
///
/// On a terminal, local echo is cleared for the probe and restored before this returns.
/// A timeout is not an error: [`ProbeReply::saw_device_attributes`] is false.
///
/// # Errors
///
/// Returns an error when the write, the read, or restoring the terminal mode fails.
pub fn probe_fd(fd: BorrowedFd<'_>, env: &ProbeEnv) -> io::Result<ProbeReply> {
    let saved = prepare(fd)?;
    let result = exchange(fd, env);
    match (result, restore(fd, saved.as_ref())) {
        (Ok(reply), Ok(())) => Ok(reply),
        (Err(err), _) | (Ok(_), Err(err)) => Err(err),
    }
}

fn prepare(fd: BorrowedFd<'_>) -> io::Result<Option<Termios>> {
    if !isatty(fd) {
        return Ok(None);
    }
    let saved = tcgetattr(fd)?;
    let mut raw = saved.clone();
    raw.make_raw();
    tcsetattr(fd, OptionalActions::Now, &raw)?;
    Ok(Some(saved))
}

fn restore(fd: BorrowedFd<'_>, saved: Option<&Termios>) -> io::Result<()> {
    let Some(saved) = saved else {
        return Ok(());
    };
    tcsetattr(fd, OptionalActions::Now, saved)?;
    Ok(())
}

fn exchange(fd: BorrowedFd<'_>, env: &ProbeEnv) -> io::Result<ProbeReply> {
    write_all(fd, QUERIES)?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 256];
    let start = Instant::now();
    loop {
        if has_device_attributes(&buf) {
            return finish(fd, env, &buf);
        }
        if buf.len() >= 65_536 {
            return Ok(from_replies(env, &buf));
        }
        let left = PROBE_BUDGET.saturating_sub(start.elapsed());
        if left.is_zero() {
            return Ok(from_replies(env, &buf));
        }
        let millis = i32::try_from(left.as_millis()).unwrap_or(i32::MAX);
        match wait_read(fd, millis)? {
            Wait::Timeout => return Ok(from_replies(env, &buf)),
            Wait::Ready => match read_some(fd, &mut tmp)? {
                Read::Eof => return Ok(from_replies(env, &buf)),
                Read::Bytes(n) => buf.extend_from_slice(&tmp[..n]),
                Read::Again => {}
            },
            Wait::Interrupted => {}
        }
    }
}

fn finish(fd: BorrowedFd<'_>, env: &ProbeEnv, buf: &[u8]) -> io::Result<ProbeReply> {
    let reply = from_replies(env, buf);
    if let Some(enable) = reply.caps.grapheme_enable() {
        write_all(fd, enable)?;
    }
    Ok(reply)
}

fn write_all(fd: BorrowedFd<'_>, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        match rustix::io::write(fd, bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "capability probe write returned no bytes",
                ));
            }
            Ok(n) => bytes = &bytes[n..],
            Err(Errno::INTR) => {}
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

enum Read {
    Bytes(usize),
    Again,
    Eof,
}

fn read_some(fd: BorrowedFd<'_>, buf: &mut [u8]) -> io::Result<Read> {
    match rustix::io::read(fd, buf) {
        Ok(0) => Ok(Read::Eof),
        Ok(n) => Ok(Read::Bytes(n)),
        Err(Errno::INTR | Errno::AGAIN) => Ok(Read::Again),
        Err(err) => Err(err.into()),
    }
}

enum Wait {
    Ready,
    Timeout,
    Interrupted,
}

fn wait_read(fd: BorrowedFd<'_>, timeout_ms: i32) -> io::Result<Wait> {
    let mut poll_fd = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = {
        // SAFETY: `poll_fd` refers to an open descriptor and lives for this call. `nfds` is 1.
        unsafe { libc::poll(&raw mut poll_fd, 1, timeout_ms) }
    };
    if ready < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(Wait::Interrupted);
        }
        return Err(err);
    }
    if ready == 0 {
        Ok(Wait::Timeout)
    } else {
        Ok(Wait::Ready)
    }
}
