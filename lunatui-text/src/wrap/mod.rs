// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Word-wrapping and cached row indices (PRD §2.5).

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
    let graphemes: Vec<(usize, &str)> = UnicodeSegmentation::graphemes(line, true)
        .scan(0usize, |byte, g| {
            let start = *byte;
            *byte += g.len();
            Some((start, g))
        })
        .collect();
    if graphemes.is_empty() {
        row_starts.push(base_offset);
        return;
    }
    row_starts.push(base_offset);
    let mut col = 0usize;
    let mut index = 0usize;
    while index < graphemes.len() {
        let (word_byte, word_width, after_word) = next_word(&graphemes, index);
        let (_ws_byte, ws_width, after_ws) = next_whitespace(&graphemes, after_word);
        let add = if col == 0 && trim {
            word_width
        } else {
            ws_width + word_width
        };
        if col > 0 && col + add > max {
            let row_byte = word_byte;
            row_starts.push(base_offset + row_byte);
            col = word_width;
            index = after_word;
            continue;
        }
        if col == 0 && trim {
            col += word_width;
        } else {
            col += ws_width + word_width;
        }
        if col > max && word_width <= max {
            row_starts.push(base_offset + word_byte);
            col = word_width;
        }
        index = after_ws;
    }
}

fn next_word(graphemes: &[(usize, &str)], start: usize) -> (usize, usize, usize) {
    let mut index = start;
    let mut width = 0usize;
    while index < graphemes.len() {
        let (_byte, g) = graphemes[index];
        if g.chars().all(char::is_whitespace) {
            break;
        }
        width += cluster_display_width(g);
        index += 1;
        if index >= graphemes.len() {
            return (graphemes[start].0, width, index);
        }
    }
    let byte = graphemes.get(start).map_or(0, |(b, _)| *b);
    (byte, width, index)
}

fn next_whitespace(graphemes: &[(usize, &str)], start: usize) -> (usize, usize, usize) {
    let mut index = start;
    let mut width = 0usize;
    let byte = graphemes.get(start).map_or(0, |(b, _)| *b);
    while index < graphemes.len() {
        let (_, g) = graphemes[index];
        if !g.chars().all(char::is_whitespace) {
            break;
        }
        width += cluster_display_width(g);
        index += 1;
    }
    (byte, width, index)
}

/// Re-wraps `text` without caching (for tests and cache validation).
#[must_use]
pub fn wrap_rows_fresh(text: &str, line_width: u16, wrap: Wrap) -> RowIndex {
    wrap_rows(text, line_width, wrap)
}
