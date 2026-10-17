// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::simd::{
    ascii_display_width, ascii_display_width_for_level, ascii_display_width_levels_agree,
    ascii_display_width_scalar, differential_levels,
};
use lunatui_core::{text_display_width, width};
use proptest::prelude::*;

#[test]
fn dispatched_matches_scalar_on_empty_ascii() {
    assert_eq!(ascii_display_width_scalar(""), 0);
    assert_eq!(ascii_display_width(""), 0);
}

#[test]
fn dispatched_matches_scalar_with_controls() {
    let text = "a\tb\n c";
    assert!(text.is_ascii());
    assert_eq!(ascii_display_width_scalar(text), 4);
    assert_eq!(ascii_display_width(text), 4);
    assert_eq!(width(text), 4);
    for &level in differential_levels() {
        assert_eq!(ascii_display_width_for_level(level)(text), 4);
    }
}

#[test]
fn printable_run_width_equals_len() {
    let text = "Hello, roadmap PR 49!";
    assert_eq!(ascii_display_width_scalar(text), text.len());
    assert_eq!(text_display_width(text), text.len());
    assert!(ascii_display_width_levels_agree(text));
}

proptest! {
    #[test]
    fn all_levels_match_scalar_on_ascii(
        bytes in prop::collection::vec(0u8..=127, 0..4096),
    ) {
        let text: String = bytes.into_iter().map(char::from).collect();
        let reference = ascii_display_width_scalar(&text);
        prop_assert_eq!(ascii_display_width(&text), reference);
        prop_assert_eq!(text_display_width(&text), reference);
        for &level in differential_levels() {
            prop_assert_eq!(ascii_display_width_for_level(level)(&text), reference);
        }
    }
}
