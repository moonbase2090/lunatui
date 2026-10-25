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
    wrap_rows_limited(text, line_width, wrap, usize::MAX)
}

/// Like [`wrap_rows`], but may stop once more than `max_starts` row starts exist.
///
/// Rows before the last recorded start are identical to the full wrap, so a caller painting rows
/// `0..n` passes `n + 1` to get a correct end offset for row `n - 1`.
#[must_use]
pub fn wrap_rows_limited(text: &str, line_width: u16, wrap: Wrap, max_starts: usize) -> RowIndex {
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
        if row_starts.len() > max_starts {
            break;
        }
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
    let graphemes = grapheme_vec(line);
    if graphemes.is_empty() {
        row_starts.push(base_offset);
        return;
    }
    row_starts.push(base_offset);
    let mut col = 0usize;
    let mut index = 0usize;
    while index < graphemes.len() {
        let (byte, grapheme) = graphemes[index];
        if grapheme.chars().all(char::is_whitespace) {
            let width = cluster_display_width(grapheme);
            if width > max {
                index += 1;
                continue;
            }
            if col == 0 && trim {
                index += 1;
                continue;
            }
            if col > 0 && col + width > max {
                push_row_start(line, byte, trim, base_offset, row_starts);
                col = 0;
                if trim {
                    index += 1;
                    continue;
                }
            }
            col += width;
            index += 1;
            continue;
        }
        let word_byte = byte;
        let mut word_width = 0usize;
        let mut end = index;
        while end < graphemes.len() {
            let (_, g) = graphemes[end];
            if g.chars().all(char::is_whitespace) {
                break;
            }
            let w = cluster_display_width(g);
            if w > max {
                break;
            }
            word_width += w;
            end += 1;
        }
        if end == index {
            index += 1;
            continue;
        }
        if col > 0 && col + word_width > max {
            push_row_start(line, word_byte, trim, base_offset, row_starts);
            col = 0;
        }
        if word_width > max {
            hard_break_word(
                &graphemes[index..end],
                line,
                trim,
                base_offset,
                max,
                row_starts,
                &mut col,
            );
            index = end;
            continue;
        }
        col += word_width;
        index = end;
    }
}

fn hard_break_word(
    graphemes: &[(usize, &str)],
    line: &str,
    trim: bool,
    base_offset: usize,
    max: usize,
    row_starts: &mut Vec<usize>,
    col: &mut usize,
) {
    for &(byte, grapheme) in graphemes {
        let width = cluster_display_width(grapheme);
        if width > max {
            continue;
        }
        if *col > 0 && *col + width > max {
            push_row_start(line, byte, trim, base_offset, row_starts);
            *col = 0;
        }
        *col += width;
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
        // `byte_in_line` is a grapheme start, so scanning from it matches a scan from the line start.
        let tail = line.get(byte_in_line..).unwrap_or("");
        for (byte, grapheme) in grapheme_offsets(tail) {
            if grapheme.chars().all(char::is_whitespace) {
                at = byte_in_line + byte + grapheme.len();
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

/// Grapheme starts of `line`.
///
/// Two adjacent ASCII bytes never share a grapheme (UAX #29 joins only CR LF, and `line` holds no
/// newline), so only the stretches touching non-ASCII text go through the segmenter.
fn grapheme_vec(line: &str) -> Vec<(usize, &str)> {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut start = 0;
    for i in 1..=bytes.len() {
        if i < bytes.len() && !(bytes[i - 1].is_ascii() && bytes[i].is_ascii()) {
            continue;
        }
        let chunk = &line[start..i];
        if chunk.len() == 1 {
            out.push((start, chunk));
        } else {
            out.extend(grapheme_offsets(chunk).map(|(at, g)| (start + at, g)));
        }
        start = i;
    }
    out
}

/// Re-wraps `text` without caching (for tests and cache validation).
#[must_use]
pub fn wrap_rows_fresh(text: &str, line_width: u16, wrap: Wrap) -> RowIndex {
    wrap_rows(text, line_width, wrap)
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{grapheme_offsets, grapheme_vec};

    #[test]
    fn grapheme_vec_matches_segmenter() {
        let cases = [
            "",
            "plain ascii line",
            "a\u{301}b",
            "e\u{301}",
            "x \u{600}abc",
            "ab\u{200D}cd",
            "👨\u{200D}👩\u{200D}👧 family a",
            "🇺🇸🇯🇵 flags",
            "日本語 mixed ascii — dash end",
            "tab\tand\rcr",
            "a—b—c",
            "—",
        ];
        for case in cases {
            let want: Vec<(usize, &str)> = grapheme_offsets(case).collect();
            assert_eq!(grapheme_vec(case), want, "{case:?}");
        }
    }
}
