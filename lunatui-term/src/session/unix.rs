// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::File;
use std::io::{self, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::panic;
use std::sync::Mutex;

use rustix::fd::AsFd;
use rustix::io::Errno;
use rustix::termios::isatty;
use rustix::termios::{OptionalActions, Termios, tcgetattr, tcsetattr};

const ENTER_ALT: &[u8] = b"\x1b[?1049h";
const LEAVE_ALT: &[u8] = b"\x1b[?1049l";
const SHOW_CURSOR: &[u8] = b"\x1b[?25h";
const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
const SYNC_BEGIN: &[u8] = b"\x1b[?2026h";
const SYNC_END: &[u8] = b"\x1b[?2026l";

/// Whether frame payloads are wrapped in synchronized-update sequences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncOutput {
    /// Emit `CSI ? 2026 h` / `l` around each frame.
    Enabled,
    /// Write frame bytes without sync wrapping.
    Disabled,
}

/// Options for [`Session::enter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionOptions {
    /// Synchronized output wrapping for [`Session::present_frame`].
    pub sync_output: SyncOutput,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            sync_output: SyncOutput::Enabled,
        }
    }
}

/// Errors from terminal session setup and presentation.
#[derive(Debug)]
pub enum SessionError {
    /// The writer is not connected to a terminal.
    NotATty,
    /// An underlying I/O or termios operation failed.
    Io(io::Error),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotATty => f.write_str("not a terminal"),
            Self::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::NotATty => None,
        }
    }
}

impl From<SessionError> for io::Error {
    fn from(err: SessionError) -> Self {
        match err {
            SessionError::NotATty => Self::new(io::ErrorKind::NotConnected, err.to_string()),
            SessionError::Io(inner) => inner,
        }
    }
}

struct RestoreState {
    tty: File,
    saved: Termios,
}

static ACTIVE_RESTORE: Mutex<Option<RestoreState>> = Mutex::new(None);

/// Owns raw mode and the alternate screen until dropped.
pub struct Session<W> {
    writer: W,
    tty: File,
    saved: Termios,
    sync_output: SyncOutput,
    frame_buf: Vec<u8>,
    active: bool,
}

impl<W: Write> Session<W> {
    /// Opens a fullscreen session on the controlling terminal.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::NotATty`] when stdout is not a terminal, or
    /// [`SessionError::Io`] when termios setup fails.
    pub fn enter(writer: W) -> Result<Self, SessionError> {
        Self::enter_with_options(writer, SessionOptions::default())
    }

    /// Opens a session with the given options.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::NotATty`] when stdout is not a terminal, or
    /// [`SessionError::Io`] when termios setup fails.
    pub fn enter_with_options(writer: W, options: SessionOptions) -> Result<Self, SessionError> {
        let tty = open_controlling_tty()?;
        let saved = tcgetattr(&tty).map_err(map_errno)?;
        let mut raw = saved.clone();
        raw.make_raw();
        tcsetattr(&tty, OptionalActions::Now, &raw).map_err(map_errno)?;
        install_panic_restore();
        let mut session = Self {
            writer,
            tty,
            saved,
            sync_output: options.sync_output,
            frame_buf: Vec::new(),
            active: true,
        };
        session.write_startup()?;
        register_restore(&session.tty, &session.saved)?;
        Ok(session)
    }

    /// Writes one frame with optional sync wrapping in a single `write_all`.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Io`] when the frame write fails.
    pub fn present_frame(&mut self, payload: &[u8]) -> Result<(), SessionError> {
        compose_frame(&mut self.frame_buf, payload, self.sync_output);
        self.writer
            .write_all(&self.frame_buf)
            .map_err(SessionError::Io)
    }
}

impl<W> Session<W> {
    fn write_startup(&mut self) -> Result<(), SessionError> {
        write_tty(&self.tty, ENTER_ALT)?;
        write_tty(&self.tty, HIDE_CURSOR)?;
        Ok(())
    }

    fn restore(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        let _ = write_tty(&self.tty, SHOW_CURSOR);
        let _ = write_tty(&self.tty, LEAVE_ALT);
        let _ = tcsetattr(&self.tty, OptionalActions::Now, &self.saved);
        clear_restore();
    }
}

impl<W> Drop for Session<W> {
    fn drop(&mut self) {
        self.restore();
    }
}

fn write_tty(tty: &File, bytes: &[u8]) -> Result<(), SessionError> {
    rustix::io::write(tty, bytes).map_err(map_errno)?;
    Ok(())
}

fn open_controlling_tty() -> Result<File, SessionError> {
    let stdout = std::io::stdout();
    let stdout_fd = stdout.as_raw_fd();
    // SAFETY: `stdout_fd` comes from `AsRawFd` on the process stdout handle.
    unsafe {
        if isatty(rustix::fd::BorrowedFd::borrow_raw(stdout_fd)) {
            let dup = libc::dup(stdout_fd);
            if dup < 0 {
                return Err(SessionError::Io(io::Error::last_os_error()));
            }
            return Ok(File::from_raw_fd(dup));
        }
    }
    let tty = File::open("/dev/tty").map_err(SessionError::Io)?;
    if !isatty(&tty) {
        return Err(SessionError::NotATty);
    }
    Ok(tty)
}

fn register_restore(tty: &File, saved: &Termios) -> Result<(), SessionError> {
    let state = RestoreState {
        tty: tty.try_clone().map_err(SessionError::Io)?,
        saved: saved.clone(),
    };
    with_restore_slot(|slot| *slot = Some(state));
    Ok(())
}

fn clear_restore() {
    with_restore_slot(|slot| *slot = None);
}

fn with_restore_slot<F>(f: F)
where
    F: FnOnce(&mut Option<RestoreState>),
{
    match ACTIVE_RESTORE.lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poison) => f(&mut poison.into_inner()),
    }
}

fn install_panic_restore() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let prior = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            panic_restore();
            prior(info);
        }));
    });
}

fn panic_restore() {
    with_restore_slot(|slot| {
        if let Some(state) = slot.take() {
            let _ = write_restore(&state.tty, &state.saved);
        }
    });
}

fn write_restore(tty: &File, saved: &Termios) -> Result<(), SessionError> {
    let fd = tty.as_fd();
    let _ = rustix::io::write(fd, SHOW_CURSOR);
    let _ = rustix::io::write(fd, LEAVE_ALT);
    tcsetattr(tty, OptionalActions::Now, saved).map_err(map_errno)?;
    Ok(())
}

fn map_errno(err: Errno) -> SessionError {
    SessionError::Io(err.into())
}

pub(crate) fn compose_frame(buffer: &mut Vec<u8>, payload: &[u8], sync: SyncOutput) {
    buffer.clear();
    if sync == SyncOutput::Enabled {
        buffer.extend_from_slice(SYNC_BEGIN);
    }
    buffer.extend_from_slice(payload);
    if sync == SyncOutput::Enabled {
        buffer.extend_from_slice(SYNC_END);
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use lunatui_bench::CountingWriter;
    use rustix::termios::isatty;

    use super::{SyncOutput, compose_frame};

    #[test]
    fn compose_frame_wraps_sync_when_enabled() {
        let mut buf = Vec::new();
        compose_frame(&mut buf, b"XY", SyncOutput::Enabled);
        assert!(buf.starts_with(b"\x1b[?2026h"));
        assert!(buf.ends_with(b"\x1b[?2026l"));
        assert!(buf.windows(2).any(|pair| pair == b"XY"));
    }

    #[test]
    fn compose_frame_omits_sync_when_disabled() {
        let mut buf = Vec::new();
        compose_frame(&mut buf, b"XY", SyncOutput::Disabled);
        assert_eq!(buf, b"XY");
    }

    #[test]
    fn present_frame_uses_one_write() {
        use super::{Session, SessionOptions};

        if !matches!(File::open("/dev/tty"), Ok(file) if isatty(&file)) {
            return;
        }
        let mut writer = CountingWriter::new(Vec::new());
        {
            let Ok(mut session) =
                Session::enter_with_options(&mut writer, SessionOptions::default())
            else {
                return;
            };
            if session.present_frame(b"frame").is_err() {
                return;
            }
        }
        let stats = writer.stats();
        assert_eq!(stats.writes, 1);
        let bytes = writer.into_inner();
        assert!(bytes.windows(8).any(|w| w == b"\x1b[?2026h"));
    }
}
