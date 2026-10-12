// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt;
use std::io;
use std::io::Write;

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
/// Stub mirror of Unix mouse tracking (always disabled on non-Unix).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MouseTracking {
    /// No mouse events.
    #[default]
    Disabled,
    /// SGR mouse modes (Unix only).
    Sgr,
}

pub struct SessionOptions {
    /// Synchronized output wrapping for [`Session::present_frame`].
    pub sync_output: SyncOutput,
    /// Pointer reporting (unsupported on non-Unix stubs).
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
    /// Session APIs are only implemented on Unix in PR 10.
    Unsupported,
    /// The writer is not connected to a terminal.
    NotATty,
    /// An underlying I/O or termios operation failed.
    Io(io::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("terminal session is only supported on Unix"),
            Self::NotATty => f.write_str("not a terminal"),
            Self::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Unsupported | Self::NotATty => None,
        }
    }
}

impl From<SessionError> for io::Error {
    fn from(err: SessionError) -> Self {
        match err {
            SessionError::Io(inner) => inner,
            SessionError::Unsupported | SessionError::NotATty => {
                Self::new(io::ErrorKind::Unsupported, err.to_string())
            }
        }
    }
}

/// Owns raw mode and the alternate screen until dropped.
pub struct Session<W> {
    writer: W,
}

impl<W: Write> Session<W> {
    /// Opens a fullscreen session (Unix only in PR 10).
    ///
    /// # Errors
    ///
    /// Always returns [`SessionError::Unsupported`] on non-Unix targets.
    pub fn enter(_writer: W) -> Result<Self, SessionError> {
        Err(SessionError::Unsupported)
    }

    /// Opens a session with the given options.
    ///
    /// # Errors
    ///
    /// Always returns [`SessionError::Unsupported`] on non-Unix targets.
    pub fn enter_with_options(_writer: W, _options: SessionOptions) -> Result<Self, SessionError> {
        Err(SessionError::Unsupported)
    }

    /// Writes one frame with optional sync wrapping in a single `write_all`.
    ///
    /// # Errors
    ///
    /// Always returns [`SessionError::Unsupported`] on non-Unix targets.
    pub fn present_frame(&mut self, _payload: &[u8]) -> Result<(), SessionError> {
        Err(SessionError::Unsupported)
    }
}
