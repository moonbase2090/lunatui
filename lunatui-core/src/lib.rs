// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![no_std]

mod attr;
mod cell;
mod color;
mod diff;
mod grapheme;
mod grid;
mod size;
mod style;
mod width;

pub use attr::{Attrs, UnderlineStyle};
pub use cell::{Cell, CellFlags, CellWidth, Meta};
pub use color::Color;
pub use diff::{ChangedRun, diff_runs, diff_runs_naive};
pub use grapheme::{
    CLUSTER_TAG, CONTINUATION_GLYPH, ClusterId, ClusterTable, DecodedGlyph, GlyphSlot,
    MAX_CLUSTER_IDS, cluster_display_width, text_display_width,
};
pub use grid::{Grid, Rect, RowDirty};
pub use size::Size;
pub use style::Style;
pub use width::{UNICODE_VERSION, char_width, width};
