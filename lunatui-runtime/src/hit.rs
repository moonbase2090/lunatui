// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;

/// Stable identifier for an interactive region (PRD §2.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HitId(
    /// Application-defined id value.
    pub u32,
);

/// One interactive region recorded during a frame render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HitRegion {
    id: HitId,
    area: Rect,
    z: u16,
}

/// Flat hit map for a single frame. Later entries with the same `z` win ties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HitMap {
    regions: Vec<HitRegion>,
}

impl HitMap {
    /// Starts empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops every region.
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// Records `id` at `area`. Higher `z` wins when regions overlap.
    pub fn push(&mut self, id: HitId, area: Rect, z: u16) {
        self.regions.push(HitRegion { id, area, z });
    }

    /// Returns the topmost hit at `(column, row)`, if any.
    #[must_use]
    pub fn pick(&self, column: u16, row: u16) -> Option<HitId> {
        let mut best: Option<&HitRegion> = None;
        for region in &self.regions {
            if !contains(region.area, column, row) {
                continue;
            }
            best = Some(match best {
                None => region,
                Some(prev) if region.z >= prev.z => region,
                Some(prev) => prev,
            });
        }
        best.map(|region| region.id)
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    let right = area.x.saturating_add(area.width);
    let bottom = area.y.saturating_add(area.height);
    (area.x..right).contains(&column) && (area.y..bottom).contains(&row)
}

#[cfg(test)]
mod tests {
    use lunatui_core::Rect;

    use super::{HitId, HitMap};

    #[test]
    fn pick_returns_none_outside_regions() {
        let mut map = HitMap::new();
        map.push(HitId(1), Rect::new(1, 1, 3, 2), 0);
        assert_eq!(map.pick(0, 0), None);
    }

    #[test]
    fn pick_returns_the_highest_z_layer() {
        let mut map = HitMap::new();
        map.push(HitId(1), Rect::new(0, 0, 4, 4), 0);
        map.push(HitId(2), Rect::new(1, 1, 2, 2), 1);
        assert_eq!(map.pick(2, 2), Some(HitId(2)));
    }

    #[test]
    fn later_region_wins_on_equal_z() {
        let mut map = HitMap::new();
        map.push(HitId(1), Rect::new(0, 0, 4, 4), 0);
        map.push(HitId(2), Rect::new(0, 0, 4, 4), 0);
        assert_eq!(map.pick(1, 1), Some(HitId(2)));
    }
}
