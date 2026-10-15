// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

const VERTICAL: &str = "│";
const HORIZONTAL: &str = "─";
const TOP_RIGHT: &str = "┐";
const TOP_LEFT: &str = "┌";
const BOTTOM_RIGHT: &str = "┘";
const BOTTOM_LEFT: &str = "└";

const ROUNDED_TOP_RIGHT: &str = "╮";
const ROUNDED_TOP_LEFT: &str = "╭";
const ROUNDED_BOTTOM_RIGHT: &str = "╯";
const ROUNDED_BOTTOM_LEFT: &str = "╰";

const DOUBLE_VERTICAL: &str = "║";
const DOUBLE_HORIZONTAL: &str = "═";
const DOUBLE_TOP_RIGHT: &str = "╗";
const DOUBLE_TOP_LEFT: &str = "╔";
const DOUBLE_BOTTOM_RIGHT: &str = "╝";
const DOUBLE_BOTTOM_LEFT: &str = "╚";

const THICK_VERTICAL: &str = "┃";
const THICK_HORIZONTAL: &str = "━";
const THICK_TOP_RIGHT: &str = "┓";
const THICK_TOP_LEFT: &str = "┏";
const THICK_BOTTOM_RIGHT: &str = "┛";
const THICK_BOTTOM_LEFT: &str = "┗";

/// A set of line symbols for borders and rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Set<'a> {
    /// Vertical rule.
    pub vertical: &'a str,
    /// Horizontal rule.
    pub horizontal: &'a str,
    /// Top-right corner.
    pub top_right: &'a str,
    /// Top-left corner.
    pub top_left: &'a str,
    /// Bottom-right corner.
    pub bottom_right: &'a str,
    /// Bottom-left corner.
    pub bottom_left: &'a str,
}

/// Plain single-width lines.
pub const NORMAL: Set = Set {
    vertical: VERTICAL,
    horizontal: HORIZONTAL,
    top_right: TOP_RIGHT,
    top_left: TOP_LEFT,
    bottom_right: BOTTOM_RIGHT,
    bottom_left: BOTTOM_LEFT,
};

/// Rounded corners.
pub const ROUNDED: Set = Set {
    top_right: ROUNDED_TOP_RIGHT,
    top_left: ROUNDED_TOP_LEFT,
    bottom_right: ROUNDED_BOTTOM_RIGHT,
    bottom_left: ROUNDED_BOTTOM_LEFT,
    ..NORMAL
};

/// Double lines.
pub const DOUBLE: Set = Set {
    vertical: DOUBLE_VERTICAL,
    horizontal: DOUBLE_HORIZONTAL,
    top_right: DOUBLE_TOP_RIGHT,
    top_left: DOUBLE_TOP_LEFT,
    bottom_right: DOUBLE_BOTTOM_RIGHT,
    bottom_left: DOUBLE_BOTTOM_LEFT,
};

/// Thick lines.
pub const THICK: Set = Set {
    vertical: THICK_VERTICAL,
    horizontal: THICK_HORIZONTAL,
    top_right: THICK_TOP_RIGHT,
    top_left: THICK_TOP_LEFT,
    bottom_right: THICK_BOTTOM_RIGHT,
    bottom_left: THICK_BOTTOM_LEFT,
};
