// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use alloc::vec::Vec;

/// Arrangements chosen by the size of the area, such as a sidebar on wide terminals and stacked
/// panes on narrow ones.
///
/// `base` is used below the first breakpoint. Each [`at`](Self::at) adds an arrangement that takes
/// over once the size reaches its minimum. The value is anything the app arranges with: a
/// [`Layout`](crate::Layout), a bundle of layouts, or a plain enum. Switch on the width, the
/// height, or any other cell count by passing that number to [`select`](Self::select).
///
/// # Example
///
/// ```
/// use lunatui_core::Rect;
/// use lunatui_layout::{Breakpoints, Constraint, Layout};
///
/// let arrangement = Breakpoints::new(Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]))
///     .at(60, Layout::horizontal([Constraint::Length(20), Constraint::Fill(1)]));
///
/// let narrow = Rect::new(0, 0, 40, 10);
/// let wide = Rect::new(0, 0, 80, 10);
/// let stacked = arrangement.select(narrow.width).split(narrow);
/// let side_by_side = arrangement.select(wide.width).split(wide);
/// assert_eq!(stacked.segments[1], Rect::new(0, 3, 40, 7));
/// assert_eq!(side_by_side.segments[1], Rect::new(20, 0, 60, 10));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breakpoints<T> {
    base: T,
    steps: Vec<(u16, T)>,
}

impl<T> Breakpoints<T> {
    /// The arrangement for sizes below every breakpoint.
    #[must_use]
    pub const fn new(base: T) -> Self {
        Self {
            base,
            steps: Vec::new(),
        }
    }

    /// Uses `value` from `min` cells up, until a larger breakpoint takes over. Breakpoints can be
    /// added in any order. Adding the same `min` again replaces its value. A `min` of zero
    /// replaces the base arrangement at every size.
    #[must_use]
    pub fn at(mut self, min: u16, value: T) -> Self {
        match self.steps.binary_search_by_key(&min, |step| step.0) {
            Ok(found) => self.steps[found].1 = value,
            Err(place) => self.steps.insert(place, (min, value)),
        }
        self
    }

    /// The arrangement for `size`: the one with the largest minimum at or below it, or the base.
    #[must_use]
    pub fn select(&self, size: u16) -> &T {
        match self.index(size) {
            0 => &self.base,
            index => &self.steps[index - 1].1,
        }
    }

    /// Which arrangement `size` picks: zero for the base, then one per breakpoint in ascending
    /// order of minimum. Compare it between frames to notice a switch.
    #[must_use]
    pub fn index(&self, size: u16) -> usize {
        self.steps.partition_point(|step| step.0 <= size)
    }

    /// The number of arrangements, including the base.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len() + 1
    }

    /// Always false: the base arrangement is always present.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}
