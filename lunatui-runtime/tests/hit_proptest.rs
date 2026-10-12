// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{HitId, HitMap};
use proptest::prelude::*;

#[derive(Clone, Copy)]
struct Entry {
    id: HitId,
    area: Rect,
    z: u16,
}

fn brute_pick(entries: &[Entry], column: u16, row: u16) -> Option<HitId> {
    let mut best: Option<(u16, usize, HitId)> = None;
    for (index, entry) in entries.iter().enumerate() {
        if column < entry.area.x
            || row < entry.area.y
            || column >= entry.area.x.saturating_add(entry.area.width)
            || row >= entry.area.y.saturating_add(entry.area.height)
        {
            continue;
        }
        let candidate = (entry.z, index, entry.id);
        best = Some(match best {
            None => candidate,
            Some(prev)
                if candidate.0 > prev.0 || (candidate.0 == prev.0 && candidate.1 > prev.1) =>
            {
                candidate
            }
            Some(prev) => prev,
        });
    }
    best.map(|(_, _, id)| id)
}

proptest! {
    #[test]
    fn topmost_hit_wins(
        regions in prop::collection::vec((1_u32..30, 0_u16..30, 0_u16..15, 1_u16..8, 1_u16..6, 0_u16..4), 1..16),
        column in 0_u16..30,
        row in 0_u16..15,
    ) {
        let entries: Vec<Entry> = regions
            .into_iter()
            .map(|(id, x, y, w, h, z)| Entry {
                id: HitId(id),
                area: Rect::new(x, y, w, h),
                z,
            })
            .collect();
        let mut map = HitMap::new();
        for entry in &entries {
            map.push(entry.id, entry.area, entry.z);
        }
        prop_assert_eq!(map.pick(column, row), brute_pick(&entries, column, row));
    }
}
