// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io;
use std::thread;
use std::time::Duration;

/// How often [`watch_size`] re-reads the terminal size.
pub const SIZE_POLL: Duration = Duration::from_millis(100);

/// Reads the terminal size as `(columns, rows)`, or `None` when no terminal is attached or the size is zero.
#[must_use]
pub fn terminal_size() -> Option<(u16, u16)> {
    let (columns, rows) = platform_size()?;
    (columns > 0 && rows > 0).then_some((columns, rows))
}

#[cfg(unix)]
fn platform_size() -> Option<(u16, u16)> {
    let size = rustix::termios::tcgetwinsize(std::io::stdout())
        .or_else(|_| rustix::termios::tcgetwinsize(std::io::stderr()))
        .or_else(|_| rustix::termios::tcgetwinsize(std::io::stdin()))
        .ok()?;
    Some((size.ws_col, size.ws_row))
}

#[cfg(windows)]
fn platform_size() -> Option<(u16, u16)> {
    crossterm::terminal::size().ok()
}

#[cfg(not(any(unix, windows)))]
fn platform_size() -> Option<(u16, u16)> {
    None
}

/// Calls `notify(columns, rows)` from a background thread once at start and again whenever `read` reports a different size.
///
/// The thread ends when `notify` returns `false` or `done` returns `true`, which is checked on every poll.
///
/// # Errors
///
/// Returns an error if the thread cannot be started.
pub fn watch_size<R, N, D>(mut read: R, poll: Duration, mut notify: N, done: D) -> io::Result<()>
where
    R: FnMut() -> Option<(u16, u16)> + Send + 'static,
    D: Fn() -> bool + Send + 'static,
    N: FnMut(u16, u16) -> bool + Send + 'static,
{
    thread::Builder::new()
        .name(String::from("lunatui-size"))
        .spawn(move || {
            let mut last = None;
            while !done() {
                if let Some(size) = read()
                    && last != Some(size)
                {
                    last = Some(size);
                    if !notify(size.0, size.1) {
                        return;
                    }
                }
                thread::sleep(poll);
            }
        })
        .map(drop)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn reports_start_size_then_each_change_once() -> Result<(), Box<dyn std::error::Error>> {
        let sizes = Arc::new(Mutex::new((80, 24)));
        let reader = Arc::clone(&sizes);
        let (sender, receiver) = mpsc::channel();
        watch_size(
            move || reader.lock().ok().map(|size| *size),
            Duration::from_millis(1),
            move |columns, rows| sender.send((columns, rows)).is_ok(),
            || false,
        )?;
        assert_eq!(receiver.recv_timeout(Duration::from_secs(5))?, (80, 24));
        *sizes.lock().map_err(|_| "poisoned")? = (120, 40);
        assert_eq!(receiver.recv_timeout(Duration::from_secs(5))?, (120, 40));
        assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
        Ok(())
    }

    #[test]
    fn stops_when_the_listener_is_gone() -> Result<(), Box<dyn std::error::Error>> {
        let (done, finished) = mpsc::channel();
        watch_size(
            || Some((1, 1)),
            Duration::from_millis(1),
            move |_, _| {
                let _ = done.send(());
                false
            },
            || false,
        )?;
        finished.recv_timeout(Duration::from_secs(5))?;
        assert!(finished.recv_timeout(Duration::from_millis(50)).is_err());
        Ok(())
    }

    #[test]
    fn stops_without_a_change_once_done() -> Result<(), Box<dyn std::error::Error>> {
        let (reads, counted) = mpsc::channel();
        let over = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&over);
        watch_size(
            move || {
                let _ = reads.send(());
                Some((1, 1))
            },
            Duration::from_millis(1),
            |_, _| true,
            move || flag.load(Ordering::SeqCst),
        )?;
        counted.recv_timeout(Duration::from_secs(5))?;
        over.store(true, Ordering::SeqCst);
        while counted.recv_timeout(Duration::from_millis(200)).is_ok() {}
        Ok(())
    }
}
