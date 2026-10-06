// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// The Graphite-like dashboard: rail, tabs, checks table, wrapped log, gauge and status bar.
pub mod dashboard;
/// The truecolor gradient full frame used to count bytes and `write()` calls.
pub mod gradient;
/// The 5,000-line log rendered by a wrapped or unwrapped `Paragraph`.
pub mod log;
/// The 500-row checks table.
pub mod table;

use std::io;

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::crossterm::style::Colored;

/// Whether ratatui's crossterm backend writes color sequences.
///
/// The measured ratatui-perf baselines (PRD §1.3) ran with `NO_COLOR` set, so they
/// contain no color sequences. [`ColorOutput::Disabled`] reproduces them exactly, and
/// [`ColorOutput::Enabled`] is what a color terminal receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorOutput {
    /// Colors are written, as for a color terminal.
    Enabled,
    /// Colors are left out, as with `NO_COLOR` set.
    Disabled,
}

/// Sets color output for every crossterm encode in this process, overriding `NO_COLOR`.
///
/// The setting is process-wide, so every test in one test binary must use the same value.
pub fn set_color_output(output: ColorOutput) {
    Colored::set_ansi_color_disabled(output == ColorOutput::Disabled);
}

/// Output of encoding the difference between two frames with ratatui's crossterm backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoded {
    /// Cells that differ between the frames.
    pub changed_cells: usize,
    /// Bytes written by the encoder.
    pub bytes: usize,
    /// Control sequence introducers (`ESC [`) in the output.
    pub csi_sequences: usize,
}

/// Encodes the cells that changed from `previous` to `next` into `out`, which is cleared first.
///
/// # Errors
///
/// Returns any error from the crossterm backend.
pub fn encode(previous: &Buffer, next: &Buffer, out: &mut Vec<u8>) -> io::Result<Encoded> {
    out.clear();
    CrosstermBackend::new(&mut *out).draw(previous.diff_iter(next))?;
    Ok(Encoded {
        changed_cells: previous.diff_iter(next).count(),
        bytes: out.len(),
        csi_sequences: out.windows(2).filter(|pair| *pair == b"\x1b[").count(),
    })
}
