// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use lunatui_core::{ClusterTable, Grid};

/// Whether the encoder writes color sequences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// Skip fg/bg SGR (PRD ratatui baselines with `NO_COLOR`).
    NoColor,
    /// Emit truecolor and indexed colors.
    TrueColor,
}

/// Reusable encoder buffer with pen-state tracking (PRD §2.6).
#[derive(Clone, Debug, Default)]
pub struct Encoder {
    buffer: Vec<u8>,
}

impl Encoder {
    /// An encoder with an empty output buffer and a reset pen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes `next` against `previous` into the internal buffer and returns the bytes.
    pub fn encode_diff(
        &mut self,
        _previous: &Grid,
        _next: &Grid,
        _clusters: &ClusterTable,
        _colors: ColorMode,
    ) -> &[u8] {
        self.buffer.clear();
        &self.buffer
    }
}
