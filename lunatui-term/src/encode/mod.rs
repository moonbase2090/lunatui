// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod cursor;
mod sgr;

use lunatui_core::{
    Cell, CellWidth, ChangedRun, ClusterTable, Color, DecodedGlyph, Grid, diff_runs_into,
};

use cursor::write_cup;
pub use cursor::write_cursor_state;
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
    runs: Vec<ChangedRun>,
    pen: Pen,
    cursor_col: u16,
    cursor_row: u16,
    cursor_placed: bool,
    token_palette: Vec<Color>,
}

impl Encoder {
    /// An encoder with an empty output buffer and a reset pen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Token colors resolve through this palette during encoding (PRD §2.11).
    pub fn set_token_palette(&mut self, palette: Vec<Color>) {
        self.token_palette = palette;
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
        let mut one_wide_key: Option<u128> = None;
        diff_runs_into(previous, next, &mut self.runs);
        let runs = std::mem::take(&mut self.runs);
        for run in &runs {
            let mut col = run.col;
            let words = next.row_bits(run.row);
            while col < run.col + run.width {
                let Some(&bits) = words.get(usize::from(col)) else {
                    col += 1;
                    continue;
                };
                let key = bits >> 32;
                let ascii = u8::try_from(bits & u128::from(u32::MAX))
                    .ok()
                    .filter(u8::is_ascii);
                if let Some(byte) = ascii
                    && Some(key) == one_wide_key
                    && col == self.cursor_col
                    && run.row == self.cursor_row
                {
                    self.buffer.push(byte);
                    self.cursor_col += 1;
                    col += 1;
                    continue;
                }
                let Some(cell) = Cell::from_bits(bits) else {
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
                write_sgr_delta(
                    &mut self.buffer,
                    &mut self.pen,
                    style,
                    color,
                    &self.token_palette,
                );
                self.write_glyph(cell.glyph(), clusters);
                let advance = match cell.meta().width {
                    CellWidth::Two => 2,
                    CellWidth::One | CellWidth::Continuation => 1,
                };
                self.cursor_col += advance;
                col += advance;
                one_wide_key = (advance == 1).then_some(key);
            }
        }
        self.runs = runs;
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
