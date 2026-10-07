// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use lunatui_core::{
    CLUSTER_TAG, CONTINUATION_GLYPH, ClusterId, ClusterTable, DecodedGlyph, GlyphSlot,
    cluster_display_width, width,
};
use proptest::prelude::*;

#[test]
fn inline_scalars_round_trip() {
    for scalar in ['a', '日', '🦀', '\u{301}'] {
        assert_eq!(
            GlyphSlot::inline_scalar(scalar).map(|slot| (slot.decode(), slot.bits() & CLUSTER_TAG)),
            Some((DecodedGlyph::Inline(scalar), 0)),
        );
    }
}

#[test]
fn tagged_clusters_use_bit_31() {
    let id = ClusterId::from_index(7);
    let slot = id.and_then(GlyphSlot::cluster);
    assert_eq!(slot.map(GlyphSlot::bits), Some(CLUSTER_TAG | 7));
    if let (Some(id), Some(slot)) = (id, slot) {
        assert_eq!(slot.decode(), DecodedGlyph::Cluster(id));
    }
}

#[test]
fn continuation_glyph_matches_adr() {
    assert_eq!(CONTINUATION_GLYPH, 0x0011_0000);
    assert_eq!(
        GlyphSlot::continuation().decode(),
        DecodedGlyph::Continuation
    );
}

#[test]
fn cluster_widths_match_terminals() {
    assert_eq!(cluster_display_width("👩\u{200D}💻"), 2);
    assert_eq!(cluster_display_width("❤\u{FE0F}"), 2);
    assert_eq!(cluster_display_width("\u{1F1FA}\u{1F1F8}"), 2);
    assert_eq!(cluster_display_width("e\u{301}"), 1);
}

#[test]
fn width_uses_grapheme_clusters() {
    assert_eq!(width("👩\u{200D}💻"), 2);
    assert_eq!(width("❤\u{FE0F}"), 2);
    assert_eq!(width("\u{1F1FA}\u{1F1F8}"), 2);
}

#[test]
fn equal_clusters_keep_ids_across_frames() {
    let woman_dev = "👩\u{200D}💻";
    let mut frame0 = ClusterTable::new();
    let id0 = frame0.intern(woman_dev);
    let used = frame0.referenced().clone();
    let mut frame1 = ClusterTable::begin_frame(Some(&frame0), &used);
    let id1 = frame1.intern(woman_dev);
    assert_eq!(id0, id1);
    let heart = frame1.intern("❤\u{FE0F}");
    assert_ne!(id0, heart);
}

#[test]
fn intern_round_trips_cluster_text() {
    let mut table = ClusterTable::new();
    let id = table.intern("👩\u{200D}💻");
    assert_eq!(table.cluster_text(id), Some("👩\u{200D}💻"));
    assert_eq!(table.cluster_width(id), Some(2));
}

#[test]
fn glyph_for_inline_scalar_skips_the_cluster_table() {
    let mut table = ClusterTable::new();
    let before = table.referenced().len();
    let slot = table.glyph_for("日");
    assert_eq!(slot.decode(), DecodedGlyph::Inline('日'));
    assert_eq!(table.referenced().len(), before);
}

proptest! {
    #[test]
    fn cluster_round_trip_prop(cluster in "\\PC{1,8}") {
        let mut table = ClusterTable::new();
        let id = table.intern(&cluster);
        prop_assert_eq!(table.cluster_text(id), Some(cluster.as_str()));
        prop_assert_eq!(table.cluster_width(id), Some(cluster_display_width(&cluster)));
    }

    #[test]
    fn id_stability_prop(cluster in "\\PC{2,6}") {
        let mut frame0 = ClusterTable::new();
        let id0 = frame0.intern(&cluster);
        let used = frame0.referenced().clone();
        let mut frame1 = ClusterTable::begin_frame(Some(&frame0), &used);
        let id1 = frame1.intern(&cluster);
        prop_assert_eq!(id0, id1);
    }
}
