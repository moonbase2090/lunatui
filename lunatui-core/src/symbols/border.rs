// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::line;

/// Border symbols for block widgets (ratatui `symbols::border` parity).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Set<'a> {
    /// Top-left corner.
    pub top_left: &'a str,
    /// Top-right corner.
    pub top_right: &'a str,
    /// Bottom-left corner.
    pub bottom_left: &'a str,
    /// Bottom-right corner.
    pub bottom_right: &'a str,
    /// Left edge.
    pub vertical_left: &'a str,
    /// Right edge.
    pub vertical_right: &'a str,
    /// Top edge.
    pub horizontal_top: &'a str,
    /// Bottom edge.
    pub horizontal_bottom: &'a str,
}

impl Default for Set<'_> {
    fn default() -> Self {
        PLAIN
    }
}

const fn from_line_set(line_set: line::Set<'_>) -> Set<'_> {
    Set {
        top_left: line_set.top_left,
        top_right: line_set.top_right,
        bottom_left: line_set.bottom_left,
        bottom_right: line_set.bottom_right,
        vertical_left: line_set.vertical,
        vertical_right: line_set.vertical,
        horizontal_top: line_set.horizontal,
        horizontal_bottom: line_set.horizontal,
    }
}

/// Plain box border.
pub const PLAIN: Set = from_line_set(line::NORMAL);

/// Rounded corners.
pub const ROUNDED: Set = from_line_set(line::ROUNDED);

/// Double-line border.
pub const DOUBLE: Set = from_line_set(line::DOUBLE);

/// Thick border.
pub const THICK: Set = from_line_set(line::THICK);
