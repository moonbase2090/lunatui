// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::vec::Vec;

use lunatui_core::cluster_display_width;
use unicode_segmentation::UnicodeSegmentation;

mod cache;
mod index;

pub use cache::WrapCache;
pub use index::RowIndex;

/// Whether leading whitespace is trimmed when a wrapped line continues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Wrap {
    /// Trims leading whitespace on continuation rows (ratatui `Wrap::trim`).
    pub trim: bool,
}

/// Builds a [`RowIndex`] by wrapping `text` to `line_width` columns.
#[must_use]
pub fn wrap_rows(text: &str, line_width: u16, wrap: Wrap) -> RowIndex {
    if line_width == 0 {
        return RowIndex::default();
    }
    let mut row_starts = Vec::new();
    if text.is_empty() {
        row_starts.push(0);
        return RowIndex { row_starts };
    }
    let mut offset = 0usize;
    while offset <= text.len() {
        if offset == text.len() {
            if row_starts.is_empty() {
                row_starts.push(0);
            }
            break;
        }
        let line_end = text[offset..]
            .find('\n')
            .map_or(text.len(), |index| offset + index);
        let line = text.get(offset..line_end).unwrap_or("");
        wrap_logical_line(line, line_width, wrap.trim, offset, &mut row_starts);
        if line_end >= text.len() {
            break;
        }
        offset = line_end + 1;
    }
    if row_starts.is_empty() {
        row_starts.push(0);
    }
    RowIndex { row_starts }
}

fn wrap_logical_line(
    line: &str,
    line_width: u16,
    trim: bool,
    base_offset: usize,
    row_starts: &mut Vec<usize>,
) {
    let max = usize::from(line_width);
    if line.is_empty() {
        row_starts.push(base_offset);
        return;
    }
    row_starts.push(base_offset);
    let mut col = 0usize;
    for (byte, grapheme) in grapheme_offsets(line) {
        let width = cluster_display_width(grapheme);
        if width > max {
            continue;
        }
        let is_ws = grapheme.chars().all(char::is_whitespace);
        if is_ws {
            if col == 0 && trim {
                continue;
            }
            if col > 0 && col + width > max {
                push_row_start(line, byte, trim, base_offset, row_starts);
                col = 0;
                if trim {
                    continue;
                }
            }
            col += width;
            continue;
        }
        if col > 0 && col + width > max {
            push_row_start(line, byte, trim, base_offset, row_starts);
            col = 0;
        }
        col += width;
    }
}

fn push_row_start(
    line: &str,
    byte_in_line: usize,
    trim: bool,
    base_offset: usize,
    row_starts: &mut Vec<usize>,
) {
    let mut at = byte_in_line.min(line.len());
    if trim {
        for (byte, grapheme) in grapheme_offsets(line) {
            if byte < byte_in_line {
                continue;
            }
            if grapheme.chars().all(char::is_whitespace) {
                at = byte + grapheme.len();
            } else {
                break;
            }
        }
    }
    let offset = base_offset + at;
    if row_starts.last().copied() != Some(offset) {
        row_starts.push(offset);
    }
}

fn grapheme_offsets(line: &str) -> impl Iterator<Item = (usize, &str)> {
    UnicodeSegmentation::graphemes(line, true).scan(0usize, |byte, g| {
        let start = *byte;
        *byte += g.len();
        Some((start, g))
    })
}

/// Re-wraps `text` without caching (for tests and cache validation).
#[must_use]
pub fn wrap_rows_fresh(text: &str, line_width: u16, wrap: Wrap) -> RowIndex {
    wrap_rows(text, line_width, wrap)
}
