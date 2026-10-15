// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_text::wrap::{Wrap, wrap_rows, wrap_rows_fresh};

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
