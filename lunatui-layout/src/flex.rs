// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Which axis a [`Layout`](crate::Layout) divides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Columns. Segment width changes and height stays the area height.
    Horizontal,
    /// Rows. Segment height changes and width stays the area width.
    Vertical,
}

/// Where leftover space goes after constraints take their size.
///
/// This matches ratatui 0.30's `Flex` names. [`Flex::Start`] is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Flex {
    /// First and last gaps stay empty. Leftover cells go to the last segment that can grow.
    Legacy,
    /// Pack segments at the start. Leftover cells go to the last gap.
    #[default]
    Start,
    /// Pack segments at the end. Leftover cells go to the first gap.
    End,
    /// Split leftover cells across the first and last gaps. The last gap gets the odd cell.
    Center,
    /// Split leftover cells across the gaps between segments. End gaps stay empty.
    SpaceBetween,
    /// Split leftover cells across every gap, including both ends.
    SpaceEvenly,
    /// Like [`Flex::SpaceEvenly`] for one segment. Otherwise end gaps get half of a middle gap.
    SpaceAround,
}
