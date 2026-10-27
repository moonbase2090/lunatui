// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::vec::Vec;
use core::ops::Range;

use lunatui_core::Rect;

use crate::constraint::Constraint;
use crate::flex::{Axis, Flex};
use crate::solve_fast;

/// The result of [`Layout::split_wrapped`](crate::Layout::split_wrapped).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrapped {
    /// One rectangle per constraint, in order.
    pub segments: Vec<Rect>,
    /// One entry per line, first line first.
    pub lines: Vec<WrappedLine>,
    /// A line's split, or the split of lines across the cross axis, had to break a minimum or a
    /// positive spacing.
    pub needs_solver: bool,
}

/// One line of a [`Wrapped`] split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedLine {
    /// The band this line occupies: the full main-axis length and the line's cross-axis size.
    pub area: Rect,
    /// Indexes into [`Wrapped::segments`] for the segments on this line.
    pub segments: Range<usize>,
}

pub(crate) struct WrapSpec<'a> {
    pub axis: Axis,
    pub flex: Flex,
    pub spacing: i16,
    pub line_gap: u16,
    pub constraints: &'a [Constraint],
}

pub(crate) fn split(inner: Rect, spec: &WrapSpec<'_>, line: Constraint) -> Wrapped {
    let (main_len, cross_axis) = match spec.axis {
        Axis::Horizontal => (inner.width, Axis::Vertical),
        Axis::Vertical => (inner.height, Axis::Horizontal),
    };
    let cross_len = match cross_axis {
        Axis::Horizontal => inner.width,
        Axis::Vertical => inner.height,
    };
    let breaks = break_lines(main_len, spec.constraints, spec.spacing);
    if breaks.is_empty() {
        return Wrapped {
            segments: Vec::new(),
            lines: Vec::new(),
            needs_solver: false,
        };
    }
    let line_constraints = alloc::vec![line; breaks.len()];
    let line_gap = i16::try_from(spec.line_gap).unwrap_or(i16::MAX);
    let (line_sizes, line_gaps, mut needs_solver) =
        solve_fast::axis_sizes_bounded(cross_len, &line_constraints, &[], line_gap, Flex::Start);
    let (bands, _) = solve_fast::place(inner, cross_axis, &line_sizes, &line_gaps);
    let mut segments = Vec::with_capacity(spec.constraints.len());
    let mut lines = Vec::with_capacity(breaks.len());
    for (range, band) in breaks.into_iter().zip(bands) {
        let (sizes, gaps, line_needs_solver) = solve_fast::axis_sizes_bounded(
            main_len,
            &spec.constraints[range.clone()],
            &[],
            spec.spacing,
            spec.flex,
        );
        needs_solver = needs_solver || line_needs_solver;
        let (placed, _) = solve_fast::place(band, spec.axis, &sizes, &gaps);
        segments.extend(placed);
        lines.push(WrappedLine {
            area: band,
            segments: range,
        });
    }
    Wrapped {
        segments,
        lines,
        needs_solver,
    }
}

fn break_lines(len: u16, constraints: &[Constraint], spacing: i16) -> Vec<Range<usize>> {
    let len = i64::from(len);
    let gap = i64::from(spacing);
    let mut lines = Vec::new();
    let mut start = 0;
    let mut used = 0_i64;
    for (index, constraint) in constraints.iter().enumerate() {
        let size = natural_size(*constraint, len);
        if index == start {
            used = size;
            continue;
        }
        if used + gap + size.max(1) <= len {
            used += gap + size;
        } else {
            lines.push(start..index);
            start = index;
            used = size;
        }
    }
    if start < constraints.len() {
        lines.push(start..constraints.len());
    }
    lines
}

fn natural_size(constraint: Constraint, len: i64) -> i64 {
    match constraint {
        Constraint::Min(size) | Constraint::Max(size) | Constraint::Length(size) => i64::from(size),
        Constraint::Percentage(percent) => len * i64::from(percent) / 100,
        Constraint::Ratio(numer, denom) => {
            let size = i128::from(len) * i128::from(numer) / i128::from(denom.max(1));
            i64::try_from(size).unwrap_or(i64::MAX)
        }
        Constraint::Fill(_) => 0,
    }
}
