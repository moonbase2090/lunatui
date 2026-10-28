// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;

/// A floor and a cap on one segment's size, on top of its [`Constraint`](crate::Constraint).
///
/// Set with [`Layout::limits`](crate::Layout::limits). A segment never ends up smaller than its
/// floor unless the area is too small, in which case the floor is cut and the split reports
/// `needs_solver`, like a cut [`Constraint::Min`](crate::Constraint::Min). A segment never ends up
/// larger than its cap, and the cells it cannot take go to the other growing segments, except
/// where a flex mode must cover the area: [`Flex::Legacy`](crate::Flex::Legacy), and a lone
/// segment under [`Flex::SpaceBetween`](crate::Flex::SpaceBetween). When the floor is above the
/// cap, the floor wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limit {
    min: u16,
    max: u16,
}

impl Limit {
    /// No floor and no cap.
    pub const NONE: Self = Self {
        min: 0,
        max: u16::MAX,
    };

    /// At least `min` cells.
    #[must_use]
    pub const fn at_least(min: u16) -> Self {
        Self { min, max: u16::MAX }
    }

    /// At most `max` cells.
    #[must_use]
    pub const fn at_most(max: u16) -> Self {
        Self { min: 0, max }
    }

    /// At least `min` and at most `max` cells.
    #[must_use]
    pub const fn between(min: u16, max: u16) -> Self {
        Self { min, max }
    }

    /// The floor.
    #[must_use]
    pub const fn min(self) -> u16 {
        self.min
    }

    /// The cap.
    #[must_use]
    pub const fn max(self) -> u16 {
        self.max
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self::NONE
    }
}

/// The largest rectangle inside `area` whose width and height are in the ratio `cols : rows`,
/// centred in `area`.
///
/// Sizes round down. When the fit leaves an odd cell, it goes after the rectangle (right or below),
/// as with [`Flex::Center`](crate::Flex::Center). A zero in the ratio is treated as one. Terminal
/// cells are about twice as tall as they are wide, so a shape that looks square is `2 : 1`.
#[must_use]
pub fn aspect_fit(area: Rect, cols: u16, rows: u16) -> Rect {
    let cols = u32::from(cols.max(1));
    let rows = u32::from(rows.max(1));
    let area_width = u32::from(area.width);
    let area_height = u32::from(area.height);
    let (width, height) = if area_width * rows <= area_height * cols {
        (area_width, area_width * rows / cols)
    } else {
        (area_height * cols / rows, area_height)
    };
    let width = u16::try_from(width).unwrap_or(area.width);
    let height = u16::try_from(height).unwrap_or(area.height);
    Rect {
        x: area.x.saturating_add((area.width - width) / 2),
        y: area.y.saturating_add((area.height - height) / 2),
        width,
        height,
    }
}
