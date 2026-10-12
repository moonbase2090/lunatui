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

    /// Moves focus to the nearest neighbor in `direction`.
    pub fn arrow(&mut self, direction: FocusDirection) {
        let Some(index) = self.index else {
            if !self.entries.is_empty() {
                self.index = Some(0);
            }
            return;
        };
        let (_, current) = self.entries[index];
        let (cx, cy) = center(current);
        let mut best: Option<usize> = None;
        let mut best_distance = f64::MAX;
        for (candidate, (_, area)) in self.entries.iter().enumerate() {
            if candidate == index {
                continue;
            }
            let (tx, ty) = center(*area);
            if !in_direction(direction, cx, cy, tx, ty) {
                continue;
            }
            let distance = (tx - cx).powi(2) + (ty - cy).powi(2);
            if distance < best_distance {
                best_distance = distance;
                best = Some(candidate);
            }
        }
        if let Some(next) = best {
            self.index = Some(next);
        }
    }
}

fn center(area: Rect) -> (f64, f64) {
    (
        f64::from(area.x) + f64::from(area.width) / 2.0,
        f64::from(area.y) + f64::from(area.height) / 2.0,
    )
}

fn in_direction(direction: FocusDirection, cx: f64, cy: f64, tx: f64, ty: f64) -> bool {
    match direction {
        FocusDirection::Left => tx < cx,
        FocusDirection::Right => tx > cx,
        FocusDirection::Up => ty < cy,
        FocusDirection::Down => ty > cy,
    }
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
