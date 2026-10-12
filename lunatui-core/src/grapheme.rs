// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
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
    let chars: Vec<char> = cluster.chars().collect();
    if chars.len() == 2 && regional_indicator(chars[0]) && regional_indicator(chars[1]) {
        return 2;
    }
    if cluster.contains('\u{FE0F}') {
        let widest = chars.iter().map(|&c| char_width(c)).max().unwrap_or(0);
        return 2.max(widest);
    }
    if cluster.contains('\u{200D}') {
        return chars.iter().map(|&c| char_width(c)).max().unwrap_or(0);
    }
    chars.iter().map(|&c| char_width(c)).sum()
}

/// Sums [`cluster_display_width`] over the extended grapheme clusters of `text`.
#[must_use]
pub fn text_display_width(text: &str) -> usize {
    if text.is_ascii() {
        text.bytes()
            .filter(|byte| (b' '..=b'~').contains(byte))
            .count()
    } else {
        UnicodeSegmentation::graphemes(text, true)
            .map(cluster_display_width)
            .sum()
    }
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
    use super::*;

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
}
