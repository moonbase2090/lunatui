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
#[cfg(feature = "render-color")]
/// Shared perceptual color conversion and interpolation.
pub mod oklab;
#[cfg(feature = "render-color")]
/// Prepared color ramps and fixed-point compositing for frame rendering.
pub mod paint;
/// Runtime SIMD dispatch and scalar reference kernels (roadmap PR 47+).
pub mod simd;
mod size;
mod style;
#[cfg(feature = "subcell")]
/// Startup glyph selection and subcell coverage lookup.
pub mod subcell;
/// Border, corner, and line glyphs for drawing widgets.
pub mod symbols;
mod width;

pub use attr::{Attrs, UnderlineStyle};
pub use cell::{Cell, CellFlags, CellWidth, Meta};
pub use color::Color;
pub use diff::{ChangedRun, diff_runs, diff_runs_into, diff_runs_naive};
pub use grapheme::{
    CLUSTER_TAG, CONTINUATION_GLYPH, ClusterId, ClusterTable, DecodedGlyph, GlyphSlot,
    MAX_CLUSTER_IDS, clip_display_prefix, cluster_display_width, text_display_width,
};
pub use grid::{Grid, Rect, RowDirty};
pub use size::Size;
pub use style::Style;
pub use width::{UNICODE_VERSION, char_width, width};
