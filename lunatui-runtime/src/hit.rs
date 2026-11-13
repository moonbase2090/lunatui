// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;

mod accessibility;
mod control;
pub use control::{AccessibilityRole, AmbiguousControl, Control, ControlMetadata};

/// Stable identifier for an interactive region (PRD §2.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HitId(
    /// Application-defined id value.
    pub u32,
);

impl HitId {
    /// The item index of this id in a group registered with
    /// [`HitMap::push_indexed`] from `first` for `count` items, or `None` for any other id.
    #[must_use]
    pub fn index_in(self, first: Self, count: usize) -> Option<usize> {
        let offset = usize::try_from(self.0.checked_sub(first.0)?).ok()?;
        (offset < count).then_some(offset)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Annotation {
    Geometry,
    Role(AccessibilityRole),
    Control(ControlMetadata),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HitRegion {
    id: HitId,
    area: Rect,
    z: u16,
    annotation: Annotation,
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
        self.regions.push(HitRegion {
            id,
            area,
            z,
            annotation: Annotation::Geometry,
        });
    }

    /// Records the same geometry used for rendering and named-control queries.
    /// Disabled controls remain queryable but cannot be picked for activation.
    pub fn push_control(&mut self, id: HitId, area: Rect, z: u16, metadata: ControlMetadata) {
        self.regions.push(HitRegion {
            id,
            area,
            z,
            annotation: Annotation::Control(metadata),
        });
    }

    /// Copies visible regions from an offscreen viewport to its screen position.
    /// Clips the source geometry and preserves identity, names, roles, depth and layer.
    /// Completely clipped regions are omitted.
    pub fn append_viewport(&mut self, source: &Self, viewport: Rect, x: u16, y: u16) {
        for region in &source.regions {
            let left = region.area.x.max(viewport.x);
            let top = region.area.y.max(viewport.y);
            let right = region
                .area
                .x
                .saturating_add(region.area.width)
                .min(viewport.x.saturating_add(viewport.width));
            let bottom = region
                .area
                .y
                .saturating_add(region.area.height)
                .min(viewport.y.saturating_add(viewport.height));
            if left >= right || top >= bottom {
                continue;
            }
            let mut visible = region.clone();
            visible.area = Rect::new(
                x.saturating_add(left - viewport.x),
                y.saturating_add(top - viewport.y),
                right - left,
                bottom - top,
            );
            self.regions.push(visible);
        }
    }

    /// Named controls in registration order. Raw geometry and legacy role-only
    /// annotations are not named controls.
    pub fn controls(&self) -> impl Iterator<Item = Control<'_>> {
        self.regions
            .iter()
            .filter_map(|region| match &region.annotation {
                Annotation::Control(metadata) => {
                    Some(metadata.record(region.id, region.area, region.z))
                }
                _ => None,
            })
    }

    /// Finds a control by role, exact accessible name, and owning layer.
    /// Disabled controls are included so callers can explain their availability.
    ///
    /// # Errors
    ///
    /// Returns [`AmbiguousControl`] when multiple records match. It never
    /// silently selects the first duplicate.
    pub fn find_control(
        &self,
        role: AccessibilityRole,
        name: &str,
        layer: Option<crate::LayerId>,
    ) -> Result<Option<Control<'_>>, AmbiguousControl> {
        let mut matching = self
            .controls()
            .filter(|c| c.role == role && c.name == name && c.layer == layer);
        let first = matching.next();
        let extra = matching.count();
        if extra == 0 {
            Ok(first)
        } else {
            Err(AmbiguousControl { matches: extra + 1 })
        }
    }

    /// Records one region per `(index, area)` pair, with ids counting up from `first`: item
    /// `index` gets `HitId(first.0 + index)`. Pairs come from a widget's own geometry, such as
    /// `Tabs::tab_areas` or `List::row_areas`, so what is drawn is what a click reaches.
    pub fn push_indexed(
        &mut self,
        first: HitId,
        areas: impl IntoIterator<Item = (usize, Rect)>,
        z: u16,
    ) {
        for (index, area) in areas {
            let offset = u32::try_from(index).unwrap_or(u32::MAX);
            self.push(HitId(first.0.saturating_add(offset)), area, z);
        }
    }

    /// Returns the topmost hit at `(column, row)`, if any.
    #[must_use]
    pub fn pick(&self, column: u16, row: u16) -> Option<HitId> {
        self.pick_from(column, row, 0)
    }

    /// Returns the topmost hit at `(column, row)` among regions with `z` of at least `floor`.
    /// [`LayerStack::pick`](crate::LayerStack::pick) uses it to keep presses inside an open modal
    /// and the layers above it.
    #[must_use]
    pub fn pick_from(&self, column: u16, row: u16, floor: u16) -> Option<HitId> {
        let mut best: Option<&HitRegion> = None;
        for region in &self.regions {
            if region.z < floor || !contains(region.area, column, row) {
                continue;
            }
            best = Some(match best {
                None => region,
                Some(prev) if region.z >= prev.z => region,
                Some(prev) => prev,
            });
        }
        best.and_then(|region| match &region.annotation {
            Annotation::Control(metadata)
                if metadata.record(region.id, region.area, region.z).disabled =>
            {
                None
            }
            Annotation::Control(_) | Annotation::Geometry | Annotation::Role(_) => Some(region.id),
        })
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
