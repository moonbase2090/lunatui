// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_text::wrap::{Wrap, wrap_rows, wrap_rows_fresh, wrap_rows_limited};

#[test]
fn fresh_matches_cached_builder() {
    let text = "hello world foo bar";
    let wrap = Wrap { trim: true };
    let a = wrap_rows(text, 8, wrap);
    let b = wrap_rows_fresh(text, 8, wrap);
    assert_eq!(a, b);
}

#[test]
fn empty_line_after_newline_has_a_row() {
    let text = "a\n\nb";
    let index = wrap_rows(text, 10, Wrap { trim: true });
    assert!(index.len() >= 3);
}

#[test]
fn overlong_word_splits_across_rows() {
    let index = wrap_rows("abcdef", 3, Wrap { trim: true });
    assert_eq!(index.len(), 2);
    assert_eq!(index.row_text("abcdef", 0), Some("abc"));
    assert_eq!(index.row_text("abcdef", 1), Some("def"));
}

#[test]
fn whole_word_moves_to_fresh_row_when_it_fits() {
    let index = wrap_rows("hello world", 8, Wrap { trim: true });
    assert_eq!(index.row_text("hello world", 1), Some("world"));
}

#[test]
fn exact_width_word_without_trim_does_not_add_empty_row() {
    let index = wrap_rows("abcd e", 4, Wrap { trim: false });
    let rows: Vec<_> = (0..index.len())
        .map(|row| index.row_text("abcd e", row).unwrap_or(""))
        .collect();
    assert!(!rows.iter().any(|row| row.is_empty()));
}

#[test]
fn limited_rows_match_full_wrap_prefix() {
    let text = "the quick brown fox\njumps over the lazy dog — again and again\n\nlast line here";
    let wrap = Wrap { trim: true };
    let full = wrap_rows(text, 7, wrap);
    for visible in 0..full.len() {
        let limited = wrap_rows_limited(text, 7, wrap, visible);
        for row in 0..visible {
            assert_eq!(limited.row_text(text, row), full.row_text(text, row));
        }
    }
}

#[test]
fn trailing_space_past_the_margin_does_not_start_an_empty_row() {
    let text = "abcd efgh \nij";
    let index = wrap_rows(text, 9, Wrap { trim: true });
    let rows: Vec<_> = (0..index.len())
        .map(|row| index.row_text(text, row).unwrap_or(""))
        .collect();
    assert_eq!(rows, ["abcd efgh \n", "ij"]);
}

#[test]
fn trailing_space_inside_the_margin_stays_on_its_row() {
    let text = "abc \nij";
    let index = wrap_rows(text, 9, Wrap { trim: true });
    assert_eq!(index.row_text(text, 0), Some("abc \n"));
    assert_eq!(index.len(), 2);
}
