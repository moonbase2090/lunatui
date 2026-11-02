// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Saving and restoring a split layout: `SplitTree::to_spec` and `SplitTree::from_spec`.

use std::fmt::Write;

use lunatui_core::Rect;
use lunatui_widgets::{DividerId, PaneId, SplitAxis, SplitSpecError, SplitTree};

const D1: DividerId = DividerId(1);

/// Panes 0 | (1 over (2 | 3)) with a 700 and a 250 ratio, minimums on the middle split and
/// pane 2 zoomed.
fn busy() -> SplitTree {
    let (mut tree, a) = SplitTree::new();
    let b = tree.split(a, SplitAxis::Horizontal, 500).unwrap_or(a);
    let c = tree.split(b, SplitAxis::Vertical, 700).unwrap_or(a);
    tree.split(c, SplitAxis::Horizontal, 250);
    tree.set_min(D1, 12, 9);
    tree.zoom(Some(c));
    tree
}

const BUSY: &str = "split1;4;2;h500:0:0(0,v700:12:9(1,h250:0:0(2,3)))";

fn nested(depth: usize) -> String {
    let mut spec = format!("split1;{};-;", depth + 2);
    for id in 0..depth {
        let _ = write!(spec, "h500:0:0({id},");
    }
    spec.push_str(&depth.to_string());
    spec.push_str(&")".repeat(depth));
    spec
}

/// Like `nested`, but each split nests in its first child.
fn nested_first(depth: usize) -> String {
    let mut spec = format!("split1;{};-;", depth + 2);
    spec.push_str(&"h500:0:0(".repeat(depth));
    spec.push('0');
    for id in 1..=depth {
        let _ = write!(spec, ",{id})");
    }
    spec
}

#[test]
fn a_single_pane_has_a_short_spec() {
    let (tree, _) = SplitTree::new();
    assert_eq!(tree.to_spec(), "split1;1;-;0");
}

#[test]
fn a_split_writes_axis_ratio_minimums_and_both_panes() {
    let (mut tree, a) = SplitTree::new();
    tree.split(a, SplitAxis::Vertical, 333);
    assert_eq!(tree.to_spec(), "split1;2;-;v333:0:0(0,1)");
}

#[test]
fn a_busy_tree_writes_every_field() {
    assert_eq!(busy().to_spec(), BUSY);
}

#[test]
fn a_busy_spec_reads_back_to_the_same_tree() {
    assert_eq!(SplitTree::from_spec(BUSY), Ok(busy()));
}

#[test]
fn reading_then_writing_gives_the_same_text() {
    let tree = SplitTree::from_spec(BUSY).unwrap_or_default();
    assert_eq!(tree.to_spec(), BUSY);
}

#[test]
fn a_restored_tree_lays_out_as_written() {
    let mut tree = SplitTree::from_spec("split1;2;-;h250:0:0(0,1)").unwrap_or_default();
    let panes: Vec<(u32, Rect)> = tree
        .layout(Rect::new(0, 0, 21, 5))
        .panes
        .iter()
        .map(|pane| (pane.pane.0, pane.rect))
        .collect();
    assert_eq!(
        panes,
        [(0, Rect::new(0, 0, 5, 5)), (1, Rect::new(6, 0, 15, 5))]
    );
    assert_eq!(tree.ratio(DividerId(0)), Some(250));
    assert_eq!(tree.zoomed(), None);
    tree.zoom(Some(PaneId(1)));
    assert_eq!(tree.zoomed(), Some(PaneId(1)));
}

#[test]
fn minimums_survive_a_round_trip() {
    let mut tree = SplitTree::from_spec(BUSY).unwrap_or_default();
    tree.zoom(None);
    let layout = tree.layout(Rect::new(0, 0, 40, 20));
    let middle = layout.dividers[1];
    assert_eq!((middle.min_first, middle.min_second), (12, 9));
}

#[test]
fn the_zoomed_pane_survives_a_round_trip() {
    let tree = SplitTree::from_spec(BUSY).unwrap_or_default();
    assert_eq!(tree.zoomed(), Some(PaneId(2)));
}

#[test]
fn a_closed_pane_is_gone_from_the_spec_but_its_id_is_not_reused() {
    let mut tree = busy();
    assert!(tree.close(PaneId(3)));
    let spec = tree.to_spec();
    assert_eq!(spec, "split1;4;2;h500:0:0(0,v700:12:9(1,2))");
    let mut restored = SplitTree::from_spec(&spec).unwrap_or_default();
    assert_eq!(restored, tree);
    assert_eq!(
        restored.split(PaneId(0), SplitAxis::Vertical, 500),
        Some(PaneId(4))
    );
}

#[test]
fn closing_the_zoomed_pane_clears_the_zoom_in_the_spec() {
    let mut tree = busy();
    assert!(tree.close(PaneId(2)));
    assert_eq!(tree.to_spec(), "split1;4;-;h500:0:0(0,v700:12:9(1,3))");
}

#[test]
fn zooming_and_restoring_show_in_the_spec() {
    let mut tree = busy();
    tree.zoom(None);
    assert_eq!(
        tree.to_spec(),
        "split1;4;-;h500:0:0(0,v700:12:9(1,h250:0:0(2,3)))"
    );
    tree.toggle_zoom(PaneId(0));
    assert_eq!(
        tree.to_spec(),
        "split1;4;0;h500:0:0(0,v700:12:9(1,h250:0:0(2,3)))"
    );
}

#[test]
fn a_resized_ratio_is_kept() {
    let mut tree = busy();
    tree.set_ratio(DividerId(0), 1000);
    assert_eq!(
        tree.to_spec(),
        "split1;4;2;h1000:0:0(0,v700:12:9(1,h250:0:0(2,3)))"
    );
    assert_eq!(SplitTree::from_spec(&tree.to_spec()), Ok(tree));
}

#[test]
fn the_extreme_ratios_are_accepted() {
    let low = SplitTree::from_spec("split1;2;-;h0:0:0(0,1)").unwrap_or_default();
    assert_eq!(low.ratio(DividerId(0)), Some(0));
    let high = SplitTree::from_spec("split1;2;-;h1000:65535:65535(0,1)").unwrap_or_default();
    assert_eq!(high.ratio(DividerId(0)), Some(1000));
}

#[test]
fn a_sixty_four_deep_tree_is_accepted_and_a_sixty_five_deep_one_is_not() {
    let ok = SplitTree::from_spec(&nested(64));
    assert_eq!(ok.map(|tree| tree.len()), Ok(65));
    assert_eq!(
        SplitTree::from_spec(&nested(65)),
        Err(SplitSpecError::TooDeep)
    );
}

#[test]
fn nesting_in_the_first_child_counts_toward_the_depth_cap() {
    let ok = SplitTree::from_spec(&nested_first(64));
    assert_eq!(ok.map(|tree| tree.len()), Ok(65));
    assert_eq!(
        SplitTree::from_spec(&nested_first(65)),
        Err(SplitSpecError::TooDeep)
    );
}

#[test]
fn a_very_deep_spec_is_refused_without_overflowing_the_stack() {
    let mut spec = String::from("split1;1;-;");
    spec.push_str(&"h0:0:0(0,".repeat(500));
    spec.push('0');
    spec.push_str(&")".repeat(500));
    assert!(spec.len() < 8192);
    assert_eq!(SplitTree::from_spec(&spec), Err(SplitSpecError::TooDeep));
}

#[test]
fn the_length_cap_is_8192_bytes() {
    let mut spec = String::from("split1;1;-;0");
    spec.push_str(&"x".repeat(8192 - spec.len()));
    assert_eq!(spec.len(), 8192);
    assert_eq!(
        SplitTree::from_spec(&spec),
        Err(SplitSpecError::TrailingInput(12))
    );
    spec.push('x');
    assert_eq!(SplitTree::from_spec(&spec), Err(SplitSpecError::TooLong));
}

#[test]
fn duplicate_pane_ids_are_refused() {
    assert_eq!(
        SplitTree::from_spec("split1;3;-;h500:0:0(1,1)"),
        Err(SplitSpecError::DuplicatePane(1))
    );
    assert_eq!(
        SplitTree::from_spec("split1;5;-;h500:0:0(1,v500:0:0(2,1))"),
        Err(SplitSpecError::DuplicatePane(1))
    );
}

#[test]
fn a_ratio_above_1000_is_refused() {
    assert_eq!(
        SplitTree::from_spec("split1;3;-;h1001:0:0(0,1)"),
        Err(SplitSpecError::RatioTooLarge(12))
    );
}

#[test]
fn a_zoom_of_an_unknown_pane_is_refused() {
    assert_eq!(
        SplitTree::from_spec("split1;3;5;h500:0:0(0,1)"),
        Err(SplitSpecError::UnknownZoom(5))
    );
    assert!(SplitTree::from_spec("split1;3;1;h500:0:0(0,1)").is_ok());
}

#[test]
fn trailing_text_is_refused() {
    for tail in ["x", ")", " ", "\n", ";"] {
        let spec = format!("split1;2;-;h500:0:0(0,1){tail}");
        assert_eq!(
            SplitTree::from_spec(&spec),
            Err(SplitSpecError::TrailingInput(24)),
            "{spec:?}"
        );
    }
    assert_eq!(
        SplitTree::from_spec("split1;1;-;00"),
        Err(SplitSpecError::Unexpected(12))
    );
    assert_eq!(
        SplitTree::from_spec("split1;1;-;0 "),
        Err(SplitSpecError::TrailingInput(12))
    );
}

#[test]
fn numbers_that_overflow_are_refused() {
    assert_eq!(
        SplitTree::from_spec("split1;4294967296;-;0"),
        Err(SplitSpecError::Overflow(7))
    );
    assert_eq!(
        SplitTree::from_spec("split1;2;-;99999999999"),
        Err(SplitSpecError::Overflow(11))
    );
    assert_eq!(
        SplitTree::from_spec("split1;2;-;h70000:0:0(0,1)"),
        Err(SplitSpecError::Overflow(12))
    );
    assert_eq!(
        SplitTree::from_spec("split1;2;-;h500:65536:0(0,1)"),
        Err(SplitSpecError::Overflow(16))
    );
    assert_eq!(
        SplitTree::from_spec("split1;2;-;h500:0:65536(0,1)"),
        Err(SplitSpecError::Overflow(18))
    );
    assert_eq!(
        SplitTree::from_spec("split1;2;4294967296;0"),
        Err(SplitSpecError::Overflow(9))
    );
}

#[test]
fn a_counter_that_could_reuse_an_id_is_refused() {
    assert_eq!(
        SplitTree::from_spec("split1;1;-;h500:0:0(0,1)"),
        Err(SplitSpecError::NextTooLow)
    );
    assert_eq!(
        SplitTree::from_spec("split1;0;-;0"),
        Err(SplitSpecError::NextTooLow)
    );
    assert_eq!(
        SplitTree::from_spec("split1;4294967295;-;4294967295"),
        Err(SplitSpecError::NextTooLow)
    );
}

#[test]
fn a_counter_at_the_top_of_its_range_cannot_overflow_on_split() {
    let mut tree = SplitTree::from_spec("split1;4294967294;-;0").unwrap_or_default();
    assert_eq!(
        tree.split(PaneId(0), SplitAxis::Horizontal, 500),
        Some(PaneId(4_294_967_294))
    );
    assert_eq!(tree.split(PaneId(0), SplitAxis::Horizontal, 500), None);
    let mut full = SplitTree::from_spec("split1;4294967295;-;0").unwrap_or_default();
    assert_eq!(full.split(PaneId(0), SplitAxis::Horizontal, 500), None);
    assert_eq!(full.len(), 1);
}

#[test]
fn malformed_text_is_refused_with_its_place() {
    let cases: [(&str, SplitSpecError); 14] = [
        ("", SplitSpecError::BadHeader),
        ("split2;1;-;0", SplitSpecError::BadHeader),
        (" split1;1;-;0", SplitSpecError::BadHeader),
        ("split1;", SplitSpecError::UnexpectedEnd),
        ("split1;1;-", SplitSpecError::UnexpectedEnd),
        ("split1;1;-;", SplitSpecError::UnexpectedEnd),
        ("split1;1;-;h500:0:0(0,1", SplitSpecError::UnexpectedEnd),
        ("split1;1;-;h500:0:0(0 1)", SplitSpecError::Unexpected(21)),
        ("split1;1;-;H500:0:0(0,1)", SplitSpecError::Unexpected(11)),
        ("split1;01;-;0", SplitSpecError::Unexpected(8)),
        ("split1;-1;-;0", SplitSpecError::Unexpected(7)),
        ("split1;1;-;\u{e9}", SplitSpecError::Unexpected(11)),
        ("split1;1;-;h500:0(0,1)", SplitSpecError::Unexpected(17)),
        ("split1;1;-;h:0:0(0,1)", SplitSpecError::Unexpected(12)),
    ];
    for (spec, want) in cases {
        assert_eq!(SplitTree::from_spec(spec), Err(want), "{spec:?}");
    }
}

#[test]
fn errors_explain_themselves() {
    let error: Box<dyn std::error::Error> = Box::new(SplitSpecError::DuplicatePane(7));
    assert_eq!(error.to_string(), "pane 7 appears twice in split spec");
    assert_eq!(
        SplitSpecError::TooLong.to_string(),
        "split spec is longer than 8192 bytes"
    );
    assert_eq!(
        SplitSpecError::TooDeep.to_string(),
        "split spec nests splits more than 64 deep"
    );
    assert_eq!(
        SplitSpecError::RatioTooLarge(12).to_string(),
        "ratio in split spec at byte 12 is above 1000"
    );
    assert_eq!(
        SplitSpecError::UnknownZoom(5).to_string(),
        "split spec zooms pane 5, which is not in it"
    );
    assert_eq!(
        SplitSpecError::TrailingInput(24).to_string(),
        "split spec has extra text at byte 24"
    );
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).unwrap_or(0)
    }
}

fn random_tree(rng: &mut Rng) -> SplitTree {
    let (mut tree, _) = SplitTree::new();
    for _ in 0..rng.below(24) {
        let panes = tree.panes();
        let pane = panes[rng.below(panes.len())];
        match rng.below(6) {
            0..=2 => {
                let axis = if rng.below(2) == 0 {
                    SplitAxis::Horizontal
                } else {
                    SplitAxis::Vertical
                };
                tree.split(pane, axis, u16::try_from(rng.below(1001)).unwrap_or(0));
            }
            3 => {
                tree.close(pane);
            }
            4 => tree.toggle_zoom(pane),
            _ => {
                let divider = DividerId(rng.below(tree.len()));
                tree.set_min(
                    divider,
                    u16::try_from(rng.below(40)).unwrap_or(0),
                    u16::try_from(rng.below(40)).unwrap_or(0),
                );
                tree.set_ratio(divider, u16::try_from(rng.below(1001)).unwrap_or(0));
            }
        }
    }
    tree
}

#[test]
fn random_trees_round_trip() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..500 {
        let tree = random_tree(&mut rng);
        let spec = tree.to_spec();
        assert_eq!(SplitTree::from_spec(&spec), Ok(tree), "{spec}");
    }
}

#[test]
fn truncated_and_damaged_specs_never_panic() {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let alphabet: Vec<char> = "hv0123456789:;(),-x \u{e9}\u{1f319}".chars().collect();
    let mut seeds = vec![BUSY.to_string()];
    for _ in 0..40 {
        seeds.push(random_tree(&mut rng).to_spec());
    }
    for seed in &seeds {
        let chars: Vec<char> = seed.chars().collect();
        for cut in 0..chars.len() {
            let prefix: String = chars[..cut].iter().collect();
            if let Ok(tree) = SplitTree::from_spec(&prefix) {
                assert_eq!(tree.to_spec(), prefix);
            }
        }
        for _ in 0..200 {
            let mut damaged = chars.clone();
            let at = rng.below(damaged.len());
            match rng.below(3) {
                0 => damaged[at] = alphabet[rng.below(alphabet.len())],
                1 => damaged.insert(at, alphabet[rng.below(alphabet.len())]),
                _ => {
                    damaged.remove(at);
                }
            }
            let text: String = damaged.iter().collect();
            if let Ok(tree) = SplitTree::from_spec(&text) {
                assert_eq!(tree.to_spec(), text);
            }
        }
    }
}

#[test]
fn random_text_never_panics_and_accepted_text_is_canonical() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let alphabet: Vec<char> = "hv0123456789:;(),-x".chars().collect();
    for _ in 0..30_000 {
        let mut text = String::from("split1;");
        for _ in 0..rng.below(40) {
            text.push(alphabet[rng.below(alphabet.len())]);
        }
        if let Ok(tree) = SplitTree::from_spec(&text) {
            assert_eq!(tree.to_spec(), text);
        }
    }
}
