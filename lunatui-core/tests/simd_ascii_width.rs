// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::simd::{
    ascii_display_width, ascii_display_width_for_level, ascii_display_width_levels_agree,
    ascii_display_width_scalar, differential_levels,
};
use lunatui_core::{cluster_display_width, text_display_width, width};
use proptest::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

fn grapheme_reference(text: &str) -> usize {
    UnicodeSegmentation::graphemes(text, true)
        .map(cluster_display_width)
        .sum()
}

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

#[test]
fn ascii_lengths_around_the_simd_threshold_match_scalar() {
    for len in [0, 1, 15, 16, 31, 32, 33, 47, 48, 64, 127, 128, 255, 256] {
        let text = "A".repeat(len);
        let scalar = ascii_display_width_scalar(&text);
        assert_eq!(text_display_width(&text), scalar, "ascii {len}");
        assert_eq!(width(&text), scalar, "width {len}");
        assert_eq!(grapheme_reference(&text), scalar, "graphemes {len}");
        let mixed = format!("日{text}日");
        let mixed_reference = grapheme_reference(&mixed);
        assert_eq!(
            text_display_width(&mixed),
            mixed_reference,
            "mixed ascii run {len}"
        );
        assert_eq!(width(&mixed), mixed_reference, "mixed width {len}");
    }
}

#[test]
fn non_printable_bytes_in_a_long_run_match_scalar() {
    let mut text = String::new();
    for byte in 0u8..=127 {
        text.push(char::from(byte));
    }
    text.push_str(&"Z".repeat(40));
    let scalar = ascii_display_width_scalar(&text);
    assert_eq!(text_display_width(&text), scalar);
    assert!(ascii_display_width_levels_agree(&text));
    let mixed = format!("{text}日{text}");
    let reference = grapheme_reference(&mixed);
    assert_eq!(text_display_width(&mixed), reference);
    assert_eq!(width(&mixed), reference);
}

#[test]
fn extender_after_a_long_ascii_run_stays_one_cluster() {
    let prefix = "a".repeat(40);
    let cases = [
        format!("{prefix}e\u{301}"),
        format!("{prefix}*\u{FE0F}"),
        format!("{prefix}1\u{FE0F}\u{20E3}"),
        format!("{prefix}a\u{200D}b"),
        format!("{prefix}\r\n{prefix}"),
        format!("👩\u{200D}💻{prefix}🇺🇸{prefix}❤\u{FE0F}"),
        format!("{prefix}日한é\u{200B}{prefix}"),
    ];
    for text in cases {
        let reference = grapheme_reference(&text);
        assert_eq!(text_display_width(&text), reference, "{text:?}");
        assert_eq!(width(&text), reference, "{text:?}");
    }
    assert_eq!(text_display_width(&format!("{prefix}*\u{FE0F}")), 42);
    assert_eq!(
        text_display_width(&format!("{prefix}1\u{FE0F}\u{20E3}")),
        42
    );
    assert_eq!(text_display_width(&format!("{prefix}e\u{301}")), 41);
}

fn ascii_chunk() -> impl Strategy<Value = String> {
    prop::collection::vec(0u8..=127u8, 0..=96)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect::<String>())
}

fn special_chunk() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("日"),
        Just("한"),
        Just("é"),
        Just("e\u{301}"),
        Just("❤\u{FE0F}"),
        Just("1\u{FE0F}\u{20E3}"),
        Just("👩\u{200D}💻"),
        Just("🇺🇸"),
        Just("\u{301}"),
        Just("*\u{FE0F}"),
        Just("a\u{200D}"),
        Just("\u{200B}"),
        Just(" "),
        Just("\t"),
        Just("\r\n"),
        Just("\u{7F}"),
    ]
    .prop_map(str::to_owned)
}

fn arbitrary_chunk() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..=6).prop_map(|chars| chars.into_iter().collect())
}

fn mixed_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            4 => ascii_chunk(),
            2 => special_chunk(),
            1 => arbitrary_chunk(),
        ],
        0..=16,
    )
    .prop_map(|parts| parts.concat())
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
        prop_assert_eq!(grapheme_reference(&text), reference);
        for &level in differential_levels() {
            prop_assert_eq!(ascii_display_width_for_level(level)(&text), reference);
        }
    }

    #[test]
    fn mixed_ascii_runs_match_grapheme_width(text in mixed_text()) {
        let reference = grapheme_reference(&text);
        prop_assert_eq!(text_display_width(&text), reference);
        prop_assert_eq!(width(&text), reference);
    }

    #[test]
    fn random_text_matches_grapheme_width(chars in prop::collection::vec(any::<char>(), 0..300)) {
        let text: String = chars.into_iter().collect();
        let reference = grapheme_reference(&text);
        prop_assert_eq!(text_display_width(&text), reference);
        prop_assert_eq!(width(&text), reference);
    }
}
