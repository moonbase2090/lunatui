// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::PtyProcess;

/// Validated terminal dimensions shared by the child PTY and its screen model.
///
/// Each side is between 1 and 4096 cells, and the area is at most 1,048,576 cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtySize {
    cols: u16,
    rows: u16,
}

impl PtySize {
    /// Validates columns and rows before either the emulator or OS allocates a screen.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` for empty or excessive dimensions.
    pub fn new(cols: u16, rows: u16) -> io::Result<Self> {
        if cols == 0
            || rows == 0
            || cols > 4096
            || rows > 4096
            || u32::from(cols) * u32::from(rows) > 1_048_576
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "terminal dimensions exceed their limits",
            ));
        }
        Ok(Self { cols, rows })
    }

    /// Terminal columns.
    #[must_use]
    pub const fn cols(self) -> u16 {
        self.cols
    }

    /// Terminal rows.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows
    }
}
