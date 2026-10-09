// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]
#![no_std]

mod attr;
mod cell;
mod color;
mod grapheme;
mod grid;
mod style;
mod width;

pub use attr::{Attrs, UnderlineStyle};
pub use cell::{Cell, CellFlags, CellWidth, Meta};
pub use color::Color;
pub use grapheme::{
    CLUSTER_TAG, CONTINUATION_GLYPH, ClusterId, ClusterTable, DecodedGlyph, GlyphSlot,
    MAX_CLUSTER_IDS, cluster_display_width, text_display_width,
};
pub use grid::{Grid, Rect, RowDirty};
pub use style::Style;
pub use width::{UNICODE_VERSION, char_width, width};
