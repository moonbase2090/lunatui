// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod cursor;
mod sgr;

use lunatui_core::{CellWidth, ClusterTable, DecodedGlyph, Grid, diff_runs};

use cursor::write_cup;
use sgr::{Pen, write_sgr_delta};

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
    pen: Pen,
    cursor_col: u16,
    cursor_row: u16,
    cursor_placed: bool,
}

impl Encoder {
    /// An encoder with an empty output buffer and a reset pen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes `next` against `previous` into the internal buffer and returns the bytes.
    ///
    /// The first placed cell is an absolute cursor move. A later frame cannot assume the
    /// terminal cursor stayed at the origin. The output ends with the same reset trailer
    /// ratatui's crossterm backend emits after a draw.
    pub fn encode_diff(
        &mut self,
        previous: &Grid,
        next: &Grid,
        clusters: &ClusterTable,
        colors: ColorMode,
    ) -> &[u8] {
        self.buffer.clear();
        self.pen = Pen::default();
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.cursor_placed = false;
        let color = colors == ColorMode::TrueColor;
        for run in diff_runs(previous, next) {
            let mut col = run.col;
            while col < run.col + run.width {
                let Some(cell) = next.get(col, run.row) else {
                    col += 1;
                    continue;
                };
                if cell.meta().width == CellWidth::Continuation {
                    col += 1;
                    continue;
                }
                if !self.cursor_placed || col != self.cursor_col || run.row != self.cursor_row {
                    write_cup(&mut self.buffer, col, run.row);
                    self.cursor_placed = true;
                    self.cursor_col = col;
                    self.cursor_row = run.row;
                }
                let style = cell.style();
                write_sgr_delta(&mut self.buffer, &mut self.pen, style, color);
                self.write_glyph(cell.glyph(), clusters);
                let advance = match cell.meta().width {
                    CellWidth::Two => 2,
                    CellWidth::One | CellWidth::Continuation => 1,
                };
                self.cursor_col += advance;
                col += advance;
            }
        }
        self.buffer.extend_from_slice(b"\x1b[m\x1b[m\x1b[m\x1b[0m");
        &self.buffer
    }

    fn write_glyph(&mut self, glyph: lunatui_core::GlyphSlot, clusters: &ClusterTable) {
        match glyph.decode() {
            DecodedGlyph::Inline(scalar) => {
                let mut chunk = [0u8; 4];
                let text = scalar.encode_utf8(&mut chunk);
                self.buffer.extend_from_slice(text.as_bytes());
            }
            DecodedGlyph::Cluster(id) => {
                if let Some(text) = clusters.cluster_text(id) {
                    self.buffer.extend_from_slice(text.as_bytes());
                }
            }
            DecodedGlyph::Continuation => {}
        }
    }
}
