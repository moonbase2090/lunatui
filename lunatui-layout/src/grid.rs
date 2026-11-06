// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use alloc::{string::String, vec::Vec};
use core::{fmt, ops::Range};

use lunatui_core::Rect;

use crate::{Axis, Constraint, Flex, Layout};

#[derive(Clone, Debug, PartialEq, Eq)]
struct NamedArea {
    name: String,
    rows: Range<usize>,
    columns: Range<usize>,
}

/// A two-axis layout with shared tracks and named rectangular areas.
///
/// Row and column constraints use the same rounding and shrinking rules as [`Layout`].
/// Define areas once, then call [`Self::split`] again when the available rectangle changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridLayout {
    rows: Layout,
    columns: Layout,
    areas: Vec<NamedArea>,
    horizontal_margin: u16,
    vertical_margin: u16,
}

impl GridLayout {
    /// Defines row tracks first and column tracks second.
    ///
    /// Tracks use [`Flex::Start`], with no gaps or margins. Empty track lists are valid.
    #[must_use]
    pub fn new(
        rows: impl IntoIterator<Item = Constraint>,
        columns: impl IntoIterator<Item = Constraint>,
    ) -> Self {
        Self {
            rows: Layout::vertical(rows),
            columns: Layout::horizontal(columns),
            areas: Vec::new(),
            horizontal_margin: 0,
            vertical_margin: 0,
        }
    }

    /// Names the rectangle spanning the given zero-based, end-exclusive track ranges.
    ///
    /// Names are owned, case-sensitive strings. Whitespace and Unicode are preserved.
    /// Differently named areas may overlap; callers control widget paint order.
    ///
    /// # Errors
    /// Returns [`GridError`] for an empty or duplicate name, or an empty, reversed or
    /// out-of-bounds range. Names are checked first, then rows, then columns.
    pub fn area(
        mut self,
        name: impl Into<String>,
        rows: Range<usize>,
        columns: Range<usize>,
    ) -> Result<Self, GridError> {
        let name = name.into();
        if name.is_empty() {
            return Err(GridError::EmptyName);
        }
        if self.areas.iter().any(|area| area.name == name) {
            return Err(GridError::DuplicateName { name });
        }
        validate_span(&rows, self.rows.constraints.len(), Axis::Vertical)?;
        validate_span(&columns, self.columns.constraints.len(), Axis::Horizontal)?;
        self.areas.push(NamedArea {
            name,
            rows,
            columns,
        });
        Ok(self)
    }

    /// Cells between row tracks. Negative values overlap rows, as in [`Layout::spacing`].
    #[must_use]
    pub fn row_gap(mut self, gap: i16) -> Self {
        self.rows = self.rows.spacing(gap);
        self
    }

    /// Cells between column tracks. Negative values overlap columns.
    #[must_use]
    pub fn column_gap(mut self, gap: i16) -> Self {
        self.columns = self.columns.spacing(gap);
        self
    }

    /// Where leftover vertical space goes.
    #[must_use]
    pub fn row_flex(mut self, flex: Flex) -> Self {
        self.rows = self.rows.flex(flex);
        self
    }

    /// Where leftover horizontal space goes.
    #[must_use]
    pub fn column_flex(mut self, flex: Flex) -> Self {
        self.columns = self.columns.flex(flex);
        self
    }

    /// Insets the left/right and top/bottom edges before solving either axis.
    #[must_use]
    pub const fn margin(mut self, horizontal: u16, vertical: u16) -> Self {
        self.horizontal_margin = horizontal;
        self.vertical_margin = vertical;
        self
    }

    /// Solves both axes and returns independently owned geometry and names.
    ///
    /// Spans include internal gaps. Coordinates saturate like [`Layout`]; widths and heights
    /// are calculated before translation, preserving their size near `u16::MAX` origins.
    /// This allocates through the existing axis solver and clones the area names.
    #[must_use]
    pub fn split(&self, area: Rect) -> SolvedGrid {
        let local = crate::solve_fast::inset(
            Rect::new(0, 0, area.width, area.height),
            self.horizontal_margin,
            self.vertical_margin,
        );
        let rows = self.rows.split(local);
        let columns = self.columns.split(local);
        SolvedGrid {
            rows: rows.segments,
            columns: columns.segments,
            areas: self.areas.clone(),
            origin: (area.x, area.y),
            needs_solver: rows.needs_solver || columns.needs_solver,
        }
    }
}

/// Solved grid tracks with named, cell and span lookup.
///
/// A result owns its data and remains usable after its [`GridLayout`] is dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolvedGrid {
    rows: Vec<Rect>,
    columns: Vec<Rect>,
    areas: Vec<NamedArea>,
    origin: (u16, u16),
    /// Either axis had to break a minimum or positive spacing, as in [`crate::Solved`].
    pub needs_solver: bool,
}

impl SolvedGrid {
    /// Returns a named area's rectangle, or `None` if that exact name was not declared.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Rect> {
        let area = self.areas.iter().find(|area| area.name == name)?;
        self.span(area.rows.clone(), area.columns.clone())
    }

    /// Returns one zero-based row/column intersection, or `None` for an invalid index.
    #[must_use]
    pub fn cell(&self, row: usize, column: usize) -> Option<Rect> {
        self.span(row..row.checked_add(1)?, column..column.checked_add(1)?)
    }

    /// Returns the rectangle enclosing the selected tracks, including internal gaps.
    ///
    /// Ranges are zero-based and end-exclusive. Empty, reversed and out-of-bounds ranges
    /// return `None`. With negative gaps, all selected track intervals contribute to the
    /// enclosure, even when an interior track reaches furthest.
    #[must_use]
    pub fn span(&self, rows: Range<usize>, columns: Range<usize>) -> Option<Rect> {
        let (x, width) = interval(&self.columns, columns, Axis::Horizontal)?;
        let (y, height) = interval(&self.rows, rows, Axis::Vertical)?;
        Some(Rect::new(
            self.origin.0.saturating_add(x),
            self.origin.1.saturating_add(y),
            width,
            height,
        ))
    }

    /// Iterates named rectangles in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Rect)> {
        self.areas.iter().filter_map(|area| {
            self.span(area.rows.clone(), area.columns.clone())
                .map(|rect| (area.name.as_str(), rect))
        })
    }
}

/// An invalid named grid area.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GridError {
    /// Area names cannot be empty.
    EmptyName,
    /// The same name was already declared.
    DuplicateName {
        /// The repeated, case-sensitive name.
        name: String,
    },
    /// A track range is empty, reversed or outside its axis.
    InvalidSpan {
        /// Vertical means rows; horizontal means columns.
        axis: Axis,
        /// Inclusive starting track.
        start: usize,
        /// Exclusive ending track.
        end: usize,
        /// Number of tracks on this axis.
        tracks: usize,
    },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("grid area name is empty"),
            Self::DuplicateName { name } => {
                write!(f, "grid area name {name:?} is already declared")
            }
            Self::InvalidSpan {
                axis,
                start,
                end,
                tracks,
            } => {
                let axis = match axis {
                    Axis::Vertical => "row",
                    Axis::Horizontal => "column",
                };
                write!(f, "invalid {axis} span {start}..{end} for {tracks} tracks")
            }
        }
    }
}

impl core::error::Error for GridError {}

fn validate_span(range: &Range<usize>, tracks: usize, axis: Axis) -> Result<(), GridError> {
    if range.start >= range.end || range.end > tracks {
        return Err(GridError::InvalidSpan {
            axis,
            start: range.start,
            end: range.end,
            tracks,
        });
    }
    Ok(())
}

fn interval(tracks: &[Rect], range: Range<usize>, axis: Axis) -> Option<(u16, u16)> {
    let selected = tracks.get(range)?;
    let first = selected.first()?;
    let extent = |rect: &Rect| match axis {
        Axis::Horizontal => (u32::from(rect.x), u32::from(rect.width)),
        Axis::Vertical => (u32::from(rect.y), u32::from(rect.height)),
    };
    let (mut start, width) = extent(first);
    let mut end = start + width;
    for rect in selected.iter().skip(1) {
        let (position, size) = extent(rect);
        start = start.min(position);
        end = end.max(position + size);
    }
    Some((
        u16::try_from(start).unwrap_or(u16::MAX),
        u16::try_from(end - start).unwrap_or(u16::MAX),
    ))
}
