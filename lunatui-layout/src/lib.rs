// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use lunatui_core::Rect;

mod constraint;
mod flex;
mod solve_fast;

pub use constraint::Constraint;
pub use flex::{Axis, Flex};

/// Segments and the gaps around them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solved {
    /// One rectangle per constraint, in order.
    pub segments: Vec<Rect>,
    /// One gap before each segment, plus one after the last. Empty when there are no constraints.
    pub spacers: Vec<Rect>,
    /// The fast path had to break a minimum or a positive spacing. PR 15 replaces these with kasuari.
    pub needs_solver: bool,
}

/// A one-axis split. Memoization is intentionally absent: PR 16 adds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    axis: Axis,
    flex: Flex,
    spacing: i16,
    horizontal_margin: u16,
    vertical_margin: u16,
    constraints: Vec<Constraint>,
}

impl Layout {
    /// A horizontal split. Flex is [`Flex::Start`], spacing and margin are zero.
    #[must_use]
    pub fn horizontal(constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self::new(Axis::Horizontal, constraints)
    }

    /// A vertical split. Flex is [`Flex::Start`], spacing and margin are zero.
    #[must_use]
    pub fn vertical(constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self::new(Axis::Vertical, constraints)
    }

    /// Where leftover space goes.
    #[must_use]
    pub const fn flex(mut self, flex: Flex) -> Self {
        self.flex = flex;
        self
    }

    /// Cells between segments. Negative values overlap by that many cells.
    #[must_use]
    pub const fn spacing(mut self, spacing: i16) -> Self {
        self.spacing = spacing;
        self
    }

    /// Inset on the left and right, and on the top and bottom, applied before the split.
    #[must_use]
    pub const fn margin(mut self, horizontal: u16, vertical: u16) -> Self {
        self.horizontal_margin = horizontal;
        self.vertical_margin = vertical;
        self
    }

    /// Split `area` along this layout's axis.
    #[must_use]
    pub fn split(&self, area: Rect) -> Solved {
        let inner = solve_fast::inset(area, self.horizontal_margin, self.vertical_margin);
        let len = match self.axis {
            Axis::Horizontal => inner.width,
            Axis::Vertical => inner.height,
        };
        let (sizes, gaps, needs_solver) =
            solve_fast::axis_sizes(len, &self.constraints, self.spacing, self.flex);
        let (segments, spacers) = solve_fast::place(inner, self.axis, &sizes, &gaps);
        Solved {
            segments,
            spacers,
            needs_solver,
        }
    }

    fn new(axis: Axis, constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self {
            axis,
            flex: Flex::Start,
            spacing: 0,
            horizontal_margin: 0,
            vertical_margin: 0,
            constraints: constraints.into_iter().collect(),
        }
    }
}
