// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::width;

#[test]
fn latin_text_is_one_column_per_letter() {
    assert_eq!(width("Hello, world"), 12);
    assert_eq!(width("café"), 4);
    assert_eq!(width("cafe\u{301}"), 4);
}

#[test]
fn cjk_text_is_two_columns_per_character() {
    assert_eq!(width("日本語のテキスト"), 16);
    assert_eq!(width("한국어"), 6);
    assert_eq!(width("Ａｂｃ"), 6);
}

#[test]
fn halfwidth_katakana_is_one_column_per_character() {
    assert_eq!(width("ｱｲｳ"), 3);
}

#[test]
fn thai_tone_and_vowel_marks_take_no_columns() {
    assert_eq!(width("ไทย"), 3);
    assert_eq!(width("ที่"), 1);
}

#[test]
fn devanagari_virama_and_dependent_vowels_take_no_columns() {
    assert_eq!(width("नमस्ते"), 4);
}

#[test]
fn emoji_with_emoji_presentation_take_two_columns() {
    assert_eq!(width("🦀🚀"), 4);
    assert_eq!(width("ok 🦀"), 5);
}

#[test]
fn zero_width_space_takes_no_columns() {
    assert_eq!(width("a\u{200B}b"), 2);
}

#[test]
fn multi_codepoint_clusters_use_grapheme_width() {
    assert_eq!(width("👩\u{200D}💻"), 2);
    assert_eq!(width("❤\u{FE0F}"), 2);
    assert_eq!(width("\u{1F1FA}\u{1F1F8}"), 2);
}
