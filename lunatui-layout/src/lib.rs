// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use lunatui_core::Rect;

mod constraint;
mod flex;
#[cfg(feature = "size-limits")]
mod limit;
mod solve_fast;
#[cfg(feature = "flex-wrap")]
mod wrap;

pub use constraint::Constraint;
pub use flex::{Axis, Flex};
#[cfg(feature = "size-limits")]
pub use limit::{Limit, aspect_fit};
#[cfg(feature = "flex-wrap")]
pub use wrap::{Wrapped, WrappedLine};

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
    #[cfg(feature = "flex-wrap")]
    line_gap: u16,
    horizontal_margin: u16,
    vertical_margin: u16,
    constraints: Vec<Constraint>,
    #[cfg(feature = "size-limits")]
    limits: Vec<Limit>,
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

    /// Cells between segments and between wrapped lines.
    ///
    /// This sets [`Layout::spacing`] and the line gap used by [`Layout::split_wrapped`] together.
    /// Values above `i16::MAX` are clamped for segment spacing.
    #[cfg(feature = "flex-wrap")]
    #[must_use]
    pub fn gap(mut self, gap: u16) -> Self {
        self.spacing = i16::try_from(gap).unwrap_or(i16::MAX);
        self.line_gap = gap;
        self
    }

    /// Inset on the left and right, and on the top and bottom, applied before the split.
    #[must_use]
    pub const fn margin(mut self, horizontal: u16, vertical: u16) -> Self {
        self.horizontal_margin = horizontal;
        self.vertical_margin = vertical;
        self
    }

    /// A floor and a cap for each segment, in constraint order. Segments past the end of `limits`
    /// have none, and extra limits are ignored. See [`Limit`].
    #[cfg(feature = "size-limits")]
    #[must_use]
    pub fn limits(mut self, limits: impl IntoIterator<Item = Limit>) -> Self {
        self.limits = limits.into_iter().collect();
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
        #[cfg(feature = "size-limits")]
        let bounds: Vec<(u16, u16)> = self
            .limits
            .iter()
            .map(|limit| (limit.min(), limit.max()))
            .collect();
        #[cfg(not(feature = "size-limits"))]
        let bounds: Vec<(u16, u16)> = Vec::new();
        let (sizes, gaps, needs_solver) = solve_fast::axis_sizes_bounded(
            len,
            &self.constraints,
            &bounds,
            self.spacing,
            self.flex,
        );
        let (segments, spacers) = solve_fast::place(inner, self.axis, &sizes, &gaps);
        Solved {
            segments,
            spacers,
            needs_solver,
        }
    }

    /// Split `area` along this layout's axis, starting a new line when the next segment does not
    /// fit.
    ///
    /// A segment fits when its natural size, plus the spacing before it, fits in what is left of
    /// the line. The natural size is the cell count of `Length`, `Min` and `Max`, the share of the
    /// full axis for `Percentage` and `Ratio`, and nothing for `Fill`, which only needs one free
    /// cell. A segment longer than the axis gets a line of its own. Each line is then split like
    /// [`Layout::split`] with this layout's flex and spacing, so `Fill` segments share what their
    /// line leaves over.
    ///
    /// Lines stack on the cross axis. `line` sizes every line, and [`Layout::gap`] sets the cells
    /// between lines. Lines that do not fit are clipped from the end, the same as segments in
    /// [`Layout::split`]. Margins apply before the split.
    #[cfg(feature = "flex-wrap")]
    #[must_use]
    pub fn split_wrapped(&self, area: Rect, line: Constraint) -> Wrapped {
        let inner = solve_fast::inset(area, self.horizontal_margin, self.vertical_margin);
        let spec = wrap::WrapSpec {
            axis: self.axis,
            flex: self.flex,
            spacing: self.spacing,
            line_gap: self.line_gap,
            constraints: &self.constraints,
        };
        wrap::split(inner, &spec, line)
    }

    fn new(axis: Axis, constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self {
            axis,
            flex: Flex::Start,
            spacing: 0,
            #[cfg(feature = "flex-wrap")]
            line_gap: 0,
            horizontal_margin: 0,
            vertical_margin: 0,
            constraints: constraints.into_iter().collect(),
            #[cfg(feature = "size-limits")]
            limits: Vec::new(),
        }
    }
}
