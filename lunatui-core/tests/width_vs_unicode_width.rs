// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use lunatui_core::char_width;
use unicode_width::UnicodeWidthChar;

struct Delta {
    first: u32,
    last: u32,
    ours: usize,
    theirs: Option<usize>,
}

const fn delta(first: u32, last: u32, ours: usize, theirs: Option<usize>) -> Delta {
    Delta {
        first,
        last,
        ours,
        theirs,
    }
}

const CONTROLS: [Delta; 2] = [
    delta(0x0000, 0x001F, 0, None),
    delta(0x007F, 0x009F, 0, None),
];

const SOFT_HYPHEN: [Delta; 1] = [delta(0x00AD, 0x00AD, 1, Some(0))];

const HANGUL_FILLER: [Delta; 1] = [delta(0x3164, 0x3164, 2, Some(0))];

const REGIONAL_INDICATORS: [Delta; 1] = [delta(0x1F1E6, 0x1F1FF, 2, Some(1))];

const PREPENDED_CHARACTERS: [Delta; 12] = [
    delta(0x0605, 0x0605, 1, Some(0)),
    delta(0x070F, 0x070F, 1, Some(0)),
    delta(0x0890, 0x0891, 1, Some(0)),
    delta(0x08E2, 0x08E2, 1, Some(0)),
    delta(0x0D4E, 0x0D4E, 1, Some(0)),
    delta(0x111C2, 0x111C3, 1, Some(0)),
    delta(0x113D1, 0x113D1, 1, Some(0)),
    delta(0x1193F, 0x1193F, 1, Some(0)),
    delta(0x11941, 0x11941, 1, Some(0)),
    delta(0x11A84, 0x11A89, 1, Some(0)),
    delta(0x11D46, 0x11D46, 1, Some(0)),
    delta(0x11F02, 0x11F02, 1, Some(0)),
];

const SPACING_GRAPHEME_EXTENDERS: [Delta; 47] = [
    delta(0x09BE, 0x09BE, 1, Some(0)),
    delta(0x09D7, 0x09D7, 1, Some(0)),
    delta(0x0B3E, 0x0B3E, 1, Some(0)),
    delta(0x0B57, 0x0B57, 1, Some(0)),
    delta(0x0BBE, 0x0BBE, 1, Some(0)),
    delta(0x0BD7, 0x0BD7, 1, Some(0)),
    delta(0x0CC0, 0x0CC0, 1, Some(0)),
    delta(0x0CC2, 0x0CC2, 1, Some(0)),
    delta(0x0CC7, 0x0CC8, 1, Some(0)),
    delta(0x0CCA, 0x0CCB, 1, Some(0)),
    delta(0x0CD5, 0x0CD6, 1, Some(0)),
    delta(0x0D3E, 0x0D3E, 1, Some(0)),
    delta(0x0D57, 0x0D57, 1, Some(0)),
    delta(0x0DCF, 0x0DCF, 1, Some(0)),
    delta(0x0DDF, 0x0DDF, 1, Some(0)),
    delta(0x1715, 0x1715, 1, Some(0)),
    delta(0x1734, 0x1734, 1, Some(0)),
    delta(0x1B35, 0x1B35, 1, Some(0)),
    delta(0x1B3B, 0x1B3B, 1, Some(0)),
    delta(0x1B3D, 0x1B3D, 1, Some(0)),
    delta(0x1B43, 0x1B44, 1, Some(0)),
    delta(0x1BAA, 0x1BAA, 1, Some(0)),
    delta(0x1BF2, 0x1BF3, 1, Some(0)),
    delta(0x302E, 0x302F, 2, Some(0)),
    delta(0xA953, 0xA953, 1, Some(0)),
    delta(0xA9C0, 0xA9C0, 1, Some(0)),
    delta(0xFF9E, 0xFF9F, 1, Some(0)),
    delta(0x111C0, 0x111C0, 1, Some(0)),
    delta(0x11235, 0x11235, 1, Some(0)),
    delta(0x1133E, 0x1133E, 1, Some(0)),
    delta(0x1134D, 0x1134D, 1, Some(0)),
    delta(0x11357, 0x11357, 1, Some(0)),
    delta(0x113B8, 0x113B8, 1, Some(0)),
    delta(0x113C2, 0x113C2, 1, Some(0)),
    delta(0x113C5, 0x113C5, 1, Some(0)),
    delta(0x113C7, 0x113C9, 1, Some(0)),
    delta(0x113CF, 0x113CF, 1, Some(0)),
    delta(0x114B0, 0x114B0, 1, Some(0)),
    delta(0x114BD, 0x114BD, 1, Some(0)),
    delta(0x115AF, 0x115AF, 1, Some(0)),
    delta(0x116B6, 0x116B6, 1, Some(0)),
    delta(0x11930, 0x11930, 1, Some(0)),
    delta(0x1193D, 0x1193D, 1, Some(0)),
    delta(0x11F41, 0x11F41, 1, Some(0)),
    delta(0x16FF0, 0x16FF1, 2, Some(0)),
    delta(0x1D165, 0x1D166, 1, Some(0)),
    delta(0x1D16D, 0x1D172, 1, Some(0)),
];

const UNICODE_WIDTH_SPECIAL_CASES: [Delta; 4] = [
    delta(0x17A4, 0x17A4, 1, Some(2)),
    delta(0x17D8, 0x17D8, 1, Some(3)),
    delta(0x2D7F, 0x2D7F, 0, Some(1)),
    delta(0xA8FA, 0xA8FA, 1, Some(0)),
];

const NEW_IN_UNICODE_18: [Delta; 23] = [
    delta(0x05C8, 0x05C9, 0, Some(1)),
    delta(0x0B53, 0x0B54, 0, Some(1)),
    delta(0x1ADE, 0x1ADF, 0, Some(1)),
    delta(0x1AEC, 0x1AF0, 0, Some(1)),
    delta(0x10ECB, 0x10ECF, 0, Some(1)),
    delta(0x10EF0, 0x10EF9, 0, Some(1)),
    delta(0x11DF0, 0x11DF0, 0, Some(1)),
    delta(0x18CD6, 0x18CDA, 2, Some(1)),
    delta(0x18D1F, 0x18D20, 2, Some(1)),
    delta(0x18E00, 0x19191, 2, Some(1)),
    delta(0x191A0, 0x191D2, 2, Some(1)),
    delta(0x1B123, 0x1B128, 2, Some(1)),
    delta(0x1B168, 0x1B168, 2, Some(1)),
    delta(0x1D127, 0x1D128, 0, Some(1)),
    delta(0x1D25B, 0x1D25C, 0, Some(1)),
    delta(0x1F1AE, 0x1F1AE, 2, Some(1)),
    delta(0x1F6D9, 0x1F6D9, 2, Some(1)),
    delta(0x1F7DA, 0x1F7DA, 2, Some(1)),
    delta(0x1FA8B, 0x1FA8D, 2, Some(1)),
    delta(0x1FACC, 0x1FACC, 2, Some(1)),
    delta(0x1FADD, 0x1FADD, 2, Some(1)),
    delta(0x1FAEB, 0x1FAEB, 2, Some(1)),
    delta(0x1FAF9, 0x1FAFA, 2, Some(1)),
];

fn documented() -> BTreeMap<u32, (usize, Option<usize>)> {
    [
        &CONTROLS[..],
        &SOFT_HYPHEN,
        &HANGUL_FILLER,
        &REGIONAL_INDICATORS,
        &PREPENDED_CHARACTERS,
        &SPACING_GRAPHEME_EXTENDERS,
        &UNICODE_WIDTH_SPECIAL_CASES,
        &NEW_IN_UNICODE_18,
    ]
    .into_iter()
    .flatten()
    .flat_map(|delta| {
        (delta.first..=delta.last).map(|code_point| (code_point, (delta.ours, delta.theirs)))
    })
    .collect()
}

fn observed() -> BTreeMap<u32, (usize, Option<usize>)> {
    (0..=0x10_FFFF_u32)
        .filter_map(char::from_u32)
        .map(|c| (u32::from(c), (char_width(c), c.width())))
        .filter(|(_, (ours, theirs))| Some(*ours) != *theirs)
        .collect()
}

#[test]
fn differs_from_unicode_width_only_where_documented() {
    let documented = documented();
    let observed = observed();
    let undocumented: Vec<String> = observed
        .iter()
        .filter(|(code_point, widths)| documented.get(code_point) != Some(widths))
        .take(20)
        .map(|(code_point, widths)| format!("U+{code_point:04X} {widths:?}"))
        .collect();
    let no_longer_different: Vec<String> = documented
        .keys()
        .filter(|code_point| !observed.contains_key(code_point))
        .take(20)
        .map(|code_point| format!("U+{code_point:04X}"))
        .collect();
    assert!(
        undocumented.is_empty(),
        "undocumented differences: {undocumented:?}"
    );
    assert!(
        no_longer_different.is_empty(),
        "documented differences that no longer differ: {no_longer_different:?}"
    );
}

#[test]
fn agrees_with_unicode_width_on_most_code_points() {
    let assigned_or_not = (0..=0x10_FFFF_u32).filter_map(char::from_u32).count();
    assert!(observed().len() * 100 < assigned_or_not);
}
