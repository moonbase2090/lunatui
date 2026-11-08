// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;

use crate::hit::HitId;

/// Arrow-key focus direction for spatial navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    /// Focus the nearest neighbor to the left.
    Left,
    /// Focus the nearest neighbor to the right.
    Right,
    /// Focus the nearest neighbor above.
    Up,
    /// Focus the nearest neighbor below.
    Down,
}

/// Tab order and focus ring state for interactive regions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusManager {
    entries: Vec<(HitId, Rect)>,
    index: Option<usize>,
}

impl FocusManager {
    /// Starts with no focused entry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops every entry and clears focus.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index = None;
    }

    /// Appends `id` in document order for Tab traversal.
    pub fn register(&mut self, id: HitId, area: Rect) {
        self.entries.push((id, area));
    }

    /// Replaces entries and keeps focus on `id` when it is still present.
    pub fn sync(&mut self, entries: Vec<(HitId, Rect)>) {
        let previous = self.focused();
        self.entries = entries;
        self.index = previous.and_then(|id| self.entries.iter().position(|(hit, _)| *hit == id));
        if self.index.is_none() && !self.entries.is_empty() {
            self.index = Some(0);
        }
    }

    /// Currently focused id.
    #[must_use]
    pub fn focused(&self) -> Option<HitId> {
        self.index
            .and_then(|index| self.entries.get(index).map(|(id, _)| *id))
    }

    /// Moves focus to `id` when it is registered.
    pub fn focus(&mut self, id: HitId) {
        self.index = self.entries.iter().position(|(hit, _)| *hit == id);
    }

    /// Tab or Shift-Tab focus.
    pub fn tab(&mut self, reverse: bool) {
        if self.entries.is_empty() {
            self.index = None;
            return;
        }
        let len = self.entries.len();
        let next = match self.index {
            None => 0,
            Some(index) if reverse => (index + len - 1) % len,
            Some(index) => (index + 1) % len,
        };
        self.index = Some(next);
    }

    /// Moves focus to the nearest neighbor in `direction`, based on rect geometry.
    ///
    /// A candidate must start at or beyond the current rect's edge in that
    /// direction, so overlapping or containing rects never receive arrow
    /// focus. Candidates whose range overlaps the current rect on the cross
    /// axis (the same row band for Left and Right, the same column band for
    /// Up and Down) win over diagonal ones; within a group the smallest edge
    /// gap wins, then the smallest cross-axis offset. Exact ties keep the
    /// first registered entry. Focus stays put when nothing is eligible.
    pub fn arrow(&mut self, direction: FocusDirection) {
        let Some(index) = self.index else {
            if !self.entries.is_empty() {
                self.index = Some(0);
            }
            return;
        };
        let (_, current) = self.entries[index];
        let mut best: Option<usize> = None;
        let mut best_key: Option<(bool, i64, i64, i64)> = None;
        for (candidate, (_, area)) in self.entries.iter().enumerate() {
            if candidate == index {
                continue;
            }
            let Some(key) = spatial_key(direction, current, *area) else {
                continue;
            };
            if best_key.is_none_or(|held| key < held) {
                best_key = Some(key);
                best = Some(candidate);
            }
        }
        if let Some(next) = best {
            self.index = Some(next);
        }
    }
}

struct Edges {
    primary_min: i64,
    primary_max: i64,
    cross_min: i64,
    cross_max: i64,
}

fn edges(direction: FocusDirection, area: Rect) -> Edges {
    let left = i64::from(area.x);
    let right = left + i64::from(area.width);
    let top = i64::from(area.y);
    let bottom = top + i64::from(area.height);
    match direction {
        FocusDirection::Left | FocusDirection::Right => Edges {
            primary_min: left,
            primary_max: right,
            cross_min: top,
            cross_max: bottom,
        },
        FocusDirection::Up | FocusDirection::Down => Edges {
            primary_min: top,
            primary_max: bottom,
            cross_min: left,
            cross_max: right,
        },
    }
}

fn spatial_key(
    direction: FocusDirection,
    current: Rect,
    candidate: Rect,
) -> Option<(bool, i64, i64, i64)> {
    let current = edges(direction, current);
    let candidate = edges(direction, candidate);
    let primary_gap = match direction {
        FocusDirection::Right | FocusDirection::Down => candidate.primary_min - current.primary_max,
        FocusDirection::Left | FocusDirection::Up => current.primary_min - candidate.primary_max,
    };
    if primary_gap < 0 {
        return None;
    }
    let cross_gap = (candidate.cross_min - current.cross_max)
        .max(current.cross_min - candidate.cross_max)
        .max(0);
    let aligned = cross_gap == 0;
    let score = if aligned {
        primary_gap
    } else {
        primary_gap + cross_gap
    };
    let cross_center_delta = ((candidate.cross_min + candidate.cross_max)
        - (current.cross_min + current.cross_max))
        .abs();
    Some((!aligned, score, primary_gap, cross_center_delta))
}

#[cfg(test)]
mod tests {
    use lunatui_core::Rect;

    use super::{FocusDirection, FocusManager};
    use crate::hit::HitId;

    #[test]
    fn tab_cycles_forward_and_backward() {
        let mut focus = FocusManager::new();
        focus.register(HitId(1), Rect::new(0, 0, 1, 1));
        focus.register(HitId(2), Rect::new(2, 0, 1, 1));
        focus.focus(HitId(1));
        assert_eq!(focus.focused(), Some(HitId(1)));
        focus.tab(false);
        assert_eq!(focus.focused(), Some(HitId(2)));
        focus.tab(true);
        assert_eq!(focus.focused(), Some(HitId(1)));
    }

    #[test]
    fn arrow_moves_to_nearest_right_neighbor() {
        let mut focus = FocusManager::new();
        focus.register(HitId(1), Rect::new(0, 0, 2, 1));
        focus.register(HitId(2), Rect::new(5, 0, 2, 1));
        focus.focus(HitId(1));
        focus.arrow(FocusDirection::Right);
        assert_eq!(focus.focused(), Some(HitId(2)));
    }
}
