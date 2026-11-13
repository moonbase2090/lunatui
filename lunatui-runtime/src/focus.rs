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

/// Names a roving subgroup registered with [`FocusManager::set_group`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusGroup(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Group {
    key: FocusGroup,
    members: Vec<HitId>,
    last: Option<HitId>,
}

/// Tab order and focus ring state for interactive regions.
///
/// Disabled controls never take focus from Tab, Shift-Tab, arrows, [`focus`](Self::focus) or
/// [`restore`](Self::restore). When the focused control goes away or is disabled, focus moves to
/// the nearest enabled control before it in the old order, then the nearest one after it. A
/// roving subgroup is one Tab stop: arrows move inside it, and Tab leaves it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusManager {
    entries: Vec<(HitId, Rect)>,
    index: Option<usize>,
    pending: Option<HitId>,
    disabled: Vec<HitId>,
    groups: Vec<Group>,
}

impl FocusManager {
    /// Starts with no focused entry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops every entry, disabled flag and group, and clears focus.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index = None;
        self.pending = None;
        self.disabled.clear();
        self.groups.clear();
    }

    /// Appends `id` in document order for Tab traversal.
    pub fn register(&mut self, id: HitId, area: Rect) {
        self.entries.push((id, area));
    }

    /// Marks `id` disabled or enabled. A disabled control is skipped by Tab and arrows and cannot
    /// be focused; disabling the focused control moves focus to a neighbor right away. The flag
    /// stays while `id` is registered and is dropped by the first [`sync`](Self::sync) without it.
    pub fn set_disabled(&mut self, id: HitId, disabled: bool) {
        self.disabled.retain(|held| *held != id);
        if !disabled {
            return;
        }
        self.disabled.push(id);
        if self.focused() == Some(id) {
            let old = self.ids();
            let next = self.recover(&old, self.index);
            self.set_index(next);
        }
    }

    /// Whether `id` is marked disabled.
    #[must_use]
    pub fn is_disabled(&self, id: HitId) -> bool {
        self.disabled.contains(&id)
    }

    /// Registers or replaces a roving subgroup. Arrows pressed on a member move only among the
    /// members, and the group is one Tab stop: its last focused enabled member, or its first
    /// enabled member before it was ever focused.
    pub fn set_group(&mut self, group: FocusGroup, members: &[HitId]) {
        let last = self
            .groups
            .iter()
            .find(|held| held.key == group)
            .and_then(|held| held.last)
            .filter(|id| members.contains(id));
        self.groups.retain(|held| held.key != group);
        self.groups.push(Group {
            key: group,
            members: members.to_vec(),
            last,
        });
        self.set_index(self.index);
    }

    /// Removes a roving subgroup; its members become ordinary Tab stops.
    pub fn remove_group(&mut self, group: FocusGroup) {
        self.groups.retain(|held| held.key != group);
    }

    /// Replaces entries and keeps focus on `id` when it is still present and enabled. An id given
    /// to [`restore`](Self::restore) since the last sync takes focus instead when it is present
    /// and enabled. When the focused control is gone or disabled, focus moves to the nearest
    /// enabled control before it in the old order, then after it, and to the first enabled
    /// control when neither exists.
    pub fn sync(&mut self, entries: Vec<(HitId, Rect)>) {
        let pending = self
            .pending
            .take()
            .filter(|id| !self.disabled.contains(id) && entries.iter().any(|(hit, _)| hit == id));
        let old = self.ids();
        let old_index = self.index;
        let previous = pending.or_else(|| self.focused());
        self.entries = entries;
        self.disabled
            .retain(|id| self.entries.iter().any(|(hit, _)| hit == id));
        let kept = previous
            .and_then(|id| self.entries.iter().position(|(hit, _)| *hit == id))
            .filter(|position| self.enabled(*position));
        let next = kept.or_else(|| self.recover(&old, old_index));
        self.set_index(next);
    }

    /// Currently focused id.
    #[must_use]
    pub fn focused(&self) -> Option<HitId> {
        self.index
            .and_then(|index| self.entries.get(index).map(|(id, _)| *id))
    }

    /// Moves focus to `id` when it is registered and enabled. An unregistered `id` clears focus;
    /// a disabled one leaves focus where it is.
    pub fn focus(&mut self, id: HitId) {
        let position = self.entries.iter().position(|(hit, _)| *hit == id);
        if position.is_some_and(|position| !self.enabled(position)) {
            return;
        }
        self.set_index(position);
    }

    /// Gives focus back to `id`, such as the control that opened a closing layer. Focus moves now
    /// when `id` is registered and enabled, and otherwise at the next [`sync`](Self::sync) that
    /// registers it, since the screen under a modal layer only registers again once the layer is
    /// gone. A registered but disabled `id` is ignored.
    pub fn restore(&mut self, id: HitId) {
        match self.entries.iter().position(|(hit, _)| *hit == id) {
            Some(index) if self.enabled(index) => {
                self.set_index(Some(index));
                self.pending = None;
            }
            Some(_) => self.pending = None,
            None => self.pending = Some(id),
        }
    }

    /// Tab or Shift-Tab focus. Disabled controls are skipped, and a roving subgroup counts as
    /// one stop, so Tab from inside a group moves to the next control outside it.
    pub fn tab(&mut self, reverse: bool) {
        let len = self.entries.len();
        let Some(index) = self.index else {
            let first = (0..len).find(|position| self.is_stop(*position));
            self.set_index(first);
            return;
        };
        let next = (1..=len)
            .map(|step| {
                if reverse {
                    (index + len - step) % len
                } else {
                    (index + step) % len
                }
            })
            .find(|position| self.is_stop(*position));
        if next.is_some() {
            self.set_index(next);
        }
    }

    /// Moves focus to the nearest enabled neighbor in `direction`, based on rect geometry.
    ///
    /// A candidate must start at or beyond the current rect's edge in that
    /// direction, so overlapping or containing rects never receive arrow
    /// focus. Candidates whose range overlaps the current rect on the cross
    /// axis (the same row band for Left and Right, the same column band for
    /// Up and Down) win over diagonal ones; within a group the smallest edge
    /// gap wins, then the smallest cross-axis offset. Exact ties keep the
    /// first registered entry. Focus stays put when nothing is eligible.
    /// From a member of a roving subgroup only the other members are eligible.
    pub fn arrow(&mut self, direction: FocusDirection) {
        let Some(index) = self.index else {
            let first = (0..self.entries.len()).find(|position| self.enabled(*position));
            self.set_index(first);
            return;
        };
        let (current_id, current) = self.entries[index];
        let members = self
            .groups
            .iter()
            .find(|group| group.members.contains(&current_id))
            .map(|group| group.members.clone());
        let mut best: Option<usize> = None;
        let mut best_key: Option<(bool, i64, i64, i64)> = None;
        for (candidate, (id, area)) in self.entries.iter().enumerate() {
            if candidate == index
                || !self.enabled(candidate)
                || members
                    .as_ref()
                    .is_some_and(|members| !members.contains(id))
            {
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
        if best.is_some() {
            self.set_index(best);
        }
    }

    fn ids(&self) -> Vec<HitId> {
        self.entries.iter().map(|(id, _)| *id).collect()
    }

    fn enabled(&self, position: usize) -> bool {
        self.entries
            .get(position)
            .is_some_and(|(id, _)| !self.disabled.contains(id))
    }

    fn is_stop(&self, position: usize) -> bool {
        let Some((id, _)) = self.entries.get(position) else {
            return false;
        };
        if !self.enabled(position) {
            return false;
        }
        match self.groups.iter().find(|group| group.members.contains(id)) {
            None => true,
            Some(group) => self.group_stop(group) == Some(*id),
        }
    }

    fn group_stop(&self, group: &Group) -> Option<HitId> {
        let usable = |id: &HitId| {
            self.entries
                .iter()
                .position(|(hit, _)| hit == id)
                .is_some_and(|position| self.enabled(position))
        };
        group.last.filter(|id| usable(id)).or_else(|| {
            self.entries
                .iter()
                .map(|(id, _)| *id)
                .find(|id| group.members.contains(id) && usable(id))
        })
    }

    fn set_index(&mut self, index: Option<usize>) {
        self.index = index;
        let Some(id) = self.focused() else {
            return;
        };
        for group in &mut self.groups {
            if group.members.contains(&id) {
                group.last = Some(id);
            }
        }
    }

    fn recover(&self, old: &[HitId], old_index: Option<usize>) -> Option<usize> {
        let find = |id: &HitId| {
            self.entries
                .iter()
                .position(|(hit, _)| hit == id)
                .filter(|position| self.enabled(*position))
        };
        if let Some(from) = old_index.filter(|from| *from < old.len()) {
            let before = old[..from].iter().rev().find_map(find);
            if before.is_some() {
                return before;
            }
            let after = old[from + 1..].iter().find_map(find);
            if after.is_some() {
                return after;
            }
        }
        (0..self.entries.len()).find(|position| self.enabled(*position))
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
