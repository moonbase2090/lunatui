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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ConsoleMarks {
    alt: bool,
    hidden_cursor: bool,
    mouse: bool,
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
        if let Err(error) = enable_raw_mode() {
            return Err(SessionError::Io(error));
        }
        if let Err(error) = apply_console_start(&mut writer, options.mouse_tracking) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self {
            writer,
            sync_output: options.sync_output,
            mouse_tracking: options.mouse_tracking,
            frame_buf: Vec::new(),
            active: true,
        })
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
    fn restore(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        let marks = ConsoleMarks {
            alt: true,
            hidden_cursor: true,
            mouse: self.mouse_tracking == MouseTracking::Sgr,
        };
        undo_console_start(&mut self.writer, marks);
        let _ = disable_raw_mode();
    }
}

impl<W: Write> Drop for Session<W> {
    fn drop(&mut self) {
        self.restore();
    }
}

fn apply_console_start<W: Write>(
    writer: &mut W,
    mouse_tracking: MouseTracking,
) -> Result<ConsoleMarks, SessionError> {
    let mut marks = ConsoleMarks::default();
    if writer.write_all(ENTER_ALT).is_err() {
        return Err(SessionError::Io(io::Error::other("enter alternate screen")));
    }
    marks.alt = true;
    if writer.write_all(HIDE_CURSOR).is_err() {
        undo_console_start(writer, marks);
        return Err(SessionError::Io(io::Error::other("hide cursor")));
    }
    marks.hidden_cursor = true;
    if mouse_tracking == MouseTracking::Sgr {
        if writer.write_all(MOUSE_ENABLE).is_err() {
            undo_console_start(writer, marks);
            return Err(SessionError::Io(io::Error::other("enable mouse")));
        }
        marks.mouse = true;
    }
    Ok(marks)
}

fn undo_console_start<W: Write>(writer: &mut W, marks: ConsoleMarks) {
    if marks.mouse {
        let _ = writer.write_all(MOUSE_DISABLE);
    }
    if marks.hidden_cursor {
        let _ = writer.write_all(SHOW_CURSOR);
    }
    if marks.alt {
        let _ = writer.write_all(LEAVE_ALT);
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
    use super::{
        MOUSE_DISABLE, MOUSE_ENABLE, MouseTracking, SyncOutput, apply_console_start, compose_frame,
    };
    use std::io::{self, Write};

    struct FailOnceAfterBytes {
        buffer: Vec<u8>,
        fail_after_bytes: usize,
        failed_once: bool,
    }

    impl Write for FailOnceAfterBytes {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if !self.failed_once && self.buffer.len() >= self.fail_after_bytes {
                self.failed_once = true;
                return Err(io::Error::other("write failed"));
            }
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn compose_frame_wraps_sync_when_enabled() {
        let mut buf = Vec::new();
        compose_frame(&mut buf, b"XY", SyncOutput::Enabled);
        assert!(buf.starts_with(b"\x1b[?2026h"));
        assert!(buf.ends_with(b"\x1b[?2026l"));
        assert!(buf.windows(2).any(|pair| pair == b"XY"));
    }

    #[test]
    fn compose_frame_writes_payload_only_when_sync_is_disabled() {
        let mut buf = Vec::new();
        compose_frame(&mut buf, b"XY", SyncOutput::Disabled);
        assert_eq!(buf, b"XY");
    }

    #[test]
    fn setup_writes_sgr_mouse_modes_for_vt() -> Result<(), super::SessionError> {
        let mut writer = Vec::new();
        let marks = apply_console_start(&mut writer, MouseTracking::Sgr)?;
        assert!(marks.alt && marks.hidden_cursor && marks.mouse);
        assert!(
            writer
                .windows(MOUSE_ENABLE.len())
                .any(|window| window == MOUSE_ENABLE)
        );
        assert!(
            !writer
                .windows(MOUSE_DISABLE.len())
                .any(|window| window == MOUSE_DISABLE)
        );
        Ok(())
    }

    #[test]
    fn setup_skips_mouse_modes_when_tracking_is_disabled() -> Result<(), super::SessionError> {
        let mut writer = Vec::new();
        let marks = apply_console_start(&mut writer, MouseTracking::Disabled)?;
        assert!(marks.alt && marks.hidden_cursor);
        assert!(!marks.mouse);
        assert!(
            !writer
                .windows(MOUSE_ENABLE.len())
                .any(|window| window == MOUSE_ENABLE)
        );
        Ok(())
    }

    #[test]
    fn setup_rolls_back_after_hide_cursor_write_fails() {
        let mut writer = FailOnceAfterBytes {
            buffer: Vec::new(),
            fail_after_bytes: ENTER_ALT.len(),
            failed_once: false,
        };
        let err = apply_console_start(&mut writer, MouseTracking::Disabled);
        assert!(matches!(err, Err(super::SessionError::Io(_))));
        assert!(writer.buffer.ends_with(LEAVE_ALT));
    }

    #[test]
    fn setup_rolls_back_after_mouse_enable_write_fails() {
        let mut writer = FailOnceAfterBytes {
            buffer: Vec::new(),
            fail_after_bytes: ENTER_ALT.len() + HIDE_CURSOR.len(),
            failed_once: false,
        };
        let err = apply_console_start(&mut writer, MouseTracking::Sgr);
        assert!(matches!(err, Err(super::SessionError::Io(_))));
        assert!(writer.buffer.ends_with(LEAVE_ALT));
    }

    const ENTER_ALT: &[u8] = b"\x1b[?1049h";
    const LEAVE_ALT: &[u8] = b"\x1b[?1049l";
    const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
}
