// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use core::fmt;

use unicode_segmentation::UnicodeSegmentation;

use crate::width::char_width;

/// Tag bit of the glyph slot: set when the glyph is a cluster ID rather than a scalar value.
pub const CLUSTER_TAG: u32 = 1 << 31;

/// Glyph slot value of the trailing cell of a wide grapheme (ADR 0001).
pub const CONTINUATION_GLYPH: u32 = 0x0011_0000;

/// Maximum cluster IDs per frame before the table re-interns (ADR 0001).
pub const MAX_CLUSTER_IDS: u32 = 1 << 16;

/// Identifier of an interned multi-codepoint grapheme cluster in a frame's table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClusterId(u32);

impl ClusterId {
    /// Builds an ID when `index` fits in the cluster table.
    #[must_use]
    pub fn from_index(index: u32) -> Option<Self> {
        if index >= MAX_CLUSTER_IDS {
            None
        } else {
            Some(Self(index))
        }
    }

    /// Returns the low 31 bits stored in a tagged glyph slot.
    #[must_use]
    pub fn index(self) -> u32 {
        self.0
    }
}

/// A 32-bit glyph slot: inline scalar, tagged [`ClusterId`], or [`CONTINUATION_GLYPH`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphSlot(u32);

impl GlyphSlot {
    /// Builds an inline scalar slot when `scalar` is a valid inline glyph.
    #[must_use]
    pub fn inline_scalar(scalar: char) -> Option<Self> {
        let value = u32::from(scalar);
        if value > 0x0010_FFFF || value == CONTINUATION_GLYPH || value & CLUSTER_TAG != 0 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Builds a tagged cluster slot for `id`.
    #[must_use]
    pub fn cluster(id: ClusterId) -> Option<Self> {
        if id.index() >= MAX_CLUSTER_IDS {
            None
        } else {
            Some(Self(CLUSTER_TAG | id.index()))
        }
    }

    /// The continuation glyph slot for the trailing cell of a wide grapheme.
    #[must_use]
    pub fn continuation() -> Self {
        Self(CONTINUATION_GLYPH)
    }

    /// The raw `u32` stored in a cell's glyph field.
    #[must_use]
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Wraps a raw glyph slot word.
    #[must_use]
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Decodes this slot.
    #[must_use]
    pub fn decode(self) -> DecodedGlyph {
        if self.0 == CONTINUATION_GLYPH {
            DecodedGlyph::Continuation
        } else if self.0 & CLUSTER_TAG != 0 {
            DecodedGlyph::Cluster(ClusterId(self.0 & !CLUSTER_TAG))
        } else if let Some(scalar) = char::from_u32(self.0) {
            DecodedGlyph::Inline(scalar)
        } else {
            DecodedGlyph::Inline('\u{FFFD}')
        }
    }
}

/// What a [`GlyphSlot`] represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedGlyph {
    /// One Unicode scalar stored inline in the glyph slot.
    Inline(char),
    /// A multi-codepoint cluster interned in the frame's [`ClusterTable`].
    Cluster(ClusterId),
    /// The trailing cell of a wide grapheme.
    Continuation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClusterRecord {
    text: String,
    width: u8,
}

/// Per-frame intern table for multi-codepoint grapheme clusters (ADR 0001 §4.3).
#[derive(Clone, Debug, Default)]
pub struct ClusterTable {
    by_id: BTreeMap<ClusterId, ClusterRecord>,
    by_text: BTreeMap<String, ClusterId>,
    next_id: u32,
    referenced: BTreeSet<ClusterId>,
    overflowed: bool,
}

impl ClusterTable {
    /// An empty table for the first frame.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the table hit the ID bound and re-interned this frame.
    #[must_use]
    pub fn overflowed(&self) -> bool {
        self.overflowed
    }

    /// Starts a new frame, keeping `previous`'s clusters that were referenced there.
    #[must_use]
    pub fn begin_frame(previous: Option<&Self>, referenced: &BTreeSet<ClusterId>) -> Self {
        let mut table = Self::new();
        if let Some(prev) = previous {
            for id in referenced {
                if let Some(record) = prev.by_id.get(id) {
                    table.by_id.insert(*id, record.clone());
                    table.by_text.insert(record.text.clone(), *id);
                }
            }
            table.next_id = prev.next_id;
        }
        table
    }

    /// Cluster IDs referenced during the current frame.
    #[must_use]
    pub fn referenced(&self) -> &BTreeSet<ClusterId> {
        &self.referenced
    }

    /// Looks up a cluster by ID.
    #[must_use]
    pub fn cluster_text(&self, id: ClusterId) -> Option<&str> {
        self.by_id.get(&id).map(|record| record.text.as_str())
    }

    /// Cached display width for a cluster ID.
    #[must_use]
    pub fn cluster_width(&self, id: ClusterId) -> Option<usize> {
        self.by_id.get(&id).map(|record| usize::from(record.width))
    }

    /// Resolves `text` to a glyph slot, interning multi-codepoint clusters.
    pub fn glyph_for(&mut self, text: &str) -> GlyphSlot {
        match classify_cluster(text) {
            ClusterKind::Empty => GlyphSlot::continuation(),
            ClusterKind::Inline(scalar) => {
                if let Some(slot) = GlyphSlot::inline_scalar(scalar) {
                    slot
                } else {
                    let id = self.intern(text);
                    GlyphSlot::cluster(id).unwrap_or(GlyphSlot::continuation())
                }
            }
            ClusterKind::Multi => {
                let id = self.intern(text);
                GlyphSlot::cluster(id).unwrap_or(GlyphSlot::continuation())
            }
        }
    }

    /// Interns `cluster` and returns its ID.
    pub fn intern(&mut self, cluster: &str) -> ClusterId {
        if let Some(&id) = self.by_text.get(cluster) {
            self.referenced.insert(id);
            return id;
        }
        if self.next_id >= MAX_CLUSTER_IDS {
            return self.reintern_all(cluster);
        }
        self.insert_fresh(cluster)
    }

    fn insert_fresh(&mut self, cluster: &str) -> ClusterId {
        let id = ClusterId(self.next_id);
        self.next_id += 1;
        let width = cluster_display_width(cluster);
        let record = ClusterRecord {
            text: cluster.to_owned(),
            width: u8::try_from(width).unwrap_or(255),
        };
        self.by_id.insert(id, record);
        self.by_text.insert(cluster.to_owned(), id);
        self.referenced.insert(id);
        id
    }

    fn reintern_all(&mut self, cluster: &str) -> ClusterId {
        let mut texts: BTreeSet<String> = self
            .by_id
            .values()
            .map(|record| record.text.clone())
            .collect();
        texts.insert(cluster.to_owned());
        self.by_id.clear();
        self.by_text.clear();
        self.referenced.clear();
        self.next_id = 0;
        self.overflowed = true;
        let mut last = ClusterId(0);
        for text in texts {
            last = self.insert_fresh(&text);
        }
        last
    }
}

enum ClusterKind {
    Empty,
    Inline(char),
    Multi,
}

fn classify_cluster(text: &str) -> ClusterKind {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (None, _) => ClusterKind::Empty,
        (Some(scalar), None) => ClusterKind::Inline(scalar),
        (Some(_), Some(_)) => ClusterKind::Multi,
    }
}

/// Display width of one extended grapheme cluster (`docs/width-policy.md`).
#[must_use]
pub fn cluster_display_width(cluster: &str) -> usize {
    let mut chars = cluster.chars();
    let Some(first) = chars.next() else {
        return 0;
    };
    let Some(second) = chars.next() else {
        return if first == '\u{FE0F}' {
            2
        } else {
            char_width(first)
        };
    };
    if chars.as_str().is_empty() && regional_indicator(first) && regional_indicator(second) {
        return 2;
    }
    let widths = cluster.chars().map(char_width);
    if cluster.contains('\u{FE0F}') {
        return 2.max(widths.max().unwrap_or(0));
    }
    if cluster.contains('\u{200D}') {
        return widths.max().unwrap_or(0);
    }
    widths.sum()
}

/// Sums [`cluster_display_width`] over the extended grapheme clusters of `text`.
///
/// A pure ASCII run longer than 32 bytes is counted with the printable-ASCII kernel
/// (bytes `0x20`..=`0x7e` are one column; every other ASCII byte is zero). That count
/// matches the cluster sum, including controls.
#[must_use]
pub fn text_display_width(text: &str) -> usize {
    if text.is_ascii() {
        return crate::simd::ascii_display_width(text);
    }
    let bytes = text.as_bytes();
    let mut width = 0usize;
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(run) = next_long_ascii_run(bytes, cursor) else {
            width += grapheme_display_width(&text[cursor..]);
            break;
        };
        let (measure_start, measure_end) = ascii_run_measure(bytes, run.start, run.end);
        if cursor < measure_start {
            width += grapheme_display_width(&text[cursor..measure_start]);
        }
        if measure_start < measure_end {
            width += crate::simd::ascii_display_width(&text[measure_start..measure_end]);
        }
        cursor = measure_end;
    }
    width
}

fn grapheme_display_width(text: &str) -> usize {
    UnicodeSegmentation::graphemes(text, true)
        .map(cluster_display_width)
        .sum()
}

struct LongAsciiRun {
    start: usize,
    end: usize,
}

fn next_long_ascii_run(bytes: &[u8], from: usize) -> Option<LongAsciiRun> {
    let mut index = from;
    while let Some(start) = next_ascii_byte(bytes, index) {
        let end = ascii_run_end(bytes, start);
        if end - start > crate::simd::LONG_ASCII_RUN {
            return Some(LongAsciiRun { start, end });
        }
        index = end;
    }
    None
}

fn next_ascii_byte(bytes: &[u8], mut index: usize) -> Option<usize> {
    let end = bytes.len();
    while index + 8 <= end {
        if ascii_word(bytes, index) & ASCII_HIGH_BIT != ASCII_HIGH_BIT {
            for offset in 0..8 {
                if bytes[index + offset].is_ascii() {
                    return Some(index + offset);
                }
            }
        }
        index += 8;
    }
    while index < end {
        if bytes[index].is_ascii() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn ascii_run_end(bytes: &[u8], mut index: usize) -> usize {
    let end = bytes.len();
    while index + 8 <= end {
        if ascii_word(bytes, index) & ASCII_HIGH_BIT != 0 {
            break;
        }
        index += 8;
    }
    while index < end && bytes[index].is_ascii() {
        index += 1;
    }
    index
}

const ASCII_HIGH_BIT: u64 = 0x8080_8080_8080_8080;

fn ascii_word(bytes: &[u8], index: usize) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&bytes[index..index + 8]);
    u64::from_le_bytes(word)
}

fn ascii_run_measure(bytes: &[u8], start: usize, end: usize) -> (usize, usize) {
    let mut measure_start = start;
    let mut measure_end = end;
    if start > 0 {
        measure_start = start + 1;
    }
    if end < bytes.len()
        && measure_end > measure_start
        && is_printable_ascii(bytes[measure_end - 1])
    {
        measure_end -= 1;
    }
    (measure_start, measure_end)
}

fn is_printable_ascii(byte: u8) -> bool {
    (b' '..=b'~').contains(&byte)
}

/// Prefix of `text` whose [`text_display_width`] is at most `max_cols`.
#[must_use]
pub fn clip_display_prefix(text: &str, max_cols: u16) -> &str {
    if max_cols == 0 {
        return "";
    }
    let max = usize::from(max_cols);
    if text_display_width(text) <= max {
        return text;
    }
    let mut cols = 0usize;
    let mut end = 0usize;
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        let w = cluster_display_width(grapheme);
        if cols + w > max {
            break;
        }
        cols += w;
        end += grapheme.len();
    }
    &text[..end]
}

fn regional_indicator(c: char) -> bool {
    matches!(c, '\u{1F1E6}'..='\u{1F1FF}')
}

impl fmt::Display for ClusterId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "cluster:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::String;

    use proptest::prelude::*;

    use super::*;

    fn reference_display_width(text: &str) -> usize {
        if text.is_ascii() {
            crate::simd::ascii_display_width_scalar(text)
        } else {
            grapheme_display_width(text)
        }
    }

    #[test]
    fn overflow_reinterns_and_still_resolves() {
        let mut table = ClusterTable::new();
        table.next_id = MAX_CLUSTER_IDS - 1;
        let first = table.intern("ab");
        let second = table.intern("cd");
        assert!(table.overflowed());
        assert_eq!(table.cluster_text(first), None);
        assert_eq!(table.cluster_text(second), Some("cd"));
        if let Some(id0) = ClusterId::from_index(0) {
            assert_eq!(table.cluster_text(id0), Some("ab"));
        }
        if let Some(id1) = ClusterId::from_index(1) {
            assert_eq!(table.cluster_text(id1), Some("cd"));
        }
    }

    #[test]
    fn ascii_scalar_matches_cluster_sum_around_the_simd_boundary() {
        for length in [0usize, 1, 31, 32, 33, 48, 64] {
            let printable = "A".repeat(length);
            assert_eq!(
                crate::simd::ascii_display_width_scalar(&printable),
                grapheme_display_width(&printable)
            );
            let mut mixed_controls = String::new();
            for index in 0..length {
                mixed_controls.push(char::from(u8::try_from(index % 128).unwrap_or(0)));
            }
            assert_eq!(
                crate::simd::ascii_display_width_scalar(&mixed_controls),
                grapheme_display_width(&mixed_controls)
            );
        }
    }

    #[test]
    fn long_ascii_runs_match_the_cluster_sum() {
        let cases = [
            "a".repeat(32),
            "a".repeat(33),
            "\n".repeat(64),
            format!("{}{}", "a".repeat(16), "\t\r\n\u{7f}".repeat(8)),
            format!("{}日", "a".repeat(40)),
            format!("{}日{}", "a".repeat(40), "b".repeat(40)),
            format!("{}🦀{}", "a".repeat(50), "b".repeat(50)),
            format!("{}e\u{301}", "a".repeat(40)),
            format!("{}1\u{FE0F}\u{20E3}", "x".repeat(40)),
            format!("\u{0605}{}", "5".repeat(40)),
            format!("\u{0605}{}1\u{FE0F}\u{20E3}", "5".repeat(39)),
            format!("ab\u{0605}{}", "5".repeat(40)),
            format!("{}👩‍💻{}", "log ".repeat(20), " tail"),
            format!("{}🇺🇸", "a".repeat(48)),
            format!("{}❤️", "a".repeat(48)),
            format!("日{}日", " ".repeat(33)),
            format!("{}a\u{200D}b{}", "c".repeat(40), "d".repeat(10)),
        ];
        for text in cases {
            assert_eq!(
                text_display_width(&text),
                reference_display_width(&text),
                "{text:?}"
            );
        }
    }

    #[test]
    fn clip_keeps_the_same_prefix_when_a_wide_cluster_follows_ascii() {
        let text = format!("{}日{}", "a".repeat(40), "b".repeat(10));
        assert_eq!(clip_display_prefix(&text, 40), "a".repeat(40));
        assert_eq!(clip_display_prefix(&text, 41), "a".repeat(40));
        assert_eq!(
            clip_display_prefix(&text, 42),
            format!("{}日", "a".repeat(40))
        );
    }

    proptest! {
        #[test]
        fn display_width_matches_the_scalar_or_cluster_reference(text in "\\PC{0,160}") {
            prop_assert_eq!(text_display_width(&text), reference_display_width(&text));
            if text.is_ascii() {
                prop_assert_eq!(
                    crate::simd::ascii_display_width_scalar(&text),
                    grapheme_display_width(&text)
                );
            }
        }

        #[test]
        fn ascii_runs_around_one_scalar_match_the_reference(
            head in prop::collection::vec(0u8..=127, 0..96),
            mark in prop::char::any(),
            tail in prop::collection::vec(0u8..=127, 0..96),
        ) {
            let mut text = String::new();
            text.extend(head.into_iter().map(char::from));
            text.push(mark);
            text.extend(tail.into_iter().map(char::from));
            prop_assert_eq!(text_display_width(&text), reference_display_width(&text));
        }
    }
}
