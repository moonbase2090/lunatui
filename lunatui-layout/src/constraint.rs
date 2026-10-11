// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// One segment's claim on the axis being divided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constraint {
    /// At least this many cells. In [`crate::Flex::Legacy`] it stays at this size unless it is the
    /// leftover sink. In every other flex it also shares leftover with [`Constraint::Fill`].
    Min(u16),
    /// At most this many cells, and this many when there is room.
    Max(u16),
    /// Exactly this many cells, until the area is too small or this segment is the leftover sink.
    Length(u16),
    /// `percent` hundredths of the full axis length, rounded by largest remainder.
    Percentage(u16),
    /// `numer / denom` of the full axis length. A zero denominator is treated as one.
    Ratio(u32, u32),
    /// Share of the cells left after the other constraints. Zero weight takes nothing while any
    /// positive weight is present, and shares equally when every fill weight is zero.
    Fill(u16),
}
