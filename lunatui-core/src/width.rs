// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod tables;

use core::cmp::Ordering;

use tables::{WIDE, ZERO_WIDTH};

/// The Unicode version the width tables were generated from.
pub const UNICODE_VERSION: (u8, u8, u8) = tables::UNICODE_VERSION;

/// Returns the number of terminal columns `c` occupies: 0, 1 or 2.
///
/// Combining marks, default-ignorable code points, Hangul medial and final jamo and
/// control characters are 0. East Asian Wide and Fullwidth characters and emoji with
/// emoji presentation are 2. Everything else is 1. `docs/width-policy.md` gives the
/// full policy and how it differs from the `unicode-width` crate.
///
/// ```
/// use lunatui_core::char_width;
///
/// assert_eq!(char_width('a'), 1);
/// assert_eq!(char_width('日'), 2);
/// assert_eq!(char_width('\u{301}'), 0);
/// ```
#[must_use]
pub fn char_width(c: char) -> usize {
    let code_point = u32::from(c);
    if contains(&ZERO_WIDTH, code_point) {
        0
    } else if contains(&WIDE, code_point) {
        2
    } else {
        1
    }
}

/// Returns the number of terminal columns `text` occupies: the sum of its characters'
/// [`char_width`]s.
///
/// Multi-codepoint grapheme clusters such as ZWJ emoji sequences are measured
/// character by character here. Cluster widths arrive with the grapheme slot (PR 05).
///
/// ```
/// use lunatui_core::width;
///
/// assert_eq!(width("Hello"), 5);
/// assert_eq!(width("日本語"), 6);
/// ```
#[must_use]
pub fn width(text: &str) -> usize {
    if text.is_ascii() {
        text.bytes()
            .filter(|byte| (b' '..=b'~').contains(byte))
            .count()
    } else {
        text.chars().map(char_width).sum()
    }
}

fn contains(ranges: &[(u32, u32)], code_point: u32) -> bool {
    ranges
        .binary_search_by(|&(start, end)| {
            if end < code_point {
                Ordering::Less
            } else if start > code_point {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::{char_width, width};

    #[test]
    fn printable_ascii_is_one_column() {
        assert_eq!(char_width('a'), 1);
        assert_eq!(char_width('~'), 1);
        assert_eq!(char_width(' '), 1);
    }

    #[test]
    fn control_characters_take_no_columns() {
        assert_eq!(char_width('\n'), 0);
        assert_eq!(char_width('\u{7F}'), 0);
        assert_eq!(char_width('\u{9B}'), 0);
    }

    #[test]
    fn combining_marks_take_no_columns() {
        assert_eq!(char_width('\u{301}'), 0);
        assert_eq!(char_width('\u{20DD}'), 0);
    }

    #[test]
    fn east_asian_wide_and_fullwidth_take_two_columns() {
        assert_eq!(char_width('日'), 2);
        assert_eq!(char_width('한'), 2);
        assert_eq!(char_width('Ａ'), 2);
        assert_eq!(char_width('\u{3000}'), 2);
    }

    #[test]
    fn emoji_presentation_takes_two_columns() {
        assert_eq!(char_width('🦀'), 2);
        assert_eq!(char_width('⌚'), 2);
    }

    #[test]
    fn text_presentation_emoji_take_one_column() {
        assert_eq!(char_width('©'), 1);
        assert_eq!(char_width('❤'), 1);
    }

    #[test]
    fn default_ignorables_take_no_columns() {
        assert_eq!(char_width('\u{200D}'), 0);
        assert_eq!(char_width('\u{FE0F}'), 0);
        assert_eq!(char_width('\u{200B}'), 0);
    }

    #[test]
    fn hangul_medial_and_final_jamo_take_no_columns() {
        assert_eq!(char_width('\u{1161}'), 0);
        assert_eq!(char_width('\u{11A8}'), 0);
        assert_eq!(char_width('\u{1100}'), 2);
    }

    #[test]
    fn the_policy_exceptions_hold() {
        assert_eq!(char_width('\u{AD}'), 1);
        assert_eq!(char_width('\u{3164}'), 2);
        assert_eq!(char_width('·'), 1);
        assert_eq!(char_width('ｱ'), 1);
    }

    #[test]
    fn the_last_code_points_are_narrow() {
        assert_eq!(char_width('\u{10FFFD}'), 1);
        assert_eq!(char_width('\u{E0001}'), 0);
    }

    #[test]
    fn text_width_sums_its_characters() {
        assert_eq!(width(""), 0);
        assert_eq!(width("Hello"), 5);
        assert_eq!(width("a\tb\n"), 2);
        assert_eq!(width("日本語"), 6);
        assert_eq!(width("e\u{301}"), 1);
        assert_eq!(width("🦀 rust"), 7);
    }
}
