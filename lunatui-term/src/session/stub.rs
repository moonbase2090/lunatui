// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
