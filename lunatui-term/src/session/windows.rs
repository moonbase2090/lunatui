// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, IsTerminal, Write};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

const ENTER_ALT: &[u8] = b"\x1b[?1049h";
const LEAVE_ALT: &[u8] = b"\x1b[?1049l";
const SHOW_CURSOR: &[u8] = b"\x1b[?25h";
const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
const MOUSE_ENABLE: &[u8] = b"\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h";
const MOUSE_DISABLE: &[u8] = b"\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l";
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

/// Pointer reporting for Windows Terminal VT mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MouseTracking {
    /// No mouse events.
    #[default]
    Disabled,
    /// SGR mouse modes 1000, 1002, 1003, and 1006.
    Sgr,
}

/// Options for [`Session::enter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionOptions {
    /// Synchronized output wrapping for [`Session::present_frame`].
    pub sync_output: SyncOutput,
    /// Pointer reporting for interactive apps.
    pub mouse_tracking: MouseTracking,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            sync_output: SyncOutput::Enabled,
            mouse_tracking: MouseTracking::Disabled,
        }
    }
}

/// Errors from terminal session setup and presentation.
#[derive(Debug)]
pub enum SessionError {
    /// The writer is not connected to a terminal.
    NotATty,
    /// An underlying I/O or console operation failed.
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

/// Owns raw mode and the alternate screen until dropped.
pub struct Session<W: Write> {
    writer: W,
    sync_output: SyncOutput,
    mouse_tracking: MouseTracking,
    frame_buf: Vec<u8>,
    active: bool,
}

impl<W: Write> Session<W> {
    /// Opens a fullscreen session on the Windows console.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::NotATty`] when stdout is not a terminal, or
    /// [`SessionError::Io`] when console setup fails.
    pub fn enter(writer: W) -> Result<Self, SessionError> {
        Self::enter_with_options(writer, SessionOptions::default())
    }

    /// Opens a session with the given options.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::NotATty`] when stdout is not a terminal, or
    /// [`SessionError::Io`] when console setup fails.
    pub fn enter_with_options(
        mut writer: W,
        options: SessionOptions,
    ) -> Result<Self, SessionError> {
        if !std::io::stdout().is_terminal() {
            return Err(SessionError::NotATty);
        }
        enable_raw_mode().map_err(SessionError::Io)?;
        writer.write_all(ENTER_ALT).map_err(SessionError::Io)?;
        writer.write_all(HIDE_CURSOR).map_err(SessionError::Io)?;
        let mut session = Self {
            writer,
            sync_output: options.sync_output,
            mouse_tracking: options.mouse_tracking,
            frame_buf: Vec::new(),
            active: true,
        };
        session.write_startup()?;
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

impl<W: Write> Session<W> {
    fn write_startup(&mut self) -> Result<(), SessionError> {
        if self.mouse_tracking == MouseTracking::Sgr {
            self.writer
                .write_all(MOUSE_ENABLE)
                .map_err(SessionError::Io)?;
        }
        Ok(())
    }

    fn restore(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        if self.mouse_tracking == MouseTracking::Sgr {
            let _ = self.writer.write_all(MOUSE_DISABLE);
        }
        let _ = self.writer.write_all(SHOW_CURSOR);
        let _ = self.writer.write_all(LEAVE_ALT);
        let _ = disable_raw_mode();
    }
}

impl<W: Write> Drop for Session<W> {
    fn drop(&mut self) {
        self.restore();
    }
}

fn compose_frame(buffer: &mut Vec<u8>, payload: &[u8], sync: SyncOutput) {
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
    use super::{SyncOutput, compose_frame};

    #[test]
    fn compose_frame_wraps_sync_when_enabled() {
        let mut buf = Vec::new();
        compose_frame(&mut buf, b"XY", SyncOutput::Enabled);
        assert!(buf.starts_with(b"\x1b[?2026h"));
        assert!(buf.ends_with(b"\x1b[?2026l"));
        assert!(buf.windows(2).any(|pair| pair == b"XY"));
    }
}
